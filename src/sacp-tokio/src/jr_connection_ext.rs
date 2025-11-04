//! Utilities for creating connections to spawned agent processes.

use crate::AcpAgent;
use sacp::JrConnectionTrait;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::process::Child;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

/// A future that holds a `Child` process and never resolves.
/// When dropped, the child process is killed.
struct ChildHolder {
    _child: Child,
}

impl Future for ChildHolder {
    type Output = Result<(), sacp::Error>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Never ready - just hold the child process alive
        Poll::Pending
    }
}

impl Drop for ChildHolder {
    fn drop(&mut self) {
        let _: Result<_, _> = self._child.start_kill();
    }
}

impl AcpAgent {
    /// Spawn the agent and create a connection to it.
    ///
    /// The child process is managed automatically - a background task holds it alive,
    /// and when the connection is dropped, the process will be killed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::str::FromStr;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use sacp_tokio::AcpAgent;
    /// # use sacp::JrConnectionTrait;
    /// let agent = AcpAgent::from_str("python my_agent.py")?;
    ///
    /// agent.connection()?
    ///     .on_receive_notification(|notif: sacp::SessionNotification, _cx| async move {
    ///         println!("{:?}", notif);
    ///         Ok(())
    ///     })
    ///     .with_client(|cx: sacp::JrConnectionCx| async move {
    ///         // Use the connection...
    ///         Ok(())
    ///     })
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn connection(&self) -> Result<impl JrConnectionTrait, sacp::Error> {
        let (child_stdin, child_stdout, child) = self.spawn_process()?;

        let connection = sacp::new_connection(child_stdin.compat_write(), child_stdout.compat())
            .with_spawned(ChildHolder { _child: child });

        Ok(connection)
    }
}
