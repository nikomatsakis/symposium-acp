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

use std::fmt::Debug;

use crate::{Handled, JrMessageHandler, MessageAndCx, UntypedMessage};

/// Trait for JSON-RPC connection roles.
///
/// The role determines what operations are valid on a connection and
/// provides role-specific behavior like handling unhandled messages.
pub trait JrRole: Debug + Clone + Send + 'static {}

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
pub trait Counterpart<Remote: JrRole>: SendsToRole<Remote> + ReceivesFromRole<Remote> {}

/// Roles where `send_request()` and `on_receive()` (without explicit target) are available.
///
/// This is a subset of [`Counterpart`] for connections where the target is unambiguous.
/// For example, `Client ↔ Agent` connections have a clear default target.
///
/// `Proxy ↔ Conductor` does NOT implement this because the proxy can logically
/// send to either `Client` or `Agent`, so an explicit target is required.
pub trait DefaultCounterpart<Remote: JrRole>: Counterpart<Remote> {}

/// Roles that can send messages to a specific counterpart role.
///
/// This trait provides the transformation logic for messages sent between roles.
/// For example, a proxy sending to an agent might wrap messages in a `Successor`
/// envelope, while sending to a client would pass messages through unchanged.
pub trait SendsToRole<R: JrRole>: JrRole {
    /// Transform a request before sending to the counterpart.
    fn transform_request(
        &self,
        message: UntypedMessage,
        target: &R,
    ) -> Result<UntypedMessage, crate::Error>;

    /// Transform a notification before sending to the counterpart.
    fn transform_notification(
        &self,
        message: UntypedMessage,
        target: &R,
    ) -> Result<UntypedMessage, crate::Error>;
}

/// Roles that can receive messages from a specific sender role.
///
/// This trait provides the logic for handling messages received from other roles,
/// including any necessary unwrapping (e.g., `SuccessorRequest` envelopes).
///
/// For example, a proxy receiving from an agent might need to unwrap messages from
/// a `Successor` envelope before dispatching to the handler, and rewrap unhandled
/// messages.
///
/// Most implementations are passthrough - they simply delegate to the handler.
/// The `AcpProxy: ReceivesFromRole<AcpAgent>` impl does the successor unwrap/rewrap.
///
/// The `Remote` parameter is the physical connection counterpart, which may differ
/// from `TxRole` (the logical sender). For example, a proxy physically connects to
/// a conductor but logically receives from clients/agents.
#[allow(async_fn_in_trait)]
pub trait ReceivesFromRole<TxRole: JrRole, Remote: JrRole = TxRole>: JrRole {
    /// Handle an incoming message from `TxRole`, dispatching to the given handler.
    ///
    /// Implementations may transform the message before/after dispatching.
    /// For example, unwrapping a `SuccessorRequest` envelope and rewrapping
    /// if the handler returns `Handled::No`.
    ///
    /// The `sender` parameter provides the sender role instance, which may
    /// carry runtime information needed for message transformation.
    async fn receive_message(
        &self,
        sender: &TxRole,
        message: MessageAndCx<Self, Remote>,
        handler: &mut impl JrMessageHandler<Self, Remote>,
    ) -> Result<Handled<MessageAndCx<Self, Remote>>, crate::Error>;
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
pub trait SendsTo<R: JrRole, M>: SendsToRole<R> {}

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
#[derive(Debug, Default, Clone)]
pub struct UntypedRole;

impl JrRole for UntypedRole {}

impl SendsToRole<UntypedRole> for UntypedRole {
    fn transform_request(
        &self,
        message: UntypedMessage,
        _target: &UntypedRole,
    ) -> Result<UntypedMessage, crate::Error> {
        Ok(message)
    }

    fn transform_notification(
        &self,
        message: UntypedMessage,
        _target: &UntypedRole,
    ) -> Result<UntypedMessage, crate::Error> {
        Ok(message)
    }
}

impl<M> SendsTo<UntypedRole, M> for UntypedRole {}

impl ReceivesFromRole<UntypedRole, UntypedRole> for UntypedRole {
    async fn receive_message(
        &self,
        _sender: &UntypedRole,
        message: MessageAndCx<Self, UntypedRole>,
        handler: &mut impl JrMessageHandler<Self, UntypedRole>,
    ) -> Result<Handled<MessageAndCx<Self, UntypedRole>>, crate::Error> {
        handler.handle_message(message).await
    }
}

impl Counterpart<UntypedRole> for UntypedRole {}
impl DefaultCounterpart<UntypedRole> for UntypedRole {}
