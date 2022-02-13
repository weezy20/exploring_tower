use core::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::time::Instant;
use tower::{BoxError, Service};
/// Timeout is a wrapper over a service that times how long a service
/// takes to response and if exceeds a certain amount, responds with a Error
#[derive(Clone)]
pub struct Timeout<Service> {
    pub inner: Service,
}
impl<S> Timeout<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}
/// We don't care about HTTP requests specifically for timeouts
impl<S: Service<R>, R> Service<R> for Timeout<S>
where
    <S as Service<R>>::Error: 'static + Send + Sync + std::error::Error,
{
    type Response = S::Response;
    /// We can have an inner service failure or a timeout error
    type Error = tower::BoxError;
    type Future = TimeoutFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }
    fn call(&mut self, req: R) -> Self::Future {
        TimeoutFuture {
            f: self.inner.call(req),
            timer: Instant::now(),
        }
    }
}

#[pin_project::pin_project]
pub struct TimeoutFuture<F: Future> {
    /// Inner service's future type
    #[pin]
    f: F,
    /// Duration elapsed to `call` inner service future
    timer: Instant,
}

impl<F, T, InnerServiceErr> Future for TimeoutFuture<F>
where
    F: Future<Output = Result<T, InnerServiceErr>>,
    InnerServiceErr: Into<BoxError>,
{
    type Output = Result<T, BoxError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let start = (&self.timer).clone();
        let inner = self.project().f;
        // Poll inner future
        match inner.poll(cx) {
            Poll::Ready(response) => {
                let duration = Instant::now().duration_since(start);
                log::info!("Task finished in {} milliseconds", duration.as_millis());
                Poll::Ready(response.map_err(Into::into))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
