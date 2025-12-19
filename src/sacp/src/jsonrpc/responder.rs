//! Responder trait for background tasks that run alongside a connection.
//!
//! Responders are composable background tasks that run while a connection is active.
//! They're used for things like MCP tool handlers that need to receive calls through
//! channels and invoke user-provided closures.

use std::future::Future;

use futures::future::BoxFuture;

use crate::{JrConnectionCx, role::JrRole};

/// A responder runs background tasks alongside a connection.
///
/// Responders are composed using [`ChainResponder`] and run in parallel
/// when the connection is active.
#[expect(async_fn_in_trait)]
pub trait JrResponder<Role: JrRole> {
    /// Run this responder to completion.
    async fn run(self, cx: JrConnectionCx<Role>) -> Result<(), crate::Error>;

    /// Hacky method that asserts that the [`JrResponder::run`] method is `Send`.
    /// The argument `run` should be `JrResponder::run`.
    /// This is a workaround sometimes required until [#109417](https://github.com/rust-lang/rust/issues/109417)
    /// is stabilized.
    fn assert_send<'scope, RunFuture>(self, run: impl Fn(Self, JrConnectionCx<Role>) -> RunFuture + Send + 'scope) -> AssertSend<'scope, Role, Self>
    where 
        Self: Sized,
        RunFuture: Future<Output = Result<(), crate::Error>> + Send + 'scope,
    {
        AssertSend::new(
            Role::default(),
            self,
            run
        )
    }
}

/// Hacky struct that asserts that the [`Responder::run`] method is `Send`.
/// Produced by [`Responder::assert_send`].
pub struct AssertSend<'scope, Role: JrRole, Responder> {
    #[expect(dead_code)]
    role: Role,
    responder: Responder,
    run: Box<dyn Fn(Responder, JrConnectionCx<Role>) -> BoxFuture<'scope, Result<(), crate::Error>> + Send + 'scope>,
}

impl<'scope, Role, Responder> AssertSend<'scope, Role, Responder>
where 
    Role: JrRole,
    Responder: JrResponder<Role>,
{
    /// Create a new `AssertSend` wrapper with `run` serving as evidence that a send future can be produced.
    fn new<RunFuture>(role: Role, responder: Responder, run: impl Fn(Responder, JrConnectionCx<Role>) -> RunFuture + Send + 'scope) -> Self
    where 
        RunFuture: Future<Output = Result<(), crate::Error>> + Send + 'scope,
    {
        Self {
            role,
            responder,
            run: Box::new(move |responder, cx| Box::pin(run(responder, cx))),
        }
    }
}

impl<Role, Responder> JrResponder<Role> for AssertSend<'_, Role, Responder>
where 
    Role: JrRole,
    Responder: JrResponder<Role>,
{
    async fn run(self, cx: JrConnectionCx<Role>) -> Result<(), crate::Error> {
        (self.run)(self.responder, cx).await
    }
}

/// A no-op responder that completes immediately.
#[derive(Default)]
pub struct NullResponder;

impl<Role: JrRole> JrResponder<Role> for NullResponder {
    async fn run(self, _cx: JrConnectionCx<Role>) -> Result<(), crate::Error> {
        Ok(())
    }
}

/// Chains two responders to run in parallel.
pub struct ChainResponder<A, B> {
    a: A,
    b: B,
}

impl<A, B> ChainResponder<A, B> {
    /// Create a new chained responder from two responders.
    pub fn new(a: A, b: B) -> Self {
        Self { a, b }
    }
}

impl<Role: JrRole, A: JrResponder<Role>, B: JrResponder<Role>> JrResponder<Role>
    for ChainResponder<A, B>
{
    async fn run(self, cx: JrConnectionCx<Role>) -> Result<(), crate::Error> {
        // Box the futures to avoid stack overflow with deeply nested responder chains
        let a_fut = Box::pin(self.a.run(cx.clone()));
        let b_fut = Box::pin(self.b.run(cx.clone()));
        let ((), ()) = futures::future::try_join(a_fut, b_fut).await?;
        Ok(())
    }
}

/// A responder created from a closure via [`with_spawned`](crate::JrConnectionBuilder::with_spawned).
pub struct SpawnedResponder<F> {
    task_fn: F,
    location: &'static std::panic::Location<'static>,
}

impl<F> SpawnedResponder<F> {
    /// Create a new spawned responder from a closure.
    pub fn new(location: &'static std::panic::Location<'static>, task_fn: F) -> Self {
        Self { task_fn, location }
    }
}

impl<Role, F, Fut> JrResponder<Role> for SpawnedResponder<F>
where
    Role: JrRole,
    F: FnOnce(JrConnectionCx<Role>) -> Fut + Send,
    Fut: Future<Output = Result<(), crate::Error>> + Send,
{
    async fn run(self, cx: JrConnectionCx<Role>) -> Result<(), crate::Error> {
        let location = self.location;
        (self.task_fn)(cx).await.map_err(|err| {
            let data = err.data.clone();
            err.with_data(serde_json::json! {
                {
                    "spawned_at": format!("{}:{}:{}", location.file(), location.line(), location.column()),
                    "data": data,
                }
            })
        })
    }
}
