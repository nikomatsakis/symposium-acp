use crate::jsonrpc::{Handled, IntoHandled, JrMessageHandler};
use crate::role::{JrRole, UntypedRole};
use crate::{JrConnectionCx, JrNotification, JrRequest, MessageAndCx, UntypedMessage};
// Types re-exported from crate root
use super::JrRequestCx;
use std::marker::PhantomData;
use std::ops::AsyncFnMut;

/// Null handler that accepts no messages.
#[derive(Default)]
pub struct NullHandler {
    _private: (),
}

impl<R: JrRole> JrMessageHandler<R> for NullHandler {
    fn describe_chain(&self) -> impl std::fmt::Debug {
        "(null)"
    }

    async fn handle_message(
        &mut self,
        message: MessageAndCx<R, UntypedMessage, UntypedMessage>,
    ) -> Result<Handled<MessageAndCx<R, UntypedMessage, UntypedMessage>>, crate::Error> {
        Ok(Handled::No(message))
    }
}

/// Handler for typed request messages
pub struct RequestHandler<R: JrRole = UntypedRole, Req: JrRequest = UntypedMessage, F = ()> {
    handler: F,
    phantom: PhantomData<fn(R, Req)>,
}

impl<R: JrRole, Req: JrRequest, F> RequestHandler<R, Req, F> {
    /// Creates a new request handler
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            phantom: PhantomData,
        }
    }
}

impl<R, Req, F, T> JrMessageHandler<R> for RequestHandler<R, Req, F>
where
    R: JrRole,
    Req: JrRequest,
    F: AsyncFnMut(Req, JrRequestCx<R, Req::Response>) -> Result<T, crate::Error>,
    T: crate::IntoHandled<(Req, JrRequestCx<R, Req::Response>)>,
{
    fn describe_chain(&self) -> impl std::fmt::Debug {
        std::any::type_name::<Req>()
    }

    async fn handle_message(
        &mut self,
        message_cx: MessageAndCx<R, UntypedMessage, UntypedMessage>,
    ) -> Result<Handled<MessageAndCx<R, UntypedMessage, UntypedMessage>>, crate::Error> {
        match message_cx {
            MessageAndCx::Request(message, request_cx) => {
                tracing::debug!(
                    request_type = std::any::type_name::<Req>(),
                    message = ?message,
                    "RequestHandler::handle_request"
                );
                match Req::parse_request(&message.method, &message.params) {
                    Some(Ok(req)) => {
                        tracing::trace!(?req, "RequestHandler::handle_request: parse completed");
                        let typed_request_cx = request_cx.cast();
                        let result = (self.handler)(req, typed_request_cx).await?;
                        match result.into_handled() {
                            Handled::Yes => Ok(Handled::Yes),
                            Handled::No((request, request_cx)) => {
                                // Handler returned the request back, convert to untyped
                                let untyped = request.to_untyped_message()?;
                                Ok(Handled::No(MessageAndCx::Request(
                                    untyped,
                                    request_cx.erase_to_json(),
                                )))
                            }
                        }
                    }
                    Some(Err(err)) => {
                        tracing::trace!(?err, "RequestHandler::handle_request: parse errored");
                        Err(err)
                    }
                    None => {
                        tracing::trace!("RequestHandler::handle_request: parse failed");
                        Ok(Handled::No(MessageAndCx::Request(message, request_cx)))
                    }
                }
            }

            MessageAndCx::Notification(..) => Ok(Handled::No(message_cx)),
        }
    }
}

/// Handler for typed notification messages
pub struct NotificationHandler<
    R: JrRole = UntypedRole,
    Notif: JrNotification = UntypedMessage,
    F = (),
> {
    handler: F,
    phantom: PhantomData<fn(R, Notif)>,
}

impl<R: JrRole, Notif: JrNotification, F> NotificationHandler<R, Notif, F> {
    /// Creates a new notification handler
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            phantom: PhantomData,
        }
    }
}

