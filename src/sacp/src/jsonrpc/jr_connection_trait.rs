//! Trait-based interface for `JrConnection` that hides type parameters.

use crate::*;
use futures::{AsyncRead, AsyncWrite};

/// Create a new JrConnection from output and input streams.
///
/// This is a convenience function that returns `impl JrConnectionTrait`, allowing
/// users to work with connections without naming concrete types.
///
/// # Example
///
/// ```no_run
/// # use sacp::{new_connection, JrConnectionTrait, AgentCapabilities};
/// # use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
/// # async fn example() -> Result<(), sacp::Error> {
/// new_connection(
///     tokio::io::stdout().compat_write(),
///     tokio::io::stdin().compat(),
/// )
/// .name("my-agent")
/// .on_receive_request(async |req: sacp::InitializeRequest, cx| {
///     cx.respond(sacp::InitializeResponse {
///         protocol_version: req.protocol_version,
///         agent_capabilities: AgentCapabilities::default(),
///         auth_methods: Default::default(),
///         agent_info: Default::default(),
///         meta: Default::default(),
///     })
/// })
/// .serve()
/// .await?;
/// # Ok(())
/// # }
/// ```
pub fn new_connection(
    outgoing_bytes: impl AsyncWrite + 'static,
    incoming_bytes: impl AsyncRead + 'static,
) -> impl JrConnectionTrait {
    JrConnection::new(outgoing_bytes, incoming_bytes)
}

/// Trait interface for `JrConnection` that hides type parameters behind `impl Trait`.
///
/// This allows users to work with connections without naming the concrete types
/// for writers, readers, and handler chains.
#[allow(async_fn_in_trait)]
pub trait JrConnectionTrait: Sized {
    /// Create a new JrConnection from output and input streams.
    fn new(
        outgoing_bytes: impl AsyncWrite + 'static,
        incoming_bytes: impl AsyncRead + 'static,
    ) -> impl JrConnectionTrait;

    /// Set the "name" of this connection -- used only for debugging logs.
    fn name(self, name: impl ToString) -> impl JrConnectionTrait;

    /// Register a handler for JSON-RPC requests of type `R`.
    fn on_receive_request<R, F>(self, op: F) -> impl JrConnectionTrait
    where
        R: JrRequest,
        F: AsyncFnMut(R, JrRequestCx<R::Response>) -> Result<(), Error>;

    /// Register a handler for JSON-RPC notifications of type `N`.
    fn on_receive_notification<N, F>(self, op: F) -> impl JrConnectionTrait
    where
        N: JrNotification,
        F: AsyncFnMut(N, JrConnectionCx) -> Result<(), Error>;

    /// Register a handler for messages that can be either requests OR notifications.
    fn on_receive_message<R, N, F>(self, op: F) -> impl JrConnectionTrait
    where
        R: JrRequest,
        N: JrNotification,
        F: AsyncFnMut(MessageAndCx<R, N>) -> Result<(), Error>;

    /// Returns a [`JrConnectionCx`] that allows you to send requests over the connection.
    fn connection_cx(&self) -> JrConnectionCx;

    /// Enqueue a task to run once the connection is actively serving traffic.
    fn with_spawned(
        self,
        task: impl Future<Output = Result<(), Error>> + Send + 'static,
    ) -> impl JrConnectionTrait;

    /// Run the connection in server mode, processing incoming messages until the connection closes.
    async fn serve(self) -> Result<(), Error>;

    /// Run the connection in client mode, both handling incoming messages and sending your own.
    async fn with_client(
        self,
        main_fn: impl AsyncFnOnce(JrConnectionCx) -> Result<(), Error>,
    ) -> Result<(), Error>;
}

impl<OB, IB, H> JrConnectionTrait for JrConnection<OB, IB, H>
where
    OB: AsyncWrite,
    IB: AsyncRead,
    H: JrHandler,
{
    fn new(
        outgoing_bytes: impl AsyncWrite + 'static,
        incoming_bytes: impl AsyncRead + 'static,
    ) -> impl JrConnectionTrait {
        JrConnection::new(outgoing_bytes, incoming_bytes)
    }

    fn name(self, name: impl ToString) -> impl JrConnectionTrait {
        JrConnection::name(self, name)
    }

    fn on_receive_request<R, F>(self, op: F) -> impl JrConnectionTrait
    where
        R: JrRequest,
        F: AsyncFnMut(R, JrRequestCx<R::Response>) -> Result<(), Error>,
    {
        JrConnection::on_receive_request(self, op)
    }

    fn on_receive_notification<N, F>(self, op: F) -> impl JrConnectionTrait
    where
        N: JrNotification,
        F: AsyncFnMut(N, JrConnectionCx) -> Result<(), Error>,
    {
        JrConnection::on_receive_notification(self, op)
    }

    fn on_receive_message<R, N, F>(self, op: F) -> impl JrConnectionTrait
    where
        R: JrRequest,
        N: JrNotification,
        F: AsyncFnMut(MessageAndCx<R, N>) -> Result<(), Error>,
    {
        JrConnection::on_receive_message(self, op)
    }

    fn connection_cx(&self) -> JrConnectionCx {
        JrConnection::connection_cx(self)
    }

    fn with_spawned(
        self,
        task: impl Future<Output = Result<(), Error>> + Send + 'static,
    ) -> impl JrConnectionTrait {
        JrConnection::with_spawned(self, task)
    }

    async fn serve(self) -> Result<(), Error> {
        JrConnection::serve(self).await
    }

    async fn with_client(
        self,
        main_fn: impl AsyncFnOnce(JrConnectionCx) -> Result<(), Error>,
    ) -> Result<(), Error> {
        JrConnection::with_client(self, main_fn).await
    }
}
