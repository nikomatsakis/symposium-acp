use crate::jsonrpc::responder::JrResponder;
use crate::jsonrpc::{Handled, IntoHandled, JrMessageHandler};
use crate::link::{HasPeer, JrLink};
use crate::peer::JrPeer;
use crate::{BoxFuture, JrConnectionCx, JrNotification, JrRequest, MessageCx, UntypedMessage};
// Types re-exported from crate root
use super::JrRequestCx;
use futures::StreamExt;
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use std::marker::PhantomData;
use std::ops::AsyncFnMut;

/// Null handler that accepts no messages.
pub struct NullHandler<Link: JrLink> {
    role: Link,
}

impl<Link: JrLink> NullHandler<Link> {
    /// Creates a new null handler.
    pub fn new(role: Link) -> Self {
        Self { role }
    }

    /// Returns the role.
    pub fn role(&self) -> Link {
        self.role
    }
}

impl<Link: JrLink> JrMessageHandler for NullHandler<Link> {
    type Link = Link;

    fn describe_chain(&self) -> impl std::fmt::Debug {
        "(null)"
    }

    async fn handle_message(
        &mut self,
        message: MessageCx,
        _cx: JrConnectionCx<Link>,
    ) -> Result<Handled<MessageCx>, crate::Error> {
        Ok(Handled::No {
            message,
            retry: false,
        })
    }
}

/// Handler for typed request messages
pub struct RequestHandler<
    Link: JrLink,
    Peer: JrPeer,
    Req: JrRequest = UntypedMessage,
    F = (),
    ToFut = (),
> {
    handler: F,
    to_future_hack: ToFut,
    phantom: PhantomData<fn(Link, Peer, Req)>,
}

impl<Link: JrLink, Peer: JrPeer, Req: JrRequest, F, ToFut>
    RequestHandler<Link, Peer, Req, F, ToFut>
{
    /// Creates a new request handler
    pub fn new(_peer: Peer, _link: Link, handler: F, to_future_hack: ToFut) -> Self {
        Self {
            handler,
            to_future_hack,
            phantom: PhantomData,
        }
    }
}

impl<Link: JrLink, Peer: JrPeer, Req, F, ToFut> JrMessageHandler
    for RequestHandler<Link, Peer, Req, F, ToFut>
where
    Link: HasPeer<Peer>,
    Req: JrRequest,
    F: AsyncFnMut(
            Req,
            JrRequestCx<Req::Response>,
            JrConnectionCx<Link>,
        ) -> Result<(), crate::Error>
        + Send,
    ToFut: Fn(
            &mut F,
            Req,
            JrRequestCx<Req::Response>,
            JrConnectionCx<Link>,
        ) -> crate::BoxFuture<'_, Result<(), crate::Error>>
        + Send
        + Sync,
{
    type Link = Link;

    fn describe_chain(&self) -> impl std::fmt::Debug {
        std::any::type_name::<Req>()
    }

    async fn handle_message(
        &mut self,
        message_cx: MessageCx,
        connection_cx: JrConnectionCx<Link>,
    ) -> Result<Handled<MessageCx>, crate::Error> {
        let remote_style = Link::remote_style(Peer::default());
        remote_style
            .handle_incoming_message(
                message_cx,
                connection_cx,
                async |message_cx, connection_cx| match message_cx {
                    MessageCx::Request(message, request_cx) => {
                        tracing::debug!(
                            request_type = std::any::type_name::<Req>(),
                            message = ?message,
                            "RequestHandler::handle_request"
                        );
                        match Req::parse_message(&message.method, &message.params) {
                            Some(Ok(req)) => {
                                tracing::trace!(
                                    ?req,
                                    "RequestHandler::handle_request: parse completed"
                                );
                                let typed_request_cx = request_cx.cast();
                                (self.to_future_hack)(
                                    &mut self.handler,
                                    req,
                                    typed_request_cx,
                                    connection_cx,
                                )
                                .await?;
                                Ok(Handled::Yes)
                            }
                            Some(Err(err)) => {
                                tracing::trace!(
                                    ?err,
                                    "RequestHandler::handle_request: parse errored"
                                );
                                Err(err)
                            }
                            None => {
                                tracing::trace!("RequestHandler::handle_request: parse failed");
                                Ok(Handled::No {
                                    message: MessageCx::Request(message, request_cx),
                                    retry: false,
                                })
                            }
                        }
                    }

                    MessageCx::Notification(..) => Ok(Handled::No {
                        message: message_cx,
                        retry: false,
                    }),
                },
            )
            .await
    }
}

