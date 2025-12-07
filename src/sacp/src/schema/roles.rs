//! ACP role types for type-safe protocol communication.
//!
//! These roles represent the different participants in ACP:
//! - [`AcpClient`] - The client/editor that initiates communication with agents
//! - [`AcpAgent`] - The agent that responds to client requests
//! - [`AcpProxy`] - A proxy that sits between client and agent, potentially transforming messages

use crate::{
    JrMessage,
    role::{DefaultCounterpart, JrRole, SendsTo, SendsToRole},
    schema::{
        // Client → Agent requests
        AuthenticateRequest,
        // Client → Agent notifications
        CancelNotification,
        // Agent → Client requests
        CreateTerminalRequest,
        InitializeRequest,
        KillTerminalCommandRequest,
        LoadSessionRequest,
        NewSessionRequest,
        PromptRequest,
        ReadTextFileRequest,
        ReleaseTerminalRequest,
        RequestPermissionRequest,
        // Agent → Client notifications
        SessionNotification,
        SetSessionModeRequest,
        // Proxy protocol
        SuccessorNotification,
        SuccessorRequest,
        TerminalOutputRequest,
        WaitForTerminalExitRequest,
        WriteTextFileRequest,
    },
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

// ============================================================================
// SendsTo marker trait implementations
// ============================================================================

// Client → Agent requests
impl SendsTo<AcpAgent, InitializeRequest> for AcpClient {}
impl SendsTo<AcpAgent, AuthenticateRequest> for AcpClient {}
impl SendsTo<AcpAgent, NewSessionRequest> for AcpClient {}
impl SendsTo<AcpAgent, LoadSessionRequest> for AcpClient {}
impl SendsTo<AcpAgent, PromptRequest> for AcpClient {}
impl SendsTo<AcpAgent, SetSessionModeRequest> for AcpClient {}

// Client → Agent notifications
impl SendsTo<AcpAgent, CancelNotification> for AcpClient {}

// Agent → Client requests
impl SendsTo<AcpClient, RequestPermissionRequest> for AcpAgent {}
impl SendsTo<AcpClient, ReadTextFileRequest> for AcpAgent {}
impl SendsTo<AcpClient, WriteTextFileRequest> for AcpAgent {}
impl SendsTo<AcpClient, CreateTerminalRequest> for AcpAgent {}
impl SendsTo<AcpClient, TerminalOutputRequest> for AcpAgent {}
impl SendsTo<AcpClient, ReleaseTerminalRequest> for AcpAgent {}
impl SendsTo<AcpClient, WaitForTerminalExitRequest> for AcpAgent {}
impl SendsTo<AcpClient, KillTerminalCommandRequest> for AcpAgent {}

// Agent → Client notifications
impl SendsTo<AcpClient, SessionNotification> for AcpAgent {}

// Proxy → Agent: proxy can send anything to agent that client can send
impl<M> SendsTo<AcpAgent, M> for AcpProxy where AcpClient: SendsTo<AcpAgent, M> {}

// Proxy → Client: proxy can send anything to client that agent can send
impl<M> SendsTo<AcpClient, M> for AcpProxy where AcpAgent: SendsTo<AcpClient, M> {}

// UntypedMessage can be sent in either direction (for generic code)
impl SendsTo<AcpAgent, crate::UntypedMessage> for AcpClient {}
impl SendsTo<AcpClient, crate::UntypedMessage> for AcpAgent {}
