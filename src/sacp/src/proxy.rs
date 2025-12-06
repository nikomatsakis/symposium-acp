//! Proxy support for building ACP proxy components.
//!
//! Proxies are modular components that sit between an editor and an agent,
//! intercepting and transforming messages. They enable composable agent
//! architectures where functionality can be added without modifying the base agent.
//!
//! ```text
//! Editor → Proxy 1 → Proxy 2 → Agent
//! ```
//!
//! ## Quick Start
//!
//! The simplest proxy just forwards messages unchanged:
//!
//! ```rust,ignore
//! use sacp::JrHandlerChain;
//! use sacp::proxy::AcpProxyExt;
//! use sacp::mcp_server::McpServiceRegistry;
//!
//! JrHandlerChain::new()
//!     .name("my-proxy")
//!     .provide_mcp(McpServiceRegistry::default())
//!     .proxy()
//!     .serve(connection)
//!     .await?;
//! ```

use std::marker::PhantomData;

use crate::handler::ChainedHandler;
use crate::mcp_server::McpServiceRegistry;
use crate::schema::{
    InitializeRequest, InitializeResponse, SuccessorNotification, SuccessorRequest,
};
use crate::{
    Handled, JrConnectionCx, JrHandlerChain, JrMessage, JrMessageHandler, JrNotification,
    JrRequest, JrRequestCx, MessageAndCx, MetaCapabilityExt, Proxy, UntypedMessage,
};

// =============================================================================
// Extension traits
// =============================================================================

/// Extension trait for JrHandlerChain that adds proxy-specific functionality
pub trait AcpProxyExt<H: JrMessageHandler> {
    /// Adds a handler for requests received from the successor component.
    ///
    /// The provided handler will receive unwrapped ACP messages - the
    /// `_proxy/successor/receive/*` protocol wrappers are handled automatically.
    /// Your handler processes normal ACP requests and notifications as if it were
    /// a regular ACP component.
    fn on_receive_request_from_successor<R, F>(
        self,
        op: F,
    ) -> JrHandlerChain<ChainedHandler<H, RequestFromSuccessorHandler<R, F>>>
    where
        R: JrRequest,
        F: AsyncFnMut(R, JrRequestCx<R::Response>) -> Result<(), crate::Error>;

    /// Adds a handler for notifications received from the successor component.
    ///
    /// The provided handler will receive unwrapped ACP messages - the
    /// `_proxy/successor/receive/*` protocol wrappers are handled automatically.
    /// Your handler processes normal ACP requests and notifications as if it were
    /// a regular ACP component.
    fn on_receive_notification_from_successor<N, F>(
        self,
        op: F,
    ) -> JrHandlerChain<ChainedHandler<H, NotificationFromSuccessorHandler<N, F>>>
    where
        N: JrNotification,
        F: AsyncFnMut(N, JrConnectionCx) -> Result<(), crate::Error>;

    /// Adds a handler for messages received from the successor component.
    ///
    /// The provided handler will receive unwrapped ACP messages - the
    /// `_proxy/successor/receive/*` protocol wrappers are handled automatically.
    /// Your handler processes normal ACP requests and notifications as if it were
    /// a regular ACP component.
    fn on_receive_message_from_successor<R, N, F>(
        self,
        op: F,
    ) -> JrHandlerChain<ChainedHandler<H, MessageFromSuccessorHandler<R, N, F>>>
    where
        R: JrRequest,
        N: JrNotification,
        F: AsyncFnMut(MessageAndCx<R, N>) -> Result<(), crate::Error>;

    /// Installs a proxy layer that proxies all requests/notifications to/from the successor.
    /// This is typically the last component in the chain.
    fn proxy(self) -> JrHandlerChain<ChainedHandler<H, ProxyHandler>>;

    /// Provide MCP servers to downstream successors.
    /// This layer will modify `session/new` requests to include those MCP servers
    /// (unless you intercept them earlier).
    fn provide_mcp(
        self,
        registry: impl AsRef<McpServiceRegistry>,
    ) -> JrHandlerChain<ChainedHandler<H, McpServiceRegistry>>;
}

impl<H: JrMessageHandler> AcpProxyExt<H> for JrHandlerChain<H> {
    fn on_receive_request_from_successor<R, F>(
        self,
        op: F,
    ) -> JrHandlerChain<ChainedHandler<H, RequestFromSuccessorHandler<R, F>>>
    where
        R: JrRequest,
        F: AsyncFnMut(R, JrRequestCx<R::Response>) -> Result<(), crate::Error>,
    {
        self.with_handler(RequestFromSuccessorHandler::new(op))
    }