/// Handler for typed notification messages
pub struct NotificationHandler<
    Link: JrLink,
    Peer: JrPeer,
    Notif: JrNotification = UntypedMessage,
    F = (),
    ToFut = (),
> {
    handler: F,
    to_future_hack: ToFut,
    phantom: PhantomData<fn(Link, Peer, Notif)>,
}

impl<Link: JrLink, Peer: JrPeer, Notif: JrNotification, F, ToFut>
    NotificationHandler<Link, Peer, Notif, F, ToFut>
{
    /// Creates a new notification handler
    pub fn new(_peer: Peer, _link: Link, handler: F, to_future_hack: ToFut) -> Self {
        Self {
            handler,
            to_future_hack,
            phantom: PhantomData,
        }
    }
}

impl<Link: JrLink, Peer: JrPeer, Notif, F, ToFut> JrMessageHandler
    for NotificationHandler<Link, Peer, Notif, F, ToFut>
where
    Link: HasPeer<Peer>,
    Notif: JrNotification,
    F: AsyncFnMut(Notif, JrConnectionCx<Link>) -> Result<(), crate::Error> + Send,
    ToFut: Fn(&mut F, Notif, JrConnectionCx<Link>) -> crate::BoxFuture<'_, Result<(), crate::Error>>
        + Send
        + Sync,
{
    type Link = Link;

    fn describe_chain(&self) -> impl std::fmt::Debug {
        std::any::type_name::<Notif>()
    }

    async fn handle_message(
        &mut self,
        message_cx: MessageCx,
        connection_cx: JrConnectionCx<Link>,
    ) -> Result<Handled<MessageCx>, crate::Error> {
        let remote_style = Link::remote_style(Peer::default());
        remote_style
            .handle_incoming_message(
                message_cx,
                connection_cx,
                async |message_cx, connection_cx| match message_cx {
                    MessageCx::Notification(message) => {
                        tracing::debug!(
                            request_type = std::any::type_name::<Notif>(),
                            message = ?message,
                            "NotificationHandler::handle_message"
                        );
                        match Notif::parse_message(&message.method, &message.params) {
                            Some(Ok(notif)) => {
                                tracing::trace!(
                                    ?notif,
                                    "NotificationHandler::handle_notification: parse completed"
                                );
                                (self.to_future_hack)(&mut self.handler, notif, connection_cx)
                                    .await?;
                                Ok(Handled::Yes)
                            }
                            Some(Err(err)) => {
                                tracing::trace!(
                                    ?err,
                                    "NotificationHandler::handle_notification: parse errored"
                                );
                                Err(err)
                            }
                            None => {
                                tracing::trace!(
                                    "NotificationHandler::handle_notification: parse failed"
                                );
                                Ok(Handled::No {
                                    message: MessageCx::Notification(message),
                                    retry: false,
                                })
                            }
                        }
                    }

                    MessageCx::Request(..) => Ok(Handled::No {
                        message: message_cx,
                        retry: false,
                    }),
                },
            )
            .await
    }
}

/// Handler that handles both requests and notifications of specific types.
pub struct MessageHandler<
    Link: JrLink,
    Peer: JrPeer,
    Req: JrRequest = UntypedMessage,
    Notif: JrNotification = UntypedMessage,
    F = (),
    ToFut = (),
> {
    handler: F,
    to_future_hack: ToFut,
    phantom: PhantomData<fn(Link, Peer, Req, Notif)>,
}

impl<Link: JrLink, Peer: JrPeer, Req: JrRequest, Notif: JrNotification, F, ToFut>
    MessageHandler<Link, Peer, Req, Notif, F, ToFut>
{
    /// Creates a new message handler
    pub fn new(_peer: Peer, _link: Link, handler: F, to_future_hack: ToFut) -> Self {
        Self {
            handler,
            to_future_hack,
            phantom: PhantomData,
        }
    }
}

impl<Link: JrLink, Peer: JrPeer, Req: JrRequest, Notif: JrNotification, F, ToFut> JrMessageHandler
    for MessageHandler<Link, Peer, Req, Notif, F, ToFut>
