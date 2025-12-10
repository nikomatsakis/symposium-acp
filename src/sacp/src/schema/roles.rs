//! ACP role types for type-safe protocol communication.
//!
//! These roles represent the different participants in ACP:
//! - [`AcpClient`] - The client/editor that initiates communication with agents
//! - [`AcpAgent`] - The agent that responds to client requests
//! - [`AcpProxy`] - A proxy that sits between client and agent, potentially transforming messages
//! - [`AcpConductor`] - The conductor that orchestrates proxies and agents

use crate::{
    JrNotification, JrRequest, MessageCx, UntypedMessage,
    role::{HasCounterpart, HasRemoteRole, JrRole, RemoteRoleStyle, SendsTo},
    schema::{
        // Client → Agent requests
        AuthenticateRequest,
        CancelNotification,
        CreateTerminalRequest,
        InitializeRequest,
        KillTerminalCommandRequest,
        LoadSessionRequest,
        NewSessionRequest,
        PromptRequest,
        ReadTextFileRequest,
        ReleaseTerminalRequest,
        RequestPermissionRequest,
        SessionNotification,
        SetSessionModeRequest,
        SuccessorMessage,
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
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientRole;

impl JrRole for ClientRole {}

/// The ACP agent role.
///
/// Agents respond to client requests and can send requests back to clients
/// (e.g., `sampling/createMessage` for LLM calls).
///
/// The default counterpart for an agent is [`AcpClient`].
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AgentRole;

impl JrRole for AgentRole {}

/// The ACP proxy role.
///
/// Proxies sit between clients and agents, potentially transforming messages.
/// When sending to an agent, messages are wrapped in a `Successor` envelope.
/// When sending to a client, messages pass through unchanged.
///
/// A proxy's physical counterpart is [`AcpConductor`], but it can logically
/// send to [`AcpClient`] or [`AcpAgent`] via [`SendsToRole`].
///
/// Note: `AcpProxy` does NOT implement [`DefaultCounterpart`] because
/// it has multiple logical targets and must explicitly specify the destination.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProxyRole;

impl JrRole for ProxyRole {}

impl HasRemoteRole<ClientRole> for ProxyRole {
    type Counterpart = ConductorRole;

    fn remote_style(_other: ClientRole) -> RemoteRoleStyle {
        RemoteRoleStyle::Counterpart
    }
}

impl HasRemoteRole<AgentRole> for ProxyRole {
    type Counterpart = ConductorRole;

    fn remote_style(_other: AgentRole) -> RemoteRoleStyle {
        RemoteRoleStyle::Successor
    }
}

// ============================================================================
// SendsTo marker trait implementations
// ============================================================================

// Client → Agent requests
impl SendsTo<AgentRole, InitializeRequest> for ClientRole {}
impl SendsTo<AgentRole, AuthenticateRequest> for ClientRole {}
impl SendsTo<AgentRole, NewSessionRequest> for ClientRole {}
impl SendsTo<AgentRole, LoadSessionRequest> for ClientRole {}
impl SendsTo<AgentRole, PromptRequest> for ClientRole {}
impl SendsTo<AgentRole, SetSessionModeRequest> for ClientRole {}

// Client → Agent notifications
impl SendsTo<AgentRole, CancelNotification> for ClientRole {}

// Agent → Client requests
impl SendsTo<ClientRole, RequestPermissionRequest> for AgentRole {}
impl SendsTo<ClientRole, ReadTextFileRequest> for AgentRole {}
impl SendsTo<ClientRole, WriteTextFileRequest> for AgentRole {}
impl SendsTo<ClientRole, CreateTerminalRequest> for AgentRole {}
impl SendsTo<ClientRole, TerminalOutputRequest> for AgentRole {}
impl SendsTo<ClientRole, ReleaseTerminalRequest> for AgentRole {}
impl SendsTo<ClientRole, WaitForTerminalExitRequest> for AgentRole {}
impl SendsTo<ClientRole, KillTerminalCommandRequest> for AgentRole {}

// Agent → Client notifications
impl SendsTo<ClientRole, SessionNotification> for AgentRole {}

// UntypedMessage can be sent in either direction (for generic code)
impl SendsTo<AgentRole, UntypedMessage> for ClientRole {}
impl SendsTo<ClientRole, UntypedMessage> for AgentRole {}

// Proxy → Agent: proxy can send anything to agent that client can send
impl<M> SendsTo<AgentRole, M> for ProxyRole where ClientRole: SendsTo<AgentRole, M> {}

// Proxy → Client: proxy can send anything to client that agent can send
impl<M> SendsTo<ClientRole, M> for ProxyRole where AgentRole: SendsTo<ClientRole, M> {}

// ============================================================================
// ReceivesFromRole implementations
// ============================================================================

// Proxy ↔ Client: passthrough (proxy talks to client without wrapping)

// ============================================================================
// AcpConductor role
// ============================================================================

/// The ACP conductor role.
///
/// Conductors orchestrate communication between proxies and agents.
/// They manage multiple proxy connections and a connection to the final agent.
///
/// # Connection Topologies
///
/// A conductor has:
/// - Multiple `Conductor ↔ Proxy` connections (one per proxy)
/// - One `Conductor ↔ Agent` connection (to the final agent)
///
/// Both are [`DefaultCounterpart`] connections since the conductor sends
/// directly to its physical counterpart without wrapping.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConductorRole;

impl JrRole for ConductorRole {}

// Conductor → Proxy: passthrough

// Conductor can send any message to Proxy, Agent, or Client (for generic forwarding)
impl SendsTo<ProxyRole, UntypedMessage> for ConductorRole {}
impl SendsTo<AgentRole, UntypedMessage> for ConductorRole {}
impl SendsTo<ClientRole, UntypedMessage> for ConductorRole {}

// ============================================================================
// Counterpart and DefaultCounterpart implementations
// ============================================================================

/// Client ↔ Agent: direct connection with default
impl HasCounterpart<AgentRole> for ClientRole {}

/// Client ↔ Agent: direct connection with default
impl HasCounterpart<ClientRole> for AgentRole {}

/// Conductor ↔ Proxy: connection with default
impl HasCounterpart<ProxyRole> for ConductorRole {}

/// Conductor ↔ Agent: connection with default
impl HasCounterpart<AgentRole> for ConductorRole {}

/// Conductor ↔ Client: conductor can talk to clients (but client sees conductor as agent)
impl HasCounterpart<ClientRole> for ConductorRole {}

// Proxy ↔ Conductor: physical connection, but NO DefaultCounterpart
// (proxy must explicitly specify Client or Agent as logical target)
impl HasCounterpart<ConductorRole> for ProxyRole {
    fn default_message_handler(
        message: MessageCx<Self, ConductorRole>,
    ) -> Result<(), crate::Error> {
        // Default behavior: proxy messages.
        match message {
            MessageCx::Request(request, request_cx) => {
                let cx = request_cx.connection_cx();
                match <SuccessorMessage>::parse_request(request.method(), request.params()) {
                    // If we are receiving a request from our successor (the agent),
                    // then our default is to proxy it to the client.
                    Some(Ok(SuccessorMessage {
                        message: request,
                        meta: _,
                    })) => cx
                        .send_request_to(ClientRole, request)
                        .forward_to_request_cx(request_cx),
                    Some(Err(err)) => request_cx.respond_with_error(err),
                    None => {
                        // If we are receiving a request from the client,
                        // then our default is to proxy it to the agent.
                        cx.send_request_to(AgentRole, request)
                            .forward_to_request_cx(request_cx)
                    }
                }
            }

            MessageCx::Notification(notification, cx) => {
                match <SuccessorMessage>::parse_notification(
                    notification.method(),
                    notification.params(),
                ) {
                    // If we are receiving a request from our successor (the agent),
                    // then our default is to proxy it to the client.
                    Some(Ok(SuccessorMessage {
                        message: notification,
                        meta: _,
                    })) => cx.send_notification_to(ClientRole, notification),
                    Some(Err(err)) => cx.send_error_notification(err),
                    None => {
                        // If we are receiving a request from the client,
                        // then our default is to proxy it to the agent.
                        cx.send_notification_to(AgentRole, notification)
                    }
                }
            }
        }
    }
}

// ============================================================================
// Convenience constructors for JrHandlerChain
// ============================================================================

use crate::jsonrpc::JrHandlerChain;
use crate::jsonrpc::handlers::NullHandler;

impl AgentRole {
    /// Create a handler chain for an agent talking to a client.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use sacp::schema::AcpAgent;
    ///
    /// AcpAgent.to_client()
    ///     .on_receive_request(async |req: InitializeRequest, cx| {
    ///         cx.respond(InitializeResponse { ... })
    ///     })
    ///     .serve(transport)
    ///     .await?;
    /// ```
    pub fn to_client(self) -> JrHandlerChain<NullHandler<AgentRole, ClientRole>> {
        JrHandlerChain::new(self, ClientRole)
    }
}

impl ClientRole {
    /// Create a handler chain for a client talking to an agent.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use sacp::schema::AcpClient;
    ///
    /// AcpClient.to_agent()
    ///     .connect_to(transport)?
    ///     .with_client(async |cx| {
    ///         let response = cx.send_request(InitializeRequest { ... })
    ///             .block_task()
    ///             .await?;
    ///         Ok(())
    ///     })
    ///     .await?;
    /// ```
    pub fn to_agent(self) -> JrHandlerChain<NullHandler<ClientRole, AgentRole>> {
        JrHandlerChain::new(self, AgentRole)
    }
}

impl ProxyRole {
    /// Create a handler chain for a proxy talking to a conductor.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use sacp::schema::AcpProxy;
    ///
    /// AcpProxy.to_conductor()
    ///     .on_receive_request(async |req: InitializeRequest, cx| {
    ///         // Forward to successor...
    ///     })
    ///     .serve(transport)
    ///     .await?;
    /// ```
    pub fn to_conductor(self) -> JrHandlerChain<NullHandler<ProxyRole, ConductorRole>> {
        JrHandlerChain::new(self, ConductorRole)
    }
}

impl ConductorRole {
    /// Create a handler chain for a conductor talking to a proxy.
    pub fn to_proxy(self) -> JrHandlerChain<NullHandler<ConductorRole, ProxyRole>> {
        JrHandlerChain::new(self, ProxyRole)
    }

    /// Create a handler chain for a conductor talking to an agent.
    pub fn to_agent(self) -> JrHandlerChain<NullHandler<ConductorRole, AgentRole>> {
        JrHandlerChain::new(self, AgentRole)
    }
}
