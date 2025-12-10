use crate::jsonrpc::{Handled, IntoHandled, JrMessageHandler};
use crate::role::JrRole;
use crate::{
    HasCounterpart, HasRemoteRole, JrConnectionCx, JrNotification, JrRequest, MessageCx,
    UntypedMessage,
};
// Types re-exported from crate root
use super::JrRequestCx;
use std::marker::PhantomData;
use std::ops::AsyncFnMut;

/// Null handler that accepts no messages.
pub struct NullHandler<Local: JrRole, Remote: JrRole>
where
    Local: HasRemoteRole<Remote>,
{
    local: Local,
    remote: Remote,
}

impl<Local: JrRole, Remote: JrRole> NullHandler<Local, Remote>
where
    Local: HasRemoteRole<Remote>,
{
    /// Creates a new null handler.
    pub fn new(local: Local, remote: Remote) -> Self {
        Self { local, remote }
    }

    /// Returns the local role.
    pub fn local(&self) -> Local {
        self.local.clone()
    }

    /// Returns the remote role.
    pub fn remote(&self) -> Remote {
        self.remote.clone()
    }
}

impl<Local: JrRole, Remote: JrRole, Counterpart: JrRole> JrMessageHandler
    for NullHandler<Local, Remote>
where
    Local: JrRole + HasRemoteRole<Remote, Counterpart = Counterpart> + HasCounterpart<Counterpart>,
{
    type Local = Local;
    type Remote = Remote;
    type Counterpart = Counterpart;

    fn describe_chain(&self) -> impl std::fmt::Debug {
        "(null)"
    }

    async fn handle_message(
        &mut self,
        message: MessageCx,
        cx: JrConnectionCx<Local, Counterpart>,
    ) -> Result<Handled<MessageCx<Local, Counterpart>>, crate::Error> {
        Ok(Handled::No(message))
    }
}

/// Handler for typed request messages
pub struct RequestHandler<Local: JrRole, Remote: JrRole, Req: JrRequest = UntypedMessage, F = ()>
where
    Local: HasRemoteRole<Remote>,
{
    handler: F,
    local: Local,
    remote: Remote,
    phantom: PhantomData<fn(Req)>,
}

impl<Local: JrRole, Remote: JrRole, Req: JrRequest, F> RequestHandler<Local, Remote, Req, F>
where
    Local: HasRemoteRole<Remote>,
{
    /// Creates a new request handler
    pub fn new(local: Local, remote: Remote, handler: F) -> Self {
        Self {
            handler,
            local,
            remote,
            phantom: PhantomData,
        }
    }

    /// Returns the local role.
    pub fn local(&self) -> Local {
        self.local.clone()
    }

    /// Returns the remote role.
    pub fn remote(&self) -> Remote {
        self.remote.clone()
    }
}

impl<Local: JrRole, Counterpart: JrRole, Remote: JrRole, Req, F, T> JrMessageHandler
    for RequestHandler<Local, Remote, Req, F>