where
    Link: HasPeer<Peer>,
    F: AsyncFnMut(MessageCx<Req, Notif>, JrConnectionCx<Link>) -> Result<(), crate::Error> + Send,
    ToFut: Fn(
            &mut F,
            MessageCx<Req, Notif>,
            JrConnectionCx<Link>,
        ) -> crate::BoxFuture<'_, Result<(), crate::Error>>
        + Send
        + Sync,
{
    type Link = Link;

    fn describe_chain(&self) -> impl std::fmt::Debug {
        format!(
            "({}, {})",
            std::any::type_name::<Req>(),
            std::any::type_name::<Notif>()
        )
    }

    async fn handle_message(
        &mut self,
        message_cx: MessageCx,
        connection_cx: JrConnectionCx<Link>,
    ) -> Result<Handled<MessageCx>, crate::Error> {
        let remote_style = Link::remote_style(Peer::default());
        remote_style
            .handle_incoming_message(
                message_cx,
                connection_cx,
                async |message_cx, connection_cx| match message_cx
                    .into_typed_message_cx::<Req, Notif>()?
                {
                    Ok(typed_message_cx) => {
                        (self.to_future_hack)(&mut self.handler, typed_message_cx, connection_cx)
                            .await?;
                        Ok(Handled::Yes)
                    }

                    Err(message_cx) => Ok(Handled::No {
                        message: message_cx,
                        retry: false,
                    }),
                },
            )
            .await
    }
}

// =============================================================================
// Sync Handler Types
// =============================================================================
//
// These handlers use synchronous closures (FnMut) instead of async closures.
// They're useful for routing/filtering use cases where the decision to handle
// a message is based on runtime data (e.g., checking a connection ID) rather
// than expensive async operations.

/// Sync handler for typed request messages.
///
/// Unlike [`RequestHandler`], this uses a synchronous closure. This is useful
/// for routing decisions that don't require async work.
pub struct RequestHandlerSync<Link: JrLink, Peer: JrPeer, Req: JrRequest = UntypedMessage, F = ()> {
    handler: F,
    phantom: PhantomData<fn(Link, Peer, Req)>,
}

impl<Link: JrLink, Peer: JrPeer, Req: JrRequest, F> RequestHandlerSync<Link, Peer, Req, F> {
    /// Creates a new sync request handler
    pub fn new(_peer: Peer, _link: Link, handler: F) -> Self {
        Self {
            handler,
            phantom: PhantomData,
        }
    }
}

impl<Link: JrLink, Peer: JrPeer, Req, F, T> JrMessageHandler
    for RequestHandlerSync<Link, Peer, Req, F>
where
    Link: HasPeer<Peer>,
    Req: JrRequest,
    F: FnMut(Req, JrRequestCx<Req::Response>, JrConnectionCx<Link>) -> Result<T, crate::Error>
        + Send,
    T: crate::IntoHandled<(Req, JrRequestCx<Req::Response>)>,
{
    type Link = Link;

    fn describe_chain(&self) -> impl std::fmt::Debug {
        format!("{}(sync)", std::any::type_name::<Req>())
    }

    async fn handle_message(
        &mut self,
        message_cx: MessageCx,
        connection_cx: JrConnectionCx<Link>,
    ) -> Result<Handled<MessageCx>, crate::Error> {
        let remote_style = Link::remote_style(Peer::default());
        remote_style.handle_incoming_message_sync(
            message_cx,
            connection_cx,
            |message_cx, connection_cx| match message_cx {
                MessageCx::Request(message, request_cx) => {
                    tracing::debug!(
                        request_type = std::any::type_name::<Req>(),
                        message = ?message,
                        "RequestHandlerSync::handle_request"
                    );
                    match Req::parse_message(&message.method, &message.params) {
                        Some(Ok(req)) => {
                            tracing::trace!(
                                ?req,
                                "RequestHandlerSync::handle_request: parse completed"
                            );
                            let typed_request_cx = request_cx.cast();
                            let result = (self.handler)(req, typed_request_cx, connection_cx)?;
                            match result.into_handled() {
                                Handled::Yes => Ok(Handled::Yes),
                                Handled::No {
                                    message: (request, request_cx),
                                    retry,
                                } => {
                                    let untyped = request.to_untyped_message()?;
                                    Ok(Handled::No {
                                        message: MessageCx::Request(
                                            untyped,
                                            request_cx.erase_to_json(),
                                        ),
                                        retry,
                                    })
                                }
                            }
                        }
                        Some(Err(err)) => {
                            tracing::trace!(
                                ?err,
                                "RequestHandlerSync::handle_request: parse errored"
                            );
                            Err(err)
                        }
                        None => {
                            tracing::trace!("RequestHandlerSync::handle_request: parse failed");
                            Ok(Handled::No {
                                message: MessageCx::Request(message, request_cx),
                                retry: false,
                            })
                        }
                    }
                }

                MessageCx::Notification(..) => Ok(Handled::No {
                    message: message_cx,
                    retry: false,
                }),
            },
        )
    }
}