    fn on_receive_notification_from_successor<N, F>(
        self,
        op: F,
    ) -> JrHandlerChain<ChainedHandler<H, NotificationFromSuccessorHandler<N, F>>>
    where
        N: JrNotification,
        F: AsyncFnMut(N, JrConnectionCx) -> Result<(), crate::Error>,
    {
        self.with_handler(NotificationFromSuccessorHandler::new(op))
    }

    fn on_receive_message_from_successor<R, N, F>(
        self,
        op: F,
    ) -> JrHandlerChain<ChainedHandler<H, MessageFromSuccessorHandler<R, N, F>>>
    where
        R: JrRequest,
        N: JrNotification,
        F: AsyncFnMut(MessageAndCx<R, N>) -> Result<(), crate::Error>,
    {
        self.with_handler(MessageFromSuccessorHandler::new(op))
    }

    fn proxy(self) -> JrHandlerChain<ChainedHandler<H, ProxyHandler>> {
        self.with_handler(ProxyHandler {})
    }

    fn provide_mcp(
        self,
        registry: impl AsRef<McpServiceRegistry>,
    ) -> JrHandlerChain<ChainedHandler<H, McpServiceRegistry>> {
        self.with_handler(registry.as_ref().clone())
    }
}

/// Extension trait for [`JrConnectionCx`] that adds methods for sending to successor.
///
/// This trait provides convenient methods for proxies to forward messages downstream
/// to their successor component (next proxy or agent). Messages are automatically
/// wrapped in the `_proxy/successor/send/*` protocol format.
pub trait JrCxExt {
    /// Send a request to the successor component.
    ///
    /// The request is automatically wrapped in a `SuccessorRequest` and sent
    /// using the `_proxy/successor/request` method. The orchestrator routes
    /// it to the next component in the chain.
    fn send_request_to_successor<Req: JrRequest>(
        &self,
        request: Req,
    ) -> crate::JrResponse<Req::Response>;

    /// Send a notification to the successor component.
    ///
    /// The notification is automatically wrapped in a `SuccessorNotification`
    /// and sent using the `_proxy/successor/notification` method. The
    /// orchestrator routes it to the next component in the chain.
    ///
    /// Notifications are fire-and-forget - no response is expected.
    fn send_notification_to_successor<Req: JrNotification>(
        &self,
        notification: Req,
    ) -> Result<(), crate::Error>;
}

impl JrCxExt for JrConnectionCx {
    fn send_request_to_successor<Req: JrRequest>(
        &self,
        request: Req,
    ) -> crate::JrResponse<Req::Response> {
        let wrapper = SuccessorRequest {
            request,
            meta: None,
        };
        self.send_request(wrapper)
    }

    fn send_notification_to_successor<Req: JrNotification>(
        &self,
        notification: Req,
    ) -> Result<(), crate::Error> {
        let wrapper = SuccessorNotification {
            notification,
            meta: None,
        };
        self.send_notification(wrapper)
    }
}

// =============================================================================
// Handlers
// =============================================================================

/// Handler to process a message of type `R` coming from the successor component.
pub struct MessageFromSuccessorHandler<R, N, F>
where
    R: JrRequest,
    N: JrNotification,
    F: AsyncFnMut(MessageAndCx<R, N>) -> Result<(), crate::Error>,
{
    handler: F,
    phantom: PhantomData<fn(R, N)>,
}

impl<R, N, F> MessageFromSuccessorHandler<R, N, F>
where
    R: JrRequest,
    N: JrNotification,
    F: AsyncFnMut(MessageAndCx<R, N>) -> Result<(), crate::Error>,
{
    /// Creates a new handler for requests from the successor
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            phantom: PhantomData,
        }
    }
}