where
    Local: JrRole + HasRemoteRole<Remote, Counterpart = Counterpart> + HasCounterpart<Counterpart>,
    Req: JrRequest,
    F: AsyncFnMut(
        Req,
        JrRequestCx<Req::Response>,
        JrConnectionCx<Local, Counterpart>,
    ) -> Result<T, crate::Error>,
    T: crate::IntoHandled<(Req, JrRequestCx<Req::Response>)>,
{
    type Local = Local;
    type Remote = Remote;
    type Counterpart = Counterpart;

    fn describe_chain(&self) -> impl std::fmt::Debug {
        std::any::type_name::<Req>()
    }

    async fn handle_message(
        &mut self,
        message_cx: MessageCx,
    ) -> Result<Handled<MessageCx>, crate::Error> {
        match message_cx {
            MessageCx::Request(message, request_cx) => {
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
                                Ok(Handled::No(MessageCx::Request(
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
                        Ok(Handled::No(MessageCx::Request(message, request_cx)))
                    }
                }
            }

            MessageCx::Notification(..) => Ok(Handled::No(message_cx)),
        }
    }
}

/// Handler for typed notification messages
pub struct NotificationHandler<
    Local: JrRole,
    Remote: JrRole,
    Notif: JrNotification = UntypedMessage,
    F = (),
> {
    handler: F,
    local: Local,
    remote: Remote,
    phantom: PhantomData<fn(Notif)>,
}

impl<Local: JrRole, Remote: JrRole, Notif: JrNotification, F>
    NotificationHandler<Local, Remote, Notif, F>
{
    /// Creates a new notification handler
    pub fn new(local: Local, remote: Remote, handler: F) -> Self {
        Self {
            handler,
            local,
            remote,
            phantom: PhantomData,
        }
    }

    /// Returns the local role.
    pub fn local(&self) -> Local {
        self.local.clone()
    }

    /// Returns the remote role.
    pub fn remote(&self) -> Remote {
        self.remote.clone()
    }
}

impl<Local, Remote, Counterpart: JrRole, Notif, F, T> JrMessageHandler
    for NotificationHandler<Local, Remote, Notif, F>
where
    Local: JrRole + HasRemoteRole<Remote, Counterpart = Counterpart> + HasCounterpart<Counterpart>,
    Remote: JrRole,
    Notif: JrNotification,
    F: AsyncFnMut(Notif, JrConnectionCx<Local, Counterpart>) -> Result<T, crate::Error>,
    T: crate::IntoHandled<(Notif, JrConnectionCx<Local, Counterpart>)>,
{
    type Local = Local;
    type Remote = Remote;
    type Counterpart = Counterpart;

    fn describe_chain(&self) -> impl std::fmt::Debug {
        std::any::type_name::<Notif>()
    }

    async fn handle_message(
        &mut self,
        message_cx: MessageCx<Local, Counterpart>,
    ) -> Result<Handled<MessageCx<Local, Counterpart>>, crate::Error> {
        match message_cx {
            MessageCx::Notification(message, cx) => {
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
                                Ok(Handled::No(MessageCx::Notification(untyped, cx)))
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
                        Ok(Handled::No(MessageCx::Notification(message, cx)))
                    }
                }
            }

            MessageCx::Request(..) => Ok(Handled::No(message_cx)),
        }
    }
}

/// Handler that handles both requests and notifications of specific types.
pub struct MessageHandler<
    Local: JrRole,
    Remote: JrRole,
    Req: JrRequest = UntypedMessage,
    Notif: JrNotification = UntypedMessage,
    F = (),
> {
    handler: F,
    local: Local,
    remote: Remote,
    phantom: PhantomData<fn(Req, Notif)>,
}

impl<
    Local: JrRole,
    Remote: JrRole,
    Counterpart: JrRole,
    Req: JrRequest,
    Notif: JrNotification,
    F,
    T,
> MessageHandler<Local, Remote, Req, Notif, F>
where
    Local: JrRole + HasRemoteRole<Remote, Counterpart = Counterpart> + HasCounterpart<Counterpart>,
    F: AsyncFnMut(MessageCx<Local, Counterpart, Req, Notif>) -> Result<T, crate::Error>,
    T: IntoHandled<MessageCx<Local, Counterpart, Req, Notif>>,
{
    /// Creates a new message handler
    pub fn new(local: Local, remote: Remote, handler: F) -> Self {
        Self {
            handler,
            local,
            remote,
            phantom: PhantomData,
        }
    }

    /// Returns the local role.
    pub fn local(&self) -> Local {
        self.local.clone()
    }

    /// Returns the remote role.
    pub fn remote(&self) -> Remote {
        self.remote.clone()
    }
}

impl<
    Local: JrRole,
    Remote: JrRole,
    Counterpart: JrRole,
    Req: JrRequest,
    Notif: JrNotification,
    F,
    T,