/// Sync handler for typed notification messages.
///
/// Unlike [`NotificationHandler`], this uses a synchronous closure. This is useful
/// for routing decisions that don't require async work.
pub struct NotificationHandlerSync<
    Link: JrLink,
    Peer: JrPeer,
    Notif: JrNotification = UntypedMessage,
    F = (),
> {
    handler: F,
    phantom: PhantomData<fn(Link, Peer, Notif)>,
}

impl<Link: JrLink, Peer: JrPeer, Notif: JrNotification, F>
    NotificationHandlerSync<Link, Peer, Notif, F>
{
    /// Creates a new sync notification handler
    pub fn new(_peer: Peer, _link: Link, handler: F) -> Self {
        Self {
            handler,
            phantom: PhantomData,
        }
    }
}

impl<Link: JrLink, Peer: JrPeer, Notif, F, T> JrMessageHandler
    for NotificationHandlerSync<Link, Peer, Notif, F>
where
    Link: HasPeer<Peer>,
    Notif: JrNotification,
    F: FnMut(Notif, JrConnectionCx<Link>) -> Result<T, crate::Error> + Send,
    T: crate::IntoHandled<(Notif, JrConnectionCx<Link>)>,
{
    type Link = Link;

    fn describe_chain(&self) -> impl std::fmt::Debug {
        format!("{}(sync)", std::any::type_name::<Notif>())
    }

    async fn handle_message(
        &mut self,
        message_cx: MessageCx,
        connection_cx: JrConnectionCx<Link>,
    ) -> Result<Handled<MessageCx>, crate::Error> {
        let remote_style = Link::remote_style(Peer::default());
        remote_style.handle_incoming_message_sync(
            message_cx,
            connection_cx,
            |message_cx, connection_cx| match message_cx {
                MessageCx::Notification(message) => {
                    tracing::debug!(
                        notification_type = std::any::type_name::<Notif>(),
                        message = ?message,
                        "NotificationHandlerSync::handle_notification"
                    );
                    match Notif::parse_message(&message.method, &message.params) {
                        Some(Ok(notif)) => {
                            tracing::trace!(
                                ?notif,
                                "NotificationHandlerSync::handle_notification: parse completed"
                            );
                            let result = (self.handler)(notif, connection_cx)?;
                            match result.into_handled() {
                                Handled::Yes => Ok(Handled::Yes),
                                Handled::No {
                                    message: (notification, _cx),
                                    retry,
                                } => {
                                    let untyped = notification.to_untyped_message()?;
                                    Ok(Handled::No {
                                        message: MessageCx::Notification(untyped),
                                        retry,
                                    })
                                }
                            }
                        }
                        Some(Err(err)) => {
                            tracing::trace!(
                                ?err,
                                "NotificationHandlerSync::handle_notification: parse errored"
                            );
                            Err(err)
                        }
                        None => {
                            tracing::trace!(
                                "NotificationHandlerSync::handle_notification: parse failed"
                            );
                            Ok(Handled::No {
                                message: MessageCx::Notification(message),
                                retry: false,
                            })
                        }
                    }
                }

                MessageCx::Request(..) => Ok(Handled::No {
                    message: message_cx,
                    retry: false,
                }),
            },
        )
    }
}

/// Sync handler that handles both requests and notifications of specific types.
///
/// Unlike [`MessageHandler`], this uses a synchronous closure. This is useful
/// for routing decisions that don't require async work.
pub struct MessageHandlerSync<
    Link: JrLink,
    Peer: JrPeer,
    Req: JrRequest = UntypedMessage,
    Notif: JrNotification = UntypedMessage,
    F = (),
> {
    handler: F,
    phantom: PhantomData<fn(Link, Peer, Req, Notif)>,
}

impl<Link: JrLink, Peer: JrPeer, Req: JrRequest, Notif: JrNotification, F>
    MessageHandlerSync<Link, Peer, Req, Notif, F>
{
    /// Creates a new sync message handler
    pub fn new(_peer: Peer, _link: Link, handler: F) -> Self {
        Self {
            handler,
            phantom: PhantomData,
        }
    }
}

impl<Link: JrLink, Peer: JrPeer, Req: JrRequest, Notif: JrNotification, F, T> JrMessageHandler
    for MessageHandlerSync<Link, Peer, Req, Notif, F>
