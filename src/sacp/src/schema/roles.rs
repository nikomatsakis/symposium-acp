//! ACP role types for type-safe protocol communication.
//!
//! These roles represent the different participants in ACP:
//! - [`AcpClient`] - The client/editor that initiates communication with agents
//! - [`AcpAgent`] - The agent that responds to client requests
//! - [`AcpProxy`] - A proxy that sits between client and agent, potentially transforming messages
//! - [`AcpConductor`] - The conductor that orchestrates proxies and agents

use crate::{
    Handled, JrMessage, JrMessageHandler, MessageAndCx, UntypedMessage,
    role::{Counterpart, DefaultCounterpart, JrRole, ReceivesFromRole, SendsTo, SendsToRole},
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
    util::MatchMessage,
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

impl SendsToRole<AcpAgent> for AcpClient {
    fn transform_request(
        &self,
        message: UntypedMessage,
        _target: &AcpAgent,
    ) -> Result<UntypedMessage, crate::Error> {
        Ok(message)
    }

    fn transform_notification(
        &self,
        message: UntypedMessage,
        _target: &AcpAgent,
    ) -> Result<UntypedMessage, crate::Error> {
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

impl SendsToRole<AcpClient> for AcpAgent {
    fn transform_request(
        &self,
        message: UntypedMessage,
        _target: &AcpClient,
    ) -> Result<UntypedMessage, crate::Error> {
        Ok(message)
    }

    fn transform_notification(
        &self,
        message: UntypedMessage,
        _target: &AcpClient,
    ) -> Result<UntypedMessage, crate::Error> {
        Ok(message)
    }
}

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
#[derive(Debug, Default, Clone)]
pub struct AcpProxy;

impl JrRole for AcpProxy {}

impl SendsToRole<AcpClient> for AcpProxy {
    fn transform_request(
        &self,
        message: UntypedMessage,
        _target: &AcpClient,
    ) -> Result<UntypedMessage, crate::Error> {
        Ok(message)
    }

    fn transform_notification(
        &self,
        message: UntypedMessage,
        _target: &AcpClient,
    ) -> Result<UntypedMessage, crate::Error> {
        Ok(message)
    }
}

impl SendsToRole<AcpAgent> for AcpProxy {
    fn transform_request(
        &self,
        message: UntypedMessage,
        _target: &AcpAgent,
    ) -> Result<UntypedMessage, crate::Error> {
        SuccessorRequest {
            request: message,
            meta: None,
        }
        .to_untyped_message()
    }

    fn transform_notification(
        &self,
        message: UntypedMessage,
        _target: &AcpAgent,
    ) -> Result<UntypedMessage, crate::Error> {
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
impl SendsTo<AcpAgent, UntypedMessage> for AcpClient {}
impl SendsTo<AcpClient, UntypedMessage> for AcpAgent {}

// ============================================================================
// ReceivesFromRole implementations
// ============================================================================

// Client ↔ Agent: passthrough (default counterparts, no transformation needed)
impl ReceivesFromRole<AcpAgent, AcpAgent> for AcpClient {
    async fn receive_message(
        &self,
        _sender: &AcpAgent,
        message: MessageAndCx<Self, AcpAgent>,
        handler: &mut impl JrMessageHandler<Self, AcpAgent>,
    ) -> Result<Handled<MessageAndCx<Self, AcpAgent>>, crate::Error> {
        handler.handle_message(message).await
    }
}

impl ReceivesFromRole<AcpClient, AcpClient> for AcpAgent {
    async fn receive_message(
        &self,
        _sender: &AcpClient,
        message: MessageAndCx<Self, AcpClient>,
        handler: &mut impl JrMessageHandler<Self, AcpClient>,
    ) -> Result<Handled<MessageAndCx<Self, AcpClient>>, crate::Error> {
        handler.handle_message(message).await
    }
}

// Proxy ↔ Client: passthrough (proxy talks to client without wrapping)
impl ReceivesFromRole<AcpClient, AcpClient> for AcpProxy {
    async fn receive_message(
        &self,
        _sender: &AcpClient,
        message: MessageAndCx<Self, AcpClient>,
        handler: &mut impl JrMessageHandler<Self, AcpClient>,
    ) -> Result<Handled<MessageAndCx<Self, AcpClient>>, crate::Error> {
        handler.handle_message(message).await
    }
}

// Agent ← Proxy: unwrap SuccessorRequest/SuccessorNotification envelopes
// Note: Physical connection is to Agent, logical sender is also Agent
impl ReceivesFromRole<AcpAgent, AcpAgent> for AcpProxy {
    async fn receive_message(
        &self,
        _sender: &AcpAgent,
        message: MessageAndCx<Self, AcpAgent>,
        handler: &mut impl JrMessageHandler<Self, AcpAgent>,
    ) -> Result<Handled<MessageAndCx<Self, AcpAgent>>, crate::Error> {
        MatchMessage::new(message)
            .if_request(
                async |request: SuccessorRequest<UntypedMessage>, request_cx| match handler
                    .handle_message(MessageAndCx::Request(request.request, request_cx))
                    .await?
                {
                    Handled::Yes => Ok(Handled::Yes),
                    Handled::No(MessageAndCx::Request(r, cx)) => Ok(Handled::No((
                        SuccessorRequest {
                            request: r,
                            meta: request.meta,
                        },
                        cx,
                    ))),
                    Handled::No(_) => unreachable!(),
                },
            )
            .await
            .if_notification(
                async |notification: SuccessorNotification<UntypedMessage>, cx| match handler
                    .handle_message(MessageAndCx::Notification(notification.notification, cx))
                    .await?
                {
                    Handled::Yes => Ok(Handled::Yes),
                    Handled::No(MessageAndCx::Notification(n, cx)) => Ok(Handled::No((
                        SuccessorNotification {
                            notification: n,
                            meta: notification.meta,
                        },
                        cx,
                    ))),
                    Handled::No(_) => unreachable!(),
                },
            )
            .await
            .done()
    }
}

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
#[derive(Debug, Default, Clone)]
pub struct AcpConductor;

impl JrRole for AcpConductor {}

// Conductor → Proxy: passthrough
impl SendsToRole<AcpProxy> for AcpConductor {
    fn transform_request(
        &self,
        message: UntypedMessage,
        _target: &AcpProxy,
    ) -> Result<UntypedMessage, crate::Error> {
        Ok(message)
    }

    fn transform_notification(
        &self,
        message: UntypedMessage,
        _target: &AcpProxy,
    ) -> Result<UntypedMessage, crate::Error> {
        Ok(message)
    }
}

// Conductor → Agent: passthrough
impl SendsToRole<AcpAgent> for AcpConductor {
    fn transform_request(
        &self,
        message: UntypedMessage,
        _target: &AcpAgent,
    ) -> Result<UntypedMessage, crate::Error> {
        Ok(message)
    }

    fn transform_notification(
        &self,
        message: UntypedMessage,
        _target: &AcpAgent,
    ) -> Result<UntypedMessage, crate::Error> {
        Ok(message)
    }
}

// Conductor receives from Proxy: passthrough
impl ReceivesFromRole<AcpProxy, AcpProxy> for AcpConductor {
    async fn receive_message(
        &self,
        _sender: &AcpProxy,
        message: MessageAndCx<Self, AcpProxy>,
        handler: &mut impl JrMessageHandler<Self, AcpProxy>,
    ) -> Result<Handled<MessageAndCx<Self, AcpProxy>>, crate::Error> {
        handler.handle_message(message).await
    }
}

// Conductor receives from Agent: passthrough
impl ReceivesFromRole<AcpAgent, AcpAgent> for AcpConductor {
    async fn receive_message(
        &self,
        _sender: &AcpAgent,
        message: MessageAndCx<Self, AcpAgent>,
        handler: &mut impl JrMessageHandler<Self, AcpAgent>,
    ) -> Result<Handled<MessageAndCx<Self, AcpAgent>>, crate::Error> {
        handler.handle_message(message).await
    }
}

// Proxy → Conductor: passthrough (physical connection)
impl SendsToRole<AcpConductor> for AcpProxy {
    fn transform_request(
        &self,
        message: UntypedMessage,
        _target: &AcpConductor,
    ) -> Result<UntypedMessage, crate::Error> {
        Ok(message)
    }

    fn transform_notification(
        &self,
        message: UntypedMessage,
        _target: &AcpConductor,
    ) -> Result<UntypedMessage, crate::Error> {
        Ok(message)
    }
}

// Proxy receives from Conductor: passthrough
impl ReceivesFromRole<AcpConductor, AcpConductor> for AcpProxy {
    async fn receive_message(
        &self,
        _sender: &AcpConductor,
        message: MessageAndCx<Self, AcpConductor>,
        handler: &mut impl JrMessageHandler<Self, AcpConductor>,
    ) -> Result<Handled<MessageAndCx<Self, AcpConductor>>, crate::Error> {
        handler.handle_message(message).await
    }
}

// Conductor can send any message to Proxy or Agent (for generic forwarding)
impl SendsTo<AcpProxy, UntypedMessage> for AcpConductor {}
impl SendsTo<AcpAgent, UntypedMessage> for AcpConductor {}

// ============================================================================
// Counterpart and DefaultCounterpart implementations
// ============================================================================

// Client ↔ Agent: direct connection with default
impl Counterpart<AcpAgent> for AcpClient {}
impl DefaultCounterpart<AcpAgent> for AcpClient {}

impl Counterpart<AcpClient> for AcpAgent {}
impl DefaultCounterpart<AcpClient> for AcpAgent {}

// Conductor ↔ Proxy: connection with default
impl Counterpart<AcpProxy> for AcpConductor {}
impl DefaultCounterpart<AcpProxy> for AcpConductor {}

// Conductor ↔ Agent: connection with default
impl Counterpart<AcpAgent> for AcpConductor {}
impl DefaultCounterpart<AcpAgent> for AcpConductor {}

// Proxy ↔ Conductor: physical connection, but NO DefaultCounterpart
// (proxy must explicitly specify Client or Agent as logical target)
impl Counterpart<AcpConductor> for AcpProxy {}

// ============================================================================
// Convenience constructors for JrHandlerChain
// ============================================================================

use crate::jsonrpc::JrHandlerChain;
use crate::jsonrpc::handlers::NullHandler;

impl AcpAgent {
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
    pub fn to_client(self) -> JrHandlerChain<AcpAgent, AcpClient, NullHandler> {
        JrHandlerChain::new(self, AcpClient)
    }
}

impl AcpClient {
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
    pub fn to_agent(self) -> JrHandlerChain<AcpClient, AcpAgent, NullHandler> {
        JrHandlerChain::new(self, AcpAgent)
    }
}

impl AcpProxy {
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
    pub fn to_conductor(self) -> JrHandlerChain<AcpProxy, AcpConductor, NullHandler> {
        JrHandlerChain::new(self, AcpConductor)
    }
}

impl AcpConductor {
    /// Create a handler chain for a conductor talking to a proxy.
    pub fn to_proxy(self) -> JrHandlerChain<AcpConductor, AcpProxy, NullHandler> {
        JrHandlerChain::new(self, AcpProxy)
    }

    /// Create a handler chain for a conductor talking to an agent.
    pub fn to_agent(self) -> JrHandlerChain<AcpConductor, AcpAgent, NullHandler> {
        JrHandlerChain::new(self, AcpAgent)
    }
}
