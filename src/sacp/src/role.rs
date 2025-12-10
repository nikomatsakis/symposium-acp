//! Role types for JSON-RPC connections.
//!
//! Roles determine what operations are valid on a connection and how
//! certain operations (like handling unhandled messages) behave.
//!
//! # Trait Hierarchy
//!
//! - [`JrRole`] - Base trait for all roles. Requires `Clone + Debug + Send + 'static`.
//! - [`Counterpart`] - Roles that can establish a physical connection with another role.
//!   Implies passthrough `SendsToRole` and `ReceivesFromRole`.
//! - [`DefaultCounterpart`] - Subset of `Counterpart` where `send_request()` and
//!   `on_receive()` (without explicit target) are available.
//! - [`SendsToRole`] - Roles that can send messages to a specific counterpart role,
//!   with message transformation logic.
//! - [`ReceivesFromRole`] - Roles that can receive messages from a specific sender role,
//!   with message transformation logic.
//! - [`SendsTo`] - Marker trait indicating a role can send a specific message type
//!   to a specific counterpart.

use std::{fmt::Debug, hash::Hash};

use crate::{
    Handled, JrMessage, JrMessageHandler, MessageCx, UntypedMessage,
    schema::{METHOD_SUCCESSOR_MESSAGE, SuccessorMessage},
    util::json_cast,
};

/// Trait for JSON-RPC connection roles.
///
/// The role determines what operations are valid on a connection and
/// provides role-specific behavior like handling unhandled messages.
pub trait JrRole:
    Debug + Copy + Send + Sync + 'static + PartialEq + Eq + PartialOrd + Ord + Hash + Default
{
}

/// Roles that can establish a physical connection with another role.
///
/// `Counterpart<Remote>` indicates that `Self` can establish a connection
/// where `Self` is the local endpoint and `Remote` is the remote endpoint.
///
/// This trait implies passthrough semantics for both sending and receiving:
/// messages are passed through without transformation.
///
/// # Valid Connection Topologies
///
/// Not all role combinations are valid connections. The valid topologies are:
/// - `Client ↔ Agent` (direct connection)
/// - `Conductor ↔ Proxy` (conductor managing a proxy)
/// - `Conductor ↔ Agent` (conductor's connection to final agent)
/// - `Proxy ↔ Conductor` (proxy's view of its connection to conductor)
///
/// # Physical vs Logical
///
/// `Counterpart` represents *physical* connections. For *logical* message
/// destinations (which may differ from physical), see [`SendsToRole`].
///
/// For example, `AcpProxy: Counterpart<AcpConductor>` but the proxy can
/// *logically* send to `AcpClient` or `AcpAgent` via [`SendsToRole`].
pub trait HasCounterpart<Counterpart: JrRole>:
    HasRemoteRole<Counterpart, Counterpart = Counterpart>
{
    /// Method invoked when there is no defined message handler.
    fn default_message_handler(message: MessageCx<Self, Counterpart>) -> Result<(), crate::Error> {
        let method = message.method().to_string();
        message.respond_with_error(crate::Error::method_not_found().with_data(method))
    }
}

/// Roles that can send messages to a specific counterpart role.
///
/// This trait provides the transformation logic for messages sent between roles.
/// For example, a proxy sending to an agent might wrap messages in a `Successor`
/// envelope, while sending to a client would pass messages through unchanged.
pub trait HasRemoteRole<Remote: JrRole>: JrRole {
    /// The *counterpart* is the actual role that
    /// `Self` is talking to; this is typically the
    /// same as `Remote` but, in the case of a proxy,
    /// the counterpart is `Conductor` and `Remote`
    /// can also be `Agent` or `Client`.
    type Counterpart: JrRole;

    /// The "style" of remote indicates whether the messages need to be transformed as they pass through.
    fn remote_style(other: Remote) -> RemoteRoleStyle;
}

#[non_exhaustive]
pub enum RemoteRoleStyle {
    /// Pass each message through exactly as it is.
    Counterpart,

    /// Wrap messages in a [`SuccessorMessage`] envelope.
    Successor,
}

impl RemoteRoleStyle {
    pub(crate) fn transform_outgoing_message<M: JrMessage>(
        &self,
        msg: M,
    ) -> Result<UntypedMessage, crate::Error> {
        match self {
            RemoteRoleStyle::Counterpart => msg.to_untyped_message(),
            RemoteRoleStyle::Successor => SuccessorMessage {
                message: msg,
                meta: None,
            }
            .to_untyped_message(),
        }
    }

    pub(crate) async fn handle_incoming_message<L: JrRole, R: JrRole, C: JrRole>(
        &self,
        message_cx: MessageCx<L, C>,
        handler: &mut impl JrMessageHandler<Local = L, Remote = R>,
    ) -> Result<Handled<MessageCx<L, C>>, crate::Error>
    where
        L: HasRemoteRole<R, Counterpart = C> + HasCounterpart<C>,
    {
        match self {
            RemoteRoleStyle::Counterpart => return handler.handle_message(message_cx).await,
            RemoteRoleStyle::Successor => (),
        }

        let method = message_cx.method();
        if method != METHOD_SUCCESSOR_MESSAGE {
            return Ok(Handled::No(message_cx));
        }

        let SuccessorMessage { message, meta } = json_cast(message_cx.message())?;
        let successor_message_cx = message_cx.try_map_message(|_| Ok(message))?;
        match handler.handle_message(successor_message_cx).await? {
            Handled::Yes => Ok(Handled::Yes),
            Handled::No(successor_message_cx) => {
                Ok(Handled::No(successor_message_cx.try_map_message(
                    |message| SuccessorMessage { message, meta }.to_untyped_message(),
                )?))
            }
        }
    }
}

impl<Local: JrRole, Remote: JrRole> HasRemoteRole<Remote> for Local
where
    Local: HasCounterpart<Remote>,
{
    type Counterpart = Remote;

    fn remote_style(_other: Remote) -> RemoteRoleStyle {
        RemoteRoleStyle::Counterpart
    }
}

/// Marker trait indicating a role can send a specific message type to a counterpart.
///
/// This is used for compile-time validation that a role is allowed to send
/// a particular request or notification to a specific counterpart.
///
/// # Example
///
/// ```ignore
/// impl SendsTo<AcpAgent, InitializeRequest> for AcpClient {}
/// impl SendsTo<AcpAgent, ToolsCallRequest> for AcpClient {}
/// ```
pub trait SendsTo<Remote: JrRole, M>: HasRemoteRole<Remote> {}

/// A role that opts out of type-safe role checking.
///
/// `UntypedRole` can send any message to any other `UntypedRole`, making it
/// suitable for generic code, tests, and situations where role-specific
/// behavior is not needed.
///
/// For type-safe ACP communication, use the specific role types:
/// - [`AcpClient`](crate::schema::AcpClient) for clients/editors
/// - [`AcpAgent`](crate::schema::AcpAgent) for agents
/// - [`AcpProxy`](crate::schema::AcpProxy) for proxies
/// - [`AcpConductor`](crate::schema::AcpConductor) for conductors
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UntypedRole;

impl JrRole for UntypedRole {}

impl<M> SendsTo<UntypedRole, M> for UntypedRole {}

impl HasCounterpart<UntypedRole> for UntypedRole {}
