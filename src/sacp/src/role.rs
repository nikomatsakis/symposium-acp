//! Role types for JSON-RPC connections.
//!
//! Roles determine what operations are valid on a connection and how
//! certain operations (like handling unhandled messages) behave.
//!
//! # Trait Hierarchy
//!
//! - [`JrRole`] - Base trait for all roles. Requires `Clone + Debug + Send + 'static`.
//! - [`DefaultCounterpart`] - Roles that have a default counterpart they communicate with.
//! - [`SendsToRole`] - Roles that can send messages to a specific counterpart role,
//!   with message transformation logic.
//! - [`SendsTo`] - Marker trait indicating a role can send a specific message type
//!   to a specific counterpart.

use std::fmt::Debug;

use crate::UntypedMessage;

/// Trait for JSON-RPC connection roles.
///
/// The role determines what operations are valid on a connection and
/// provides role-specific behavior like handling unhandled messages.
pub trait JrRole: Debug + Clone + Send + 'static {}

/// Roles that have a default counterpart they communicate with.
///
/// For example, an `AcpClient` has `AcpAgent` as its default counterpart,
/// while an `AcpAgent` has `AcpClient` as its default counterpart.
///
/// This enables the convenience `send_request` method that doesn't require
/// explicitly specifying the target role.
pub trait DefaultCounterpart: JrRole {
    /// The default counterpart role for this role.
    type Counterpart: JrRole + Default;
}

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

/// Roles that can receive messages from a specific counterpart role.
///
/// This trait provides the transformation logic for messages received from other roles.
/// For example, a proxy receiving from an agent might need to unwrap messages from
/// a `Successor` envelope.
pub trait ReceivesFromRole<R: JrRole>: JrRole {
    // TODO: Define methods for transforming received messages
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

/// A role that returns an error for unhandled messages.
///
/// This is the default role used when no specific role is provided.
/// It has full client and server capabilities.
#[derive(Debug, Default, Clone)]
pub struct DefaultRole;

impl JrRole for DefaultRole {}
