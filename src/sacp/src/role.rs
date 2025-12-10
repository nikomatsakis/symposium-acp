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

use std::{convert::Infallible, fmt::Debug, hash::Hash};

use crate::{
    MessageAndCx, UntypedMessage,
    schema::{ConductorRole, ProxyRole},
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
    fn default_message_handler(
        message: MessageAndCx<Self, Counterpart>,
    ) -> Result<(), crate::Error> {
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

    /// Transform a request before sending to the counterpart.
    fn transform_outgoing_request_for(
        remote: Remote,
        message: UntypedMessage,
    ) -> Result<UntypedMessage, crate::Error>;

    /// Transform a notification before sending to the counterpart.
    fn transform_outgoing_notification_for(
        remote: Remote,
        message: UntypedMessage,
    ) -> Result<UntypedMessage, crate::Error>;

    /// If true, all incoming messages to `Self` target `Remote`.
    /// This is true for a "counterpart" relationship (e.g., agent<->client, conductor<->proxy)
    /// but false for proxy<->client and proxy<->agent.
    const PASSTHROUGH_INCOMING: bool;

    /// Extract value created after matching a message.
    /// This is returned from [`Self::incoming_counterpart_to_remote`].
    /// and then passed to [`Self::incoming_remote_to_counterpart`].
    type Adjunct;

    /// Try to match a message.
    ///
    /// Returns `None` if (a) the message does come from `Remote` or (b) PASSTHROUGH_INCOMING is true.
    ///
    /// Returns `Some(Ok(msg))` otherwise (or error if there's a parse error of some kind).
    fn incoming_counterpart_to_remote(
        _remote: Remote,
        counterpart_message: &UntypedMessage,
    ) -> Result<Option<(UntypedMessage, Self::Adjunct)>, crate::Error>;

    /// Given a (potentially) transformed message from this remote,
    /// recreate the original message from the counterpart with the new content.
    /// Used when [`Self::extract_incoming_message_from`] has returned `Some` but the handler
    /// returned [`Handled::No`].
    fn incoming_remote_to_counterpart(
        _remote: Remote,
        remote_message: UntypedMessage,
        adjunct: Self::Adjunct,
    ) -> Result<UntypedMessage, crate::Error>;
}

impl<Local: JrRole, Remote: JrRole> HasRemoteRole<Remote> for Local
where
    Local: HasCounterpart<Remote>,
{
    type Counterpart = Remote;

    fn transform_outgoing_request_for(
        _remote: Remote,
        message: UntypedMessage,
    ) -> Result<UntypedMessage, crate::Error> {
        Ok(message)
    }

    fn transform_outgoing_notification_for(
        _remote: Remote,
        message: UntypedMessage,
    ) -> Result<UntypedMessage, crate::Error> {
        Ok(message)
    }

    const PASSTHROUGH_INCOMING: bool = true;

    type Adjunct = Infallible;

    fn incoming_counterpart_to_remote(
        _remote: Remote,
        _message: &UntypedMessage,
    ) -> Result<Option<(UntypedMessage, Self::Adjunct)>, crate::Error> {
        Err(crate::util::internal_error("passthrough"))
    }

    fn incoming_remote_to_counterpart(
        _remote: Remote,
        _original_message: UntypedMessage,
        _adjunct: Self::Adjunct,
    ) -> Result<UntypedMessage, crate::Error> {
        Err(crate::util::internal_error("passthrough"))
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
