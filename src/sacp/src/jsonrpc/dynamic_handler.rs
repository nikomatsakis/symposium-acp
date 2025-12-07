use futures::future::BoxFuture;
use uuid::Uuid;

use crate::role::{DefaultRole, JrRole};
use crate::{Handled, MessageAndCx, UntypedMessage, jsonrpc::JrMessageHandlerSend};

/// Internal dyn-safe wrapper around `JrMessageHandlerSend`
pub(crate) trait DynamicHandler<R: JrRole = DefaultRole>: Send {
    fn dyn_handle_message(
        &mut self,
        message: MessageAndCx<R, UntypedMessage, UntypedMessage>,
    ) -> BoxFuture<'_, Result<Handled<MessageAndCx<R, UntypedMessage, UntypedMessage>>, crate::Error>>;

    fn dyn_describe_chain(&self) -> String;
}

impl<H: JrMessageHandlerSend<R>, R: JrRole> DynamicHandler<R> for H {
    fn dyn_handle_message(
        &mut self,
        message: MessageAndCx<R, UntypedMessage, UntypedMessage>,
    ) -> BoxFuture<'_, Result<Handled<MessageAndCx<R, UntypedMessage, UntypedMessage>>, crate::Error>>
    {
        Box::pin(JrMessageHandlerSend::handle_message(self, message))
    }

    fn dyn_describe_chain(&self) -> String {
        format!("{:?}", H::describe_chain(self))
    }
}

/// Messages used to add/remove dynamic handlers
pub(crate) enum DynamicHandlerMessage<R: JrRole = DefaultRole> {
    AddDynamicHandler(Uuid, Box<dyn DynamicHandler<R>>),
    RemoveDynamicHandler(Uuid),
}
