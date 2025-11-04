//! Test using JrConnectionTrait interface to hide type parameters

use sacp::{
    AgentCapabilities, InitializeRequest, InitializeResponse, JrConnectionTrait, MessageAndCx,
    UntypedMessage,
};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

// Helper function that returns impl JrConnectionTrait
fn create_connection() -> impl JrConnectionTrait {
    sacp::JrConnection::new(
        tokio::io::stdout().compat_write(),
        tokio::io::stdin().compat(),
    )
}

#[tokio::main]
async fn main() -> Result<(), sacp::Error> {
    // Create connection using helper - completely type-parameter-free!
    let connection = create_connection();

    // Chain methods - all return `impl JrConnectionTrait`
    connection
        .name("trait-test-agent")
        .on_receive_request(async |init: InitializeRequest, cx| {
            cx.respond(InitializeResponse {
                protocol_version: init.protocol_version,
                agent_capabilities: AgentCapabilities::default(),
                auth_methods: Default::default(),
                agent_info: Default::default(),
                meta: Default::default(),
            })
        })
        .on_receive_message(async |msg: MessageAndCx<UntypedMessage, UntypedMessage>| {
            msg.respond_with_error(sacp::util::internal_error("unhandled"))
        })
        .serve()
        .await?;

    Ok(())
}