where
    Link: HasPeer<Peer>,
    F: FnMut(MessageCx<Req, Notif>, JrConnectionCx<Link>) -> Result<T, crate::Error> + Send,
    T: IntoHandled<MessageCx<Req, Notif>>,
{
    type Link = Link;

    fn describe_chain(&self) -> impl std::fmt::Debug {
        format!(
            "({}, {})(sync)",
            std::any::type_name::<Req>(),
            std::any::type_name::<Notif>()
        )
    }

    async fn handle_message(
        &mut self,
        message_cx: MessageCx,
        connection_cx: JrConnectionCx<Link>,
    ) -> Result<Handled<MessageCx>, crate::Error> {
        let remote_style = Link::remote_style(Peer::default());
        remote_style.handle_incoming_message_sync(
            message_cx,
            connection_cx,
            |message_cx, connection_cx| match message_cx.into_typed_message_cx::<Req, Notif>()? {
                Ok(typed_message_cx) => {
                    let result = (self.handler)(typed_message_cx, connection_cx)?;
                    match result.into_handled() {
                        Handled::Yes => Ok(Handled::Yes),
                        Handled::No {
                            message: MessageCx::Request(request, request_cx),
                            retry,
                        } => {
                            let untyped = request.to_untyped_message()?;
                            Ok(Handled::No {
                                message: MessageCx::Request(untyped, request_cx.erase_to_json()),
                                retry,
                            })
                        }
                        Handled::No {
                            message: MessageCx::Notification(notification),
                            retry,
                        } => {
                            let untyped = notification.to_untyped_message()?;
                            Ok(Handled::No {
                                message: MessageCx::Notification(untyped),
                                retry,
                            })
                        }
                    }
                }

                Err(message_cx) => Ok(Handled::No {
                    message: message_cx,
                    retry: false,
                }),
            },
        )
    }
}

/// Wraps a handler with an optional name for tracing/debugging.
pub struct NamedHandler<H> {
    name: Option<String>,
    handler: H,
}

impl<H: JrMessageHandler> NamedHandler<H> {
    /// Creates a new named handler
    pub fn new(name: Option<String>, handler: H) -> Self {
        Self { name, handler }
    }
}

impl<H: JrMessageHandler> JrMessageHandler for NamedHandler<H> {
    type Link = H::Link;

    fn describe_chain(&self) -> impl std::fmt::Debug {
        format!(
            "NamedHandler({:?}, {:?})",
            self.name,
            self.handler.describe_chain()
        )
    }

    async fn handle_message(
        &mut self,
        message: MessageCx,
        connection_cx: JrConnectionCx<H::Link>,
    ) -> Result<Handled<MessageCx>, crate::Error> {
        if let Some(name) = &self.name {
            crate::util::instrumented_with_connection_name(
                name.clone(),
                self.handler.handle_message(message, connection_cx),
            )
            .await
        } else {
            self.handler.handle_message(message, connection_cx).await
        }
    }
}

/// Chains two handlers together, trying the first handler and falling back to the second
pub struct ChainedHandler<H1, H2> {
    handler1: H1,
    handler2: H2,
}

impl<H1, H2> ChainedHandler<H1, H2>
where
    H1: JrMessageHandler,
    H2: JrMessageHandler<Link = H1::Link>,
{
    /// Creates a new chain handler
    pub fn new(handler1: H1, handler2: H2) -> Self {
        Self { handler1, handler2 }
    }
}

impl<H1, H2> JrMessageHandler for ChainedHandler<H1, H2>
where
    H1: JrMessageHandler,
    H2: JrMessageHandler<Link = H1::Link>,
{
    type Link = H1::Link;

    fn describe_chain(&self) -> impl std::fmt::Debug {
        format!(
            "{:?}, {:?}",
            self.handler1.describe_chain(),
            self.handler2.describe_chain()
        )
    }

    async fn handle_message(
        &mut self,
        message: MessageCx,
        connection_cx: JrConnectionCx<H1::Link>,
    ) -> Result<Handled<MessageCx>, crate::Error> {
        match self
            .handler1
            .handle_message(message, connection_cx.clone())
            .await?
        {
            Handled::Yes => Ok(Handled::Yes),
            Handled::No {
                message,
                retry: retry1,
            } => match self.handler2.handle_message(message, connection_cx).await? {
                Handled::Yes => Ok(Handled::Yes),
                Handled::No {
                    message,
                    retry: retry2,
                } => Ok(Handled::No {
                    message,
                    retry: retry1 | retry2,
                }),
            },
        }
    }
}

// =============================================================================
// Async Handler Types (Responder-based)
// =============================================================================
//
// These handlers use channels and responders to run async closures without
// blocking the message dispatch loop. This prevents deadlocks that can occur
// when an async handler awaits on something that requires the dispatch loop
// to make progress (e.g., sending a request and waiting for its response).

