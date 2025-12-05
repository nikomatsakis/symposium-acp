use futures::future::BoxFuture;
use uuid::Uuid;

use crate::{Handled, MessageAndCx, jsonrpc::JrMessageHandlerSend};

/// Internal dyn-safe wrapper around `JrMessageHandlerSend`
pub(crate) trait DynamicHandler: Send {
    fn dyn_handle_message(
        &mut self,
        message: MessageAndCx,
    ) -> BoxFuture<'_, Result<Handled<MessageAndCx>, crate::Error>>;

    fn dyn_describe_chain(&self) -> String;
}

impl<H: JrMessageHandlerSend> DynamicHandler for H {
    fn dyn_handle_message(
        &mut self,
        message: MessageAndCx,
    ) -> BoxFuture<'_, Result<Handled<MessageAndCx>, crate::Error>> {
        Box::pin(JrMessageHandlerSend::handle_message(self, message))
    }

    fn dyn_describe_chain(&self) -> String {
        format!("{:?}", H::describe_chain(self))
    }
}

/// Messages used to add/remove dynamic handlers
pub(crate) enum DynamicHandlerMessage {
    AddDynamicHandler(Uuid, Box<dyn DynamicHandler>),
    RemoveDynamicHandler(Uuid),
}