impl<R, N, F> JrMessageHandler for MessageFromSuccessorHandler<R, N, F>
where
    R: JrRequest,
    N: JrNotification,
    F: AsyncFnMut(MessageAndCx<R, N>) -> Result<(), crate::Error>,
{
    async fn handle_message(
        &mut self,
        message: MessageAndCx,
    ) -> Result<Handled<MessageAndCx>, crate::Error> {
        match message {
            MessageAndCx::Request(request, request_cx) => {
                tracing::trace!(
                    request_type = std::any::type_name::<R>(),
                    message = ?request,
                    "MessageFromSuccessorHandler::handle_message"
                );
                match <SuccessorRequest<R>>::parse_request(&request.method, &request.params) {
                    Some(Ok(request)) => {
                        tracing::trace!(
                            ?request,
                            "RequestHandler::handle_request: parse completed"
                        );
                        (self.handler)(MessageAndCx::Request(request.request, request_cx.cast()))
                            .await?;
                        Ok(Handled::Yes)
                    }
                    Some(Err(err)) => {
                        tracing::trace!(?err, "RequestHandler::handle_request: parse errored");
                        Err(err)
                    }
                    None => {
                        tracing::trace!("RequestHandler::handle_request: parse failed");
                        Ok(Handled::No(MessageAndCx::Request(request, request_cx)))
                    }
                }
            }
            MessageAndCx::Notification(notification, connection_cx) => {
                tracing::trace!(
                    ?notification,
                    "NotificationFromSuccessorHandler::handle_message"
                );
                match <SuccessorNotification<N>>::parse_notification(
                    &notification.method,
                    &notification.params,
                ) {
                    Some(Ok(notification)) => {
                        tracing::trace!(
                            ?notification,
                            "NotificationFromSuccessorHandler::handle_message: parse completed"
                        );
                        (self.handler)(MessageAndCx::Notification(
                            notification.notification,
                            connection_cx,
                        ))
                        .await?;
                        Ok(Handled::Yes)
                    }
                    Some(Err(err)) => {
                        tracing::trace!(
                            ?err,
                            "NotificationFromSuccessorHandler::handle_message: parse errored"
                        );
                        Err(err)
                    }
                    None => {
                        tracing::trace!(
                            "NotificationFromSuccessorHandler::handle_message: parse failed"
                        );
                        Ok(Handled::No(MessageAndCx::Notification(
                            notification,
                            connection_cx,
                        )))
                    }
                }
            }
        }
    }

    fn describe_chain(&self) -> impl std::fmt::Debug {
        std::any::type_name::<R>()
    }
}

/// Handler to process a request of type `R` coming from the successor component.
pub struct RequestFromSuccessorHandler<R, F>
where
    R: JrRequest,
    F: AsyncFnMut(R, JrRequestCx<R::Response>) -> Result<(), crate::Error>,
{
    handler: F,
    phantom: PhantomData<fn(R)>,
}

impl<R, F> RequestFromSuccessorHandler<R, F>
where
    R: JrRequest,
    F: AsyncFnMut(R, JrRequestCx<R::Response>) -> Result<(), crate::Error>,
{
    /// Creates a new handler for requests from the successor
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            phantom: PhantomData,
        }
    }
}

impl<R, F> JrMessageHandler for RequestFromSuccessorHandler<R, F>
where
    R: JrRequest,
    F: AsyncFnMut(R, JrRequestCx<R::Response>) -> Result<(), crate::Error>,
{
    async fn handle_message(
        &mut self,
        message: MessageAndCx,
    ) -> Result<Handled<MessageAndCx>, crate::Error> {
        let MessageAndCx::Request(request, cx) = message else {
            return Ok(Handled::No(message));
        };

        tracing::debug!(
            request_type = std::any::type_name::<R>(),
            message = ?request,
            "RequestHandler::handle_request"
        );
        match <SuccessorRequest<R>>::parse_request(&request.method, &request.params) {
            Some(Ok(request)) => {
                tracing::trace!(?request, "RequestHandler::handle_request: parse completed");
                (self.handler)(request.request, cx.cast()).await?;
                Ok(Handled::Yes)
            }
            Some(Err(err)) => {
                tracing::trace!(?err, "RequestHandler::handle_request: parse errored");
                Err(err)
            }
            None => {
                tracing::trace!("RequestHandler::handle_request: parse failed");
                Ok(Handled::No(MessageAndCx::Request(request, cx)))
            }
        }
    }

    fn describe_chain(&self) -> impl std::fmt::Debug {
        std::any::type_name::<R>()
    }
}

/// Handler to process a notification of type `N` coming from the successor component.
pub struct NotificationFromSuccessorHandler<N, F>
where
    N: JrNotification,
    F: AsyncFnMut(N, JrConnectionCx) -> Result<(), crate::Error>,
{
    handler: F,
    phantom: PhantomData<fn(N)>,
}

impl<N, F> NotificationFromSuccessorHandler<N, F>
where
    N: JrNotification,
    F: AsyncFnMut(N, JrConnectionCx) -> Result<(), crate::Error>,
{
    /// Creates a new handler for notifications from the successor
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            phantom: PhantomData,
        }
    }
}