> JrMessageHandler for MessageHandler<Local, Remote, Req, Notif, F>
where
    Local: JrRole + HasRemoteRole<Remote, Counterpart = Counterpart> + HasCounterpart<Counterpart>,
    F: AsyncFnMut(MessageCx<Local, Counterpart, Req, Notif>) -> Result<T, crate::Error>,
    T: IntoHandled<MessageCx<Local, Counterpart, Req, Notif>>,
{
    type Local = Local;
    type Remote = Remote;
    type Counterpart = Counterpart;

    fn describe_chain(&self) -> impl std::fmt::Debug {
        format!(
            "({}, {})",
            std::any::type_name::<Req>(),
            std::any::type_name::<Notif>()
        )
    }

    async fn handle_message(
        &mut self,
        message_cx: MessageCx<Local, Counterpart>,
    ) -> Result<Handled<MessageCx<Local, Counterpart>>, crate::Error> {
        match message_cx {
            MessageCx::Request(message, request_cx) => {
                tracing::debug!(
                    request_type = std::any::type_name::<Req>(),
                    message = ?message,
                    "MessageHandler::handle_request"
                );
                match Req::parse_request(&message.method, &message.params) {
                    Some(Ok(req)) => {
                        tracing::trace!(?req, "MessageHandler::handle_request: parse completed");
                        let typed_message = MessageCx::Request(req, request_cx.cast());
                        let result = (self.handler)(typed_message).await?;
                        match result.into_handled() {
                            Handled::Yes => Ok(Handled::Yes),
                            Handled::No(MessageCx::Request(request, request_cx)) => {
                                let untyped = request.to_untyped_message()?;
                                Ok(Handled::No(MessageCx::Request(
                                    untyped,
                                    request_cx.erase_to_json(),
                                )))
                            }
                            Handled::No(MessageCx::Notification(..)) => {
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
                        Ok(Handled::No(MessageCx::Request(message, request_cx)))
                    }
                }
            }

            MessageCx::Notification(message, cx) => {
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
                        let typed_message = MessageCx::Notification(notif, cx);
                        let result = (self.handler)(typed_message).await?;
                        match result.into_handled() {
                            Handled::Yes => Ok(Handled::Yes),
                            Handled::No(MessageCx::Notification(notification, cx)) => {
                                let untyped = notification.to_untyped_message()?;
                                Ok(Handled::No(MessageCx::Notification(untyped, cx)))
                            }
                            Handled::No(MessageCx::Request(..)) => {
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
                        Ok(Handled::No(MessageCx::Notification(message, cx)))
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

impl<H: JrMessageHandler> NamedHandler<H> {
    /// Creates a new named handler
    pub fn new(name: Option<String>, handler: H) -> Self {
        Self { name, handler }
    }
}

impl<H: JrMessageHandler> JrMessageHandler for NamedHandler<H> {
    type Local = H::Local;
    type Remote = H::Remote;
    type Counterpart = H::Counterpart;

    fn describe_chain(&self) -> impl std::fmt::Debug {
        format!(
            "NamedHandler({:?}, {:?})",
            self.name,
            self.handler.describe_chain()
        )
    }

    async fn handle_message(
        &mut self,
        message: MessageCx<H::Local, H::Counterpart>,
    ) -> Result<Handled<MessageCx<H::Local, H::Counterpart>>, crate::Error> {
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

impl<H1, H2> ChainedHandler<H1, H2>
where
    H1: JrMessageHandler,
    H2: JrMessageHandler<Local = H1::Local, Remote = H1::Remote>,
{
    /// Creates a new chain handler
    pub fn new(handler1: H1, handler2: H2) -> Self {
        Self { handler1, handler2 }
    }
}

impl<H1, H2> JrMessageHandler for ChainedHandler<H1, H2>
where
    H1: JrMessageHandler,
    H2: JrMessageHandler<Local = H1::Local, Remote = H1::Remote>,
{
    type Local = H1::Local;
    type Remote = H1::Remote;
    type Counterpart = H1::Counterpart;

    fn describe_chain(&self) -> impl std::fmt::Debug {
        format!(
            "{:?}, {:?}",
            self.handler1.describe_chain(),
            self.handler2.describe_chain()
        )
    }

    async fn handle_message(
        &mut self,
        message: MessageCx<H1::Local, H1::Counterpart>,
    ) -> Result<Handled<MessageCx<H1::Local, H1::Counterpart>>, crate::Error> {
        match self.handler1.handle_message(message).await? {
            Handled::Yes => Ok(Handled::Yes),
            Handled::No(message) => self.handler2.handle_message(message).await,
        }
    }
}

/// Adapts messages from one role to another before dispatching to the inner handler.
///
/// `RxRole` is the role of the handler chain (the receiver).
/// `TxRole` is the role of the sender of the messages.
///
/// The `RxRole: ReceivesFromRole<TxRole>` bound provides the logic for
/// transforming incoming messages (e.g., unwrapping `SuccessorRequest` envelopes).
pub struct AdaptRole<H: JrMessageHandler> {
    handler: H,
}

impl<H: JrMessageHandler> AdaptRole<H> {
    /// Creates a new role adapter.
    pub fn new(handler: H) -> Self {
        Self { handler }
    }
}

impl<H: JrMessageHandler> JrMessageHandler for AdaptRole<H> {
    type Local = H::Local;
    type Remote = H::Counterpart;
    type Counterpart = H::Counterpart;

    fn describe_chain(&self) -> impl std::fmt::Debug {
        format!("AdaptRole({:?})", self.handler.describe_chain())
    }

    async fn handle_message(
        &mut self,
        message: MessageCx<H::Local, H::Counterpart>,
    ) -> Result<Handled<MessageCx<H::Local, H::Counterpart>>, crate::Error> {
        <H::Local>::remote_style(H::Remote::default())
            .handle_incoming_message(message, &mut self.handler)
            .await
    }
}
