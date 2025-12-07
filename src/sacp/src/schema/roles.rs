//! ACP role types for type-safe protocol communication.
//!
//! These roles represent the different participants in ACP:
//! - [`AcpClient`] - The client/editor that initiates communication with agents
//! - [`AcpAgent`] - The agent that responds to client requests
//! - [`AcpProxy`] - A proxy that sits between client and agent, potentially transforming messages

use crate::{
    JrMessage,
    role::{DefaultCounterpart, JrRole, SendsToRole},
    schema::{SuccessorNotification, SuccessorRequest},
};

/// The ACP client role (e.g., an IDE or editor).
///
/// Clients initiate communication with agents, sending requests like
/// `initialize`, `prompt`, and `tools/call`.
///
/// The default counterpart for a client is [`AcpAgent`].
#[derive(Debug, Default, Clone)]
pub struct AcpClient;

impl JrRole for AcpClient {}

impl DefaultCounterpart for AcpClient {
    type Counterpart = AcpAgent;
}

impl SendsToRole<AcpAgent> for AcpClient {
    fn transform_request(
        &self,
        message: crate::UntypedMessage,
        _target: &AcpAgent,
    ) -> Result<crate::UntypedMessage, crate::Error> {
        Ok(message)
    }

    fn transform_notification(
        &self,
        message: crate::UntypedMessage,
        _target: &AcpAgent,
    ) -> Result<crate::UntypedMessage, crate::Error> {
        Ok(message)
    }
}

/// The ACP agent role.
///
/// Agents respond to client requests and can send requests back to clients
/// (e.g., `sampling/createMessage` for LLM calls).
///
/// The default counterpart for an agent is [`AcpClient`].
#[derive(Debug, Default, Clone)]
pub struct AcpAgent;

impl JrRole for AcpAgent {}

impl DefaultCounterpart for AcpAgent {
    type Counterpart = AcpClient;
}

impl SendsToRole<AcpClient> for AcpAgent {
    fn transform_request(
        &self,
        message: crate::UntypedMessage,
        _target: &AcpClient,
    ) -> Result<crate::UntypedMessage, crate::Error> {
        Ok(message)
    }

    fn transform_notification(
        &self,
        message: crate::UntypedMessage,
        _target: &AcpClient,
    ) -> Result<crate::UntypedMessage, crate::Error> {
        Ok(message)
    }
}

/// The ACP proxy role.
///
/// Proxies sit between clients and agents, potentially transforming messages.
/// When sending to an agent, messages may be wrapped in a `Successor` envelope.
/// When sending to a client, messages pass through unchanged.
///
/// The default counterpart for a proxy is [`AcpClient`].
#[derive(Debug, Default, Clone)]
pub struct AcpProxy;

impl JrRole for AcpProxy {}

impl DefaultCounterpart for AcpProxy {
    type Counterpart = AcpClient;
}

impl SendsToRole<AcpClient> for AcpProxy {
    fn transform_request(
        &self,
        message: crate::UntypedMessage,
        _target: &AcpClient,
    ) -> Result<crate::UntypedMessage, crate::Error> {
        Ok(message)
    }

    fn transform_notification(
        &self,
        message: crate::UntypedMessage,
        _target: &AcpClient,
    ) -> Result<crate::UntypedMessage, crate::Error> {
        Ok(message)
    }
}

impl SendsToRole<AcpAgent> for AcpProxy {
    fn transform_request(
        &self,
        message: crate::UntypedMessage,
        _target: &AcpAgent,
    ) -> Result<crate::UntypedMessage, crate::Error> {
        SuccessorRequest {
            request: message,
            meta: None,
        }
        .to_untyped_message()
    }

    fn transform_notification(
        &self,
        message: crate::UntypedMessage,
        _target: &AcpAgent,
    ) -> Result<crate::UntypedMessage, crate::Error> {
        SuccessorNotification {
            notification: message,
            meta: None,
        }
        .to_untyped_message()
    }
}