/// A request call sent through the channel to the responder.
pub struct RequestCall<Req: JrRequest, Link: JrLink> {
    pub(crate) request: Req,
    pub(crate) request_cx: JrRequestCx<Req::Response>,
    pub(crate) connection_cx: JrConnectionCx<Link>,
}

/// Sync handler that sends requests to a channel for async processing.
///
/// This handler parses incoming requests and sends them to a channel. The actual
/// async processing happens in the [`RequestHandlerResponder`].
pub struct RequestHandlerAsync<Link: JrLink, Peer: JrPeer, Req: JrRequest = UntypedMessage> {
    call_tx: UnboundedSender<RequestCall<Req, Link>>,
    phantom: PhantomData<fn(Link, Peer, Req)>,
}

impl<Link: JrLink, Peer: JrPeer, Req: JrRequest> RequestHandlerAsync<Link, Peer, Req> {
    /// Creates a new async request handler with the given channel sender.
    pub fn new(_peer: Peer, _link: Link, call_tx: UnboundedSender<RequestCall<Req, Link>>) -> Self {
        Self {
            call_tx,
            phantom: PhantomData,
        }
    }
}

impl<Link: JrLink, Peer: JrPeer, Req: JrRequest> JrMessageHandler
    for RequestHandlerAsync<Link, Peer, Req>
where
    Link: HasPeer<Peer>,
{
    type Link = Link;

    fn describe_chain(&self) -> impl std::fmt::Debug {
        format!("{}(async-responder)", std::any::type_name::<Req>())
    }

    async fn handle_message(
        &mut self,
        message_cx: MessageCx,
        connection_cx: JrConnectionCx<Link>,
    ) -> Result<Handled<MessageCx>, crate::Error> {
        let remote_style = Link::remote_style(Peer::default());
        remote_style.handle_incoming_message_sync(
            message_cx,
            connection_cx,
            |message_cx, connection_cx| match message_cx {
                MessageCx::Request(message, request_cx) => {
                    tracing::debug!(
                        request_type = std::any::type_name::<Req>(),
                        message = ?message,
                        "RequestHandlerAsync::handle_request"
                    );
                    match Req::parse_message(&message.method, &message.params) {
                        Some(Ok(request)) => {
                            tracing::trace!(
                                ?request,
                                "RequestHandlerAsync::handle_request: parse completed, sending to channel"
                            );
                            self.call_tx
                                .unbounded_send(RequestCall {
                                    request,
                                    request_cx: request_cx.cast(),
                                    connection_cx,
                                })
                                .map_err(|e| {
                                    crate::util::internal_error(format!(
                                        "failed to send request to handler channel: {}",
                                        e
                                    ))
                                })?;
                            Ok(Handled::Yes)
                        }
                        Some(Err(err)) => {
                            tracing::trace!(
                                ?err,
                                "RequestHandlerAsync::handle_request: parse errored"
                            );
                            Err(err)
                        }
                        None => {
                            tracing::trace!(
                                "RequestHandlerAsync::handle_request: parse failed"
                            );
                            Ok(Handled::No {
                                message: MessageCx::Request(message, request_cx),
                                retry: false,
                            })
                        }
                    }
                }

                MessageCx::Notification(..) => Ok(Handled::No {
                    message: message_cx,
                    retry: false,
                }),
            },
        )
    }
}

/// Responder that receives request calls and invokes the user's async closure.
pub struct RequestHandlerResponder<Link: JrLink, Req: JrRequest, F, ToFut> {
    pub(crate) func: F,
    pub(crate) call_rx: UnboundedReceiver<RequestCall<Req, Link>>,
    pub(crate) to_future_hack: ToFut,
}

impl<Link, Req, F, ToFut> JrResponder<Link> for RequestHandlerResponder<Link, Req, F, ToFut>
where
    Link: JrLink,
    Req: JrRequest,
    F: AsyncFnMut(
            Req,
            JrRequestCx<Req::Response>,
            JrConnectionCx<Link>,
        ) -> Result<(), crate::Error>
        + Send,
    ToFut: Fn(
            &mut F,
            Req,
            JrRequestCx<Req::Response>,
            JrConnectionCx<Link>,
        ) -> BoxFuture<'_, Result<(), crate::Error>>
        + Send
        + Sync,
{
    async fn run(self, _cx: JrConnectionCx<Link>) -> Result<(), crate::Error> {
        let RequestHandlerResponder {
            mut func,
            mut call_rx,
            to_future_hack,
        } = self;

        while let Some(RequestCall {
            request,
            request_cx,
            connection_cx,
        }) = call_rx.next().await
        {
            if let Err(err) = to_future_hack(&mut func, request, request_cx, connection_cx).await {
                tracing::error!(?err, "RequestHandlerResponder: handler returned error");
            }
        }
        Ok(())
    }
}

