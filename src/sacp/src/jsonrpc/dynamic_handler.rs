use futures::future::BoxFuture;
use uuid::Uuid;

use crate::role::JrRole;
use crate::{Handled, MessageAndCx, UntypedMessage, jsonrpc::JrMessageHandlerSend};

/// Internal dyn-safe wrapper around `JrMessageHandlerSend`
pub(crate) trait DynamicHandler<Local: JrRole, Remote: JrRole>: Send {
    fn dyn_handle_message(
        &mut self,
        message: MessageAndCx<Local, Remote, UntypedMessage, UntypedMessage>,
    ) -> BoxFuture<
        '_,
        Result<Handled<MessageAndCx<Local, Remote, UntypedMessage, UntypedMessage>>, crate::Error>,
    >;

    fn dyn_describe_chain(&self) -> String;
}

impl<H: JrMessageHandlerSend<Local, Remote>, Local: JrRole, Remote: JrRole>
    DynamicHandler<Local, Remote> for H
{
    fn dyn_handle_message(
        &mut self,
        message: MessageAndCx<Local, Remote, UntypedMessage, UntypedMessage>,
    ) -> BoxFuture<
        '_,
        Result<Handled<MessageAndCx<Local, Remote, UntypedMessage, UntypedMessage>>, crate::Error>,
    > {
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