impl<N, F> JrMessageHandler for NotificationFromSuccessorHandler<N, F>
where
    N: JrNotification,
    F: AsyncFnMut(N, JrConnectionCx) -> Result<(), crate::Error>,
{
    async fn handle_message(
        &mut self,
        message: MessageAndCx,
    ) -> Result<Handled<MessageAndCx>, crate::Error> {
        let MessageAndCx::Notification(message, cx) = message else {
            return Ok(Handled::No(message));
        };

        match <SuccessorNotification<N>>::parse_notification(&message.method, &message.params) {
            Some(Ok(notification)) => {
                tracing::trace!(
                    ?notification,
                    "NotificationFromSuccessorHandler::handle_request: parse completed"
                );
                (self.handler)(notification.notification, cx).await?;
                Ok(Handled::Yes)
            }
            Some(Err(err)) => {
                tracing::trace!(
                    ?err,
                    "NotificationFromSuccessorHandler::handle_request: parse errored"
                );
                Err(err)
            }
            None => {
                tracing::trace!("NotificationFromSuccessorHandler::handle_request: parse failed");
                Ok(Handled::No(MessageAndCx::Notification(message, cx)))
            }
        }
    }

    fn describe_chain(&self) -> impl std::fmt::Debug {
        format!("FromSuccessor<{}>", std::any::type_name::<N>())
    }
}

/// Handler for the "default proxy" behavior.
pub struct ProxyHandler {}

impl JrMessageHandler for ProxyHandler {
    fn describe_chain(&self) -> impl std::fmt::Debug {
        "proxy"
    }

    async fn handle_message(
        &mut self,
        message: MessageAndCx,
    ) -> Result<Handled<MessageAndCx>, crate::Error> {
        tracing::debug!(
            message = ?message.message(),
            "ProxyHandler::handle_request"
        );

        match message {
            MessageAndCx::Request(request, request_cx) => {
                // If we receive a request from the successor, send it to our predecessor.
                if let Some(result) = <SuccessorRequest<UntypedMessage>>::parse_request(
                    &request.method,
                    &request.params,
                ) {
                    let request = result?;
                    request_cx
                        .connection_cx()
                        .send_request(request.request)
                        .forward_to_request_cx(request_cx)?;
                    return Ok(Handled::Yes);
                }

                // If we receive "Initialize", require the proxy capability (and remove it)
                if let Some(result) =
                    InitializeRequest::parse_request(&request.method, &request.params)
                {
                    let request = result?;
                    return self
                        .forward_initialize(request, request_cx.cast())
                        .await
                        .map(|()| Handled::Yes);
                }

                // If we receive any other request, send it to our successor.
                request_cx
                    .connection_cx()
                    .send_request_to_successor(request)
                    .forward_to_request_cx(request_cx)?;
                Ok(Handled::Yes)
            }

            MessageAndCx::Notification(notification, cx) => {
                // If we receive a request from the successor, send it to our predecessor.
                if let Some(result) = <SuccessorNotification<UntypedMessage>>::parse_notification(
                    &notification.method,
                    &notification.params,
                ) {
                    match result {
                        Ok(r) => {
                            cx.send_notification(r.notification)?;
                            return Ok(Handled::Yes);
                        }
                        Err(err) => return Err(err),
                    }
                }

                // If we receive any other request, send it to our successor.
                cx.send_notification_to_successor(notification)?;
                Ok(Handled::Yes)
            }
        }
    }
}

impl ProxyHandler {
    /// Proxy initialization requires (1) a `Proxy` capability to be
    /// provided by the conductor and (2) provides a `Proxy` capability
    /// in our response.
    async fn forward_initialize(
        &mut self,
        mut request: InitializeRequest,
        request_cx: JrRequestCx<InitializeResponse>,
    ) -> Result<(), crate::Error> {
        tracing::debug!(
            method = request_cx.method(),
            params = ?request,
            "ProxyHandler::forward_initialize"
        );

        if !request.has_meta_capability(Proxy) {
            request_cx.respond_with_error(
                crate::Error::invalid_params()
                    .with_data("this command requires the proxy capability"),
            )?;
            return Ok(());
        }

        request = request.remove_meta_capability(Proxy);
        request_cx
            .connection_cx()
            .send_request_to_successor(request)
            .await_when_result_received(async move |mut result| {
                result = result.map(|r| r.add_meta_capability(Proxy));
                request_cx.respond_with_result(result)
            })
    }
}