/// A notification call sent through the channel to the responder.
pub struct NotificationCall<Notif: JrNotification, Link: JrLink> {
    pub(crate) notification: Notif,
    pub(crate) connection_cx: JrConnectionCx<Link>,
}

/// Sync handler that sends notifications to a channel for async processing.
pub struct NotificationHandlerAsync<
    Link: JrLink,
    Peer: JrPeer,
    Notif: JrNotification = UntypedMessage,
> {
    call_tx: UnboundedSender<NotificationCall<Notif, Link>>,
    phantom: PhantomData<fn(Link, Peer, Notif)>,
}

impl<Link: JrLink, Peer: JrPeer, Notif: JrNotification>
    NotificationHandlerAsync<Link, Peer, Notif>
{
    /// Creates a new async notification handler with the given channel sender.
    pub fn new(
        _peer: Peer,
        _link: Link,
        call_tx: UnboundedSender<NotificationCall<Notif, Link>>,
    ) -> Self {
        Self {
            call_tx,
            phantom: PhantomData,
        }
    }
}

impl<Link: JrLink, Peer: JrPeer, Notif: JrNotification> JrMessageHandler
    for NotificationHandlerAsync<Link, Peer, Notif>
where
    Link: HasPeer<Peer>,
{
    type Link = Link;

    fn describe_chain(&self) -> impl std::fmt::Debug {
        format!("{}(async-responder)", std::any::type_name::<Notif>())
    }

    async fn handle_message(
        &mut self,
        message_cx: MessageCx,
        connection_cx: JrConnectionCx<Link>,
    ) -> Result<Handled<MessageCx>, crate::Error> {
        let remote_style = Link::remote_style(Peer::default());
        remote_style.handle_incoming_message_sync(
            message_cx,
            connection_cx,
            |message_cx, connection_cx| match message_cx {
                MessageCx::Notification(message) => {
                    tracing::debug!(
                        notification_type = std::any::type_name::<Notif>(),
                        message = ?message,
                        "NotificationHandlerAsync::handle_notification"
                    );
                    match Notif::parse_message(&message.method, &message.params) {
                        Some(Ok(notification)) => {
                            tracing::trace!(
                                ?notification,
                                "NotificationHandlerAsync::handle_notification: parse completed, sending to channel"
                            );
                            self.call_tx
                                .unbounded_send(NotificationCall {
                                    notification,
                                    connection_cx,
                                })
                                .map_err(|e| {
                                    crate::util::internal_error(format!(
                                        "failed to send notification to handler channel: {}",
                                        e
                                    ))
                                })?;
                            Ok(Handled::Yes)
                        }
                        Some(Err(err)) => {
                            tracing::trace!(
                                ?err,
                                "NotificationHandlerAsync::handle_notification: parse errored"
                            );
                            Err(err)
                        }
                        None => {
                            tracing::trace!(
                                "NotificationHandlerAsync::handle_notification: parse failed"
                            );
                            Ok(Handled::No {
                                message: MessageCx::Notification(message),
                                retry: false,
                            })
                        }
                    }
                }

                MessageCx::Request(..) => Ok(Handled::No {
                    message: message_cx,
                    retry: false,
                }),
            },
        )
    }
}

/// Responder that receives notification calls and invokes the user's async closure.
pub struct NotificationHandlerResponder<Link: JrLink, Notif: JrNotification, F, ToFut> {
    pub(crate) func: F,
    pub(crate) call_rx: UnboundedReceiver<NotificationCall<Notif, Link>>,
    pub(crate) to_future_hack: ToFut,
}

impl<Link, Notif, F, ToFut> JrResponder<Link>
    for NotificationHandlerResponder<Link, Notif, F, ToFut>
where
    Link: JrLink,
    Notif: JrNotification,
    F: AsyncFnMut(Notif, JrConnectionCx<Link>) -> Result<(), crate::Error> + Send,
    ToFut: Fn(&mut F, Notif, JrConnectionCx<Link>) -> BoxFuture<'_, Result<(), crate::Error>>
        + Send
        + Sync,
{
    async fn run(self, _cx: JrConnectionCx<Link>) -> Result<(), crate::Error> {
        let NotificationHandlerResponder {
            mut func,
            mut call_rx,
            to_future_hack,
        } = self;

        while let Some(NotificationCall {
            notification,
            connection_cx,
        }) = call_rx.next().await
        {
            if let Err(err) = to_future_hack(&mut func, notification, connection_cx).await {
                tracing::error!(?err, "NotificationHandlerResponder: handler returned error");
            }
        }
        Ok(())
    }
}

