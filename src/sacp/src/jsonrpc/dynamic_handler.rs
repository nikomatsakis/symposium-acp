use futures::future::BoxFuture;
use uuid::Uuid;

use crate::HasCounterpart;
use crate::role::JrRole;
use crate::{Handled, MessageAndCx, jsonrpc::JrMessageHandlerSend};

/// Internal dyn-safe wrapper around `JrMessageHandlerSend`
pub(crate) trait DynamicHandler<Local: JrRole, Counterpart: JrRole>: Send {
    fn dyn_handle_message(
        &mut self,
        message: MessageAndCx<Local, Counterpart>,
    ) -> BoxFuture<'_, Result<Handled<MessageAndCx<Local, Counterpart>>, crate::Error>>;

    fn dyn_describe_chain(&self) -> String;
}

impl<H: JrMessageHandlerSend> DynamicHandler<H::Local, H::Remote> for H
where
    H::Local: HasCounterpart<H::Remote>,
{
    fn dyn_handle_message(
        &mut self,
        message: MessageAndCx<H::Local, H::Remote>,
    ) -> BoxFuture<'_, Result<Handled<MessageAndCx<H::Local, H::Remote>>, crate::Error>> {
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