impl<R, Notif, F, T> JrMessageHandler<R> for NotificationHandler<R, Notif, F>
where
    R: JrRole,
    Notif: JrNotification,
    F: AsyncFnMut(Notif, JrConnectionCx<R>) -> Result<T, crate::Error>,
    T: crate::IntoHandled<(Notif, JrConnectionCx<R>)>,
{
    fn describe_chain(&self) -> impl std::fmt::Debug {
        std::any::type_name::<Notif>()
    }

    async fn handle_message(
        &mut self,
        message_cx: MessageAndCx<R, UntypedMessage, UntypedMessage>,
    ) -> Result<Handled<MessageAndCx<R, UntypedMessage, UntypedMessage>>, crate::Error> {
        match message_cx {
            MessageAndCx::Notification(message, cx) => {
                tracing::debug!(
                    request_type = std::any::type_name::<Notif>(),
                    message = ?message,
                    "NotificationHandler::handle_message"
                );
                match Notif::parse_notification(&message.method, &message.params) {
                    Some(Ok(notif)) => {
                        tracing::trace!(
                            ?notif,
                            "NotificationHandler::handle_notification: parse completed"
                        );
                        let result = (self.handler)(notif, cx.clone()).await?;
                        match result.into_handled() {
                            Handled::Yes => Ok(Handled::Yes),
                            Handled::No((notification, cx)) => {
                                // Handler returned the notification back, convert to untyped
                                let untyped = notification.to_untyped_message()?;
                                Ok(Handled::No(MessageAndCx::Notification(untyped, cx)))
                            }
                        }
                    }
                    Some(Err(err)) => {
                        tracing::trace!(
                            ?err,
                            "NotificationHandler::handle_notification: parse errored"
                        );
                        Err(err)
                    }
                    None => {
                        tracing::trace!("NotificationHandler::handle_notification: parse failed");
                        Ok(Handled::No(MessageAndCx::Notification(message, cx)))
                    }
                }
            }

            MessageAndCx::Request(..) => Ok(Handled::No(message_cx)),
        }
    }
}

/// Handler that handles both requests and notifications of specific types.
pub struct MessageHandler<
    R: JrRole = UntypedRole,
    Req: JrRequest = UntypedMessage,
    Notif: JrNotification = UntypedMessage,
    F = (),
> {
    handler: F,
    phantom: PhantomData<fn(R, Req, Notif)>,
}

impl<R: JrRole, Req: JrRequest, Notif: JrNotification, F, T> MessageHandler<R, Req, Notif, F>
where
    F: AsyncFnMut(MessageAndCx<R, Req, Notif>) -> Result<T, crate::Error>,
    T: IntoHandled<MessageAndCx<R, Req, Notif>>,
{
    /// Creates a new message handler
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            phantom: PhantomData,
        }
    }
}