/// A message call sent through the channel to the responder.
pub struct MessageCall<Req: JrRequest, Notif: JrNotification, Link: JrLink> {
    pub(crate) message: MessageCx<Req, Notif>,
    pub(crate) connection_cx: JrConnectionCx<Link>,
}

/// Sync handler that sends messages to a channel for async processing.
pub struct MessageHandlerAsync<
    Link: JrLink,
    Peer: JrPeer,
    Req: JrRequest = UntypedMessage,
    Notif: JrNotification = UntypedMessage,
> {
    call_tx: UnboundedSender<MessageCall<Req, Notif, Link>>,
    phantom: PhantomData<fn(Link, Peer, Req, Notif)>,
}

impl<Link: JrLink, Peer: JrPeer, Req: JrRequest, Notif: JrNotification>
    MessageHandlerAsync<Link, Peer, Req, Notif>
{
    /// Creates a new async message handler with the given channel sender.
    pub fn new(
        _peer: Peer,
        _link: Link,
        call_tx: UnboundedSender<MessageCall<Req, Notif, Link>>,
    ) -> Self {
        Self {
            call_tx,
            phantom: PhantomData,
        }
    }
}

impl<Link: JrLink, Peer: JrPeer, Req: JrRequest, Notif: JrNotification> JrMessageHandler
    for MessageHandlerAsync<Link, Peer, Req, Notif>
where
    Link: HasPeer<Peer>,
{
    type Link = Link;

    fn describe_chain(&self) -> impl std::fmt::Debug {
        format!(
            "({}, {})(async-responder)",
            std::any::type_name::<Req>(),
            std::any::type_name::<Notif>()
        )
    }

    async fn handle_message(
        &mut self,
        message_cx: MessageCx,
        connection_cx: JrConnectionCx<Link>,
    ) -> Result<Handled<MessageCx>, crate::Error> {
        let remote_style = Link::remote_style(Peer::default());
        remote_style.handle_incoming_message_sync(
            message_cx,
            connection_cx,
            |message_cx, connection_cx| match message_cx.into_typed_message_cx::<Req, Notif>()? {
                Ok(typed_message_cx) => {
                    tracing::debug!("MessageHandlerAsync::handle_message: sending to channel");
                    self.call_tx
                        .unbounded_send(MessageCall {
                            message: typed_message_cx,
                            connection_cx,
                        })
                        .map_err(|e| {
                            crate::util::internal_error(format!(
                                "failed to send message to handler channel: {}",
                                e
                            ))
                        })?;
                    Ok(Handled::Yes)
                }

                Err(message_cx) => Ok(Handled::No {
                    message: message_cx,
                    retry: false,
                }),
            },
        )
    }
}

/// Responder that receives message calls and invokes the user's async closure.
pub struct MessageHandlerResponder<Link: JrLink, Req: JrRequest, Notif: JrNotification, F, ToFut> {
    pub(crate) func: F,
    pub(crate) call_rx: UnboundedReceiver<MessageCall<Req, Notif, Link>>,
    pub(crate) to_future_hack: ToFut,
}

impl<Link, Req, Notif, F, ToFut> JrResponder<Link>
    for MessageHandlerResponder<Link, Req, Notif, F, ToFut>
where
    Link: JrLink,
    Req: JrRequest,
    Notif: JrNotification,
    F: AsyncFnMut(MessageCx<Req, Notif>, JrConnectionCx<Link>) -> Result<(), crate::Error> + Send,
    ToFut: Fn(
            &mut F,
            MessageCx<Req, Notif>,
            JrConnectionCx<Link>,
        ) -> BoxFuture<'_, Result<(), crate::Error>>
        + Send
        + Sync,
{
    async fn run(self, _cx: JrConnectionCx<Link>) -> Result<(), crate::Error> {
        let MessageHandlerResponder {
            mut func,
            mut call_rx,
            to_future_hack,
        } = self;

        while let Some(MessageCall {
            message,
            connection_cx,
        }) = call_rx.next().await
        {
            if let Err(err) = to_future_hack(&mut func, message, connection_cx).await {
                tracing::error!(?err, "MessageHandlerResponder: handler returned error");
            }
        }
        Ok(())
    }
}
