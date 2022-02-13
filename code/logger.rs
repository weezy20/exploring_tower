use std::task::{Context, Poll};
use core::future::{Future};
use tower::Service;
use hyper::{Request, Response, Server};
use serde_json::Value;
use std::pin::Pin;

// We start by first building a middleware for our HelloWorld service that will implement
// the logging functionality. IOW, how to build a tower service that logs initiation and completion
// of a wrapped service, without heap allocations if possible
#[derive(Clone, Copy)]
pub struct Logger<Service> {
    inner: Service,
}
impl<S> Logger<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}
/// Impl logger for HTTP Services
impl<InnerService, B> Service<Request<B>> for Logger<InnerService>
where
    InnerService: Service<Request<B>> + Clone + Send + 'static,
    B: Send + 'static,
    // We shouldn't impose Unpin on InnerService because ServiceFn<_> which
    // is created by hyper's serve_fn isn't Unpin, so those services won't work with our logger
    InnerService::Future: 'static + Send,
{
    // Logging takes the inner service, and just runs a timer to wait its completion, logs
    // the result and then returns the response of the inner service
    type Response = InnerService::Response;
    type Error = InnerService::Error;
    type Future = LoggerFuture<InnerService::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    /// Return the Future of the wrapped service, while logging
    fn call(&mut self, req: Request<B>) -> Self::Future {
        // since we are mutably borrowing self
        // let inner = self.inner.clone();
        let (method, uri) = (req.method().clone(), req.uri().clone());
        let mut data = serde_json::Map::new();
        data.insert("method".to_string(), Value::from(method.as_str()));
        data.insert("uri".to_string(), Value::from(uri.path()));
        let extra_service_info = Some(ServiceInfo { data });

        let log_id: u64 = rand::random();
        log::info!("Received {method} {uri} request, dispatch log id: {log_id}");
        LoggerFuture {
            extra_service_info,
            log_id,
            f: self.inner.call(req),
        }
    }
}
#[derive(Clone)]
pub struct ServiceInfo {
    data: serde_json::Map<String, Value>,
}

#[pin_project::pin_project]
/// Our own future provides concrete type definitions
/// which avoids BoxFuture heap allocs
pub struct LoggerFuture<InnerServiceFuture: Future> {
    /// Metadata
    extra_service_info: Option<ServiceInfo>,
    /// UID for this logger
    log_id: u64,
    /// The service to be logged
    #[pin]
    f: InnerServiceFuture,
}

impl<F: Future> Future for LoggerFuture<F> {
    type Output = <F as Future>::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let log_id = self.log_id;
        // If our service is an HTTP service, we require this metadata
        let (mut metadata_init, mut method, mut uri) =
            (false, Value::default(), Value::default());

        if let Some(ref metadata) = self.extra_service_info {
            method = metadata.data.get("method").unwrap().clone();
            uri = metadata.data.get("uri").unwrap().clone();
            metadata_init = true;
        }
        // If inner service is a HTTP service
        let log_success_str = if metadata_init {
            format!("HTTP Request {method} {uri} processed successfully from logger id {log_id}")
        } else {
            format!("Request processed successfully from logger id {log_id}")
        };

        let inner = self.project().f; // Move occurs here so we must get the metadata before
                                      // Start polling the inner future
        if let Poll::Ready(fut) = inner.poll(cx) {
            log::info!("{}", log_success_str);
            Poll::Ready(fut)
        } else {
            Poll::Pending
        }
    }
}