impl<R, Req, Notif, F, T> JrMessageHandler<R> for MessageHandler<R, Req, Notif, F>
where
    R: JrRole,
    Req: JrRequest,
    Notif: JrNotification,
    F: AsyncFnMut(MessageAndCx<R, Req, Notif>) -> Result<T, crate::Error>,
    T: IntoHandled<MessageAndCx<R, Req, Notif>>,
{
    fn describe_chain(&self) -> impl std::fmt::Debug {
        format!(
            "({}, {})",
            std::any::type_name::<Req>(),
            std::any::type_name::<Notif>()
        )
    }

    async fn handle_message(
        &mut self,
        message_cx: MessageAndCx<R, UntypedMessage, UntypedMessage>,
    ) -> Result<Handled<MessageAndCx<R, UntypedMessage, UntypedMessage>>, crate::Error> {
        match message_cx {
            MessageAndCx::Request(message, request_cx) => {
                tracing::debug!(
                    request_type = std::any::type_name::<Req>(),
                    message = ?message,
                    "MessageHandler::handle_request"
                );
                match Req::parse_request(&message.method, &message.params) {
                    Some(Ok(req)) => {
                        tracing::trace!(?req, "MessageHandler::handle_request: parse completed");
                        let typed_message = MessageAndCx::Request(req, request_cx.cast());
                        let result = (self.handler)(typed_message).await?;
                        match result.into_handled() {
                            Handled::Yes => Ok(Handled::Yes),
                            Handled::No(MessageAndCx::Request(request, request_cx)) => {
                                let untyped = request.to_untyped_message()?;
                                Ok(Handled::No(MessageAndCx::Request(
                                    untyped,
                                    request_cx.erase_to_json(),
                                )))
                            }
                            Handled::No(MessageAndCx::Notification(..)) => {
                                unreachable!("Request handler returned notification")
                            }
                        }
                    }
                    Some(Err(err)) => {
                        tracing::trace!(?err, "MessageHandler::handle_request: parse errored");
                        Err(err)
                    }
                    None => {
                        tracing::trace!("MessageHandler::handle_request: parse failed");
                        Ok(Handled::No(MessageAndCx::Request(message, request_cx)))
                    }
                }
            }

            MessageAndCx::Notification(message, cx) => {
                tracing::debug!(
                    notification_type = std::any::type_name::<Notif>(),
                    message = ?message,
                    "MessageHandler::handle_notification"
                );
                match Notif::parse_notification(&message.method, &message.params) {
                    Some(Ok(notif)) => {
                        tracing::trace!(
                            ?notif,
                            "MessageHandler::handle_notification: parse completed"
                        );
                        let typed_message = MessageAndCx::Notification(notif, cx);
                        let result = (self.handler)(typed_message).await?;
                        match result.into_handled() {
                            Handled::Yes => Ok(Handled::Yes),
                            Handled::No(MessageAndCx::Notification(notification, cx)) => {
                                let untyped = notification.to_untyped_message()?;
                                Ok(Handled::No(MessageAndCx::Notification(untyped, cx)))
                            }
                            Handled::No(MessageAndCx::Request(..)) => {
                                unreachable!("Notification handler returned request")
                            }
                        }
                    }
                    Some(Err(err)) => {
                        tracing::trace!(?err, "MessageHandler::handle_notification: parse errored");
                        Err(err)
                    }
                    None => {
                        tracing::trace!("MessageHandler::handle_notification: parse failed");
                        Ok(Handled::No(MessageAndCx::Notification(message, cx)))
                    }
                }
            }
        }
    }
}

/// Chains two handlers together, trying the first handler and falling back to the second
pub struct NamedHandler<H> {
    name: Option<String>,
    handler: H,
}

impl<H> NamedHandler<H> {
    /// Creates a new named handler
    pub fn new(name: Option<String>, handler: H) -> Self {
        Self { name, handler }
    }
}

impl<H, R> JrMessageHandler<R> for NamedHandler<H>
where
    R: JrRole,
    H: JrMessageHandler<R>,
{
    fn describe_chain(&self) -> impl std::fmt::Debug {
        format!(
            "NamedHandler({:?}, {:?})",
            self.name,
            self.handler.describe_chain()
        )
    }

    async fn handle_message(
        &mut self,
        message: MessageAndCx<R, UntypedMessage, UntypedMessage>,
    ) -> Result<Handled<MessageAndCx<R, UntypedMessage, UntypedMessage>>, crate::Error> {
        if let Some(name) = &self.name {
            crate::util::instrumented_with_connection_name(
                name.clone(),
                self.handler.handle_message(message),
            )
            .await
        } else {
            self.handler.handle_message(message).await
        }
    }
}

/// Chains two handlers together, trying the first handler and falling back to the second
pub struct ChainedHandler<H1, H2> {
    handler1: H1,
    handler2: H2,
}

impl<H1, H2> ChainedHandler<H1, H2> {
    /// Creates a new chain handler
    pub fn new(handler1: H1, handler2: H2) -> Self {
        Self { handler1, handler2 }
    }
}

impl<H1, H2, R> JrMessageHandler<R> for ChainedHandler<H1, H2>
where
    R: JrRole,
    H1: JrMessageHandler<R>,
    H2: JrMessageHandler<R>,
{
    fn describe_chain(&self) -> impl std::fmt::Debug {
        format!(
            "{:?}, {:?}",
            self.handler1.describe_chain(),
            self.handler2.describe_chain()
        )
    }

    async fn handle_message(
        &mut self,
        message: MessageAndCx<R, UntypedMessage, UntypedMessage>,
    ) -> Result<Handled<MessageAndCx<R, UntypedMessage, UntypedMessage>>, crate::Error> {
        match self.handler1.handle_message(message).await? {
            Handled::Yes => Ok(Handled::Yes),
            Handled::No(message) => self.handler2.handle_message(message).await,
        }
    }
}
