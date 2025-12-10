use futures::future::BoxFuture;
use uuid::Uuid;

use crate::role::JrRole;
use crate::{Handled, MessageCx, jsonrpc::JrMessageHandlerSend};
use crate::{HasCounterpart, JrConnection, JrConnectionCx};

/// Internal dyn-safe wrapper around `JrMessageHandlerSend`
pub(crate) trait DynamicHandler<Local: JrRole, Counterpart: JrRole>: Send {
    fn dyn_handle_message(
        &mut self,
        message: MessageCx,
        cx: JrConnectionCx<Local, Counterpart>,
    ) -> BoxFuture<'_, Result<Handled<MessageCx>, crate::Error>>;

    fn dyn_describe_chain(&self) -> String;
}

impl<H: JrMessageHandlerSend> DynamicHandler<H::Local, H::Remote> for H
where
    H::Local: HasCounterpart<H::Remote>,
{
    fn dyn_handle_message(
        &mut self,
        message: MessageCx<H::Local, H::Remote>,
    ) -> BoxFuture<'_, Result<Handled<MessageCx<H::Local, H::Remote>>, crate::Error>> {
        Box::pin(JrMessageHandlerSend::handle_message(self, message))
    }

    fn dyn_describe_chain(&self) -> String {
        format!("{:?}", H::describe_chain(self))
    }
}

/// Messages used to add/remove dynamic handlers
pub(crate) enum DynamicHandlerMessage<Local: JrRole, Remote: JrRole> {
    AddDynamicHandler(Uuid, Box<dyn DynamicHandler<Local, Remote>>),
    RemoveDynamicHandler(Uuid),
}
