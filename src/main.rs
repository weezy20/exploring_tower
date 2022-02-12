#![allow(unused_imports)]
use futures::Future;
use futures::future::{FutureExt, ready, BoxFuture, Ready, self, Map};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use serde_json::Value;
use std::convert::Infallible;
use std::marker::PhantomData;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::Service;


#[tokio::main]
async fn main() {
    std::env::set_var("RUST_LOG", "tower_explorer=DEBUG");
    env_logger::init();
    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3000);
    let make_service =
        // the inner closure returns a Result of a tower service
        // service_fn is a re-export of tower::service_fn(f: T) -> ServiceFn<T>
        // ServiceFn<T> implements Service trait
        // where T: FnMut(Request) -> F,
        // F: Future<Output = Result<Response, E>>
        make_service_fn(|_conn| async { 
            // We change our service from HelloWorld to Logger<HelloWorld> to ensure
            // that HelloWorld is called through our Logging service

            // We can wrap one service in another to see how services tower
            // For this we get rid of the trait bounds in our previous commit
            let service = HelloWorld;
            let service = Logger::new(service);
            let service = Logger::new(service);
            let service = Logger::new(service);
            let service = Logger::new(service);
            let service = Logger::new(service);

            Ok::<_, Infallible>(service) });
    let server = Server::bind(&socket).serve(make_service);
    if let Err(e) = server.await {
        eprintln!("Server Error : {e}");
    }
}
// Doesn't contain any data so clones and copies are trivial
#[derive(Clone, Copy)]
struct HelloWorld;

// Implement for a regular HTTP request
#[rustfmt::skip]
impl Service<Request<Body>> for HelloWorld {
    type Response = Response<Body>;
    type Error = Infallible;
    // Ready is Unpin so it works with our current LoggerFuture
    type Future = Ready<Result<Self::Response, Self::Error>>;
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    // Problem that we are trying to solve is how to implement logging in a tower service
    // when services can be layered. Also we want to avoid allocation on every request.
    fn call(&mut self, _req: Request<Body>) -> Self::Future {
        ready(
           Ok(Response::new(Body::from("Hello World from immediate future",))) 
        )
    }
}

// We start by first building a middleware for our HelloWorld service that will implement
// the logging functionality. IOW, how to build a tower service that logs initiation and completion
// of a wrapped service, without heap allocations if possible
#[derive(Clone, Copy)]
struct Logger<Service> {
    inner: Service,
}
impl<S> Logger<S> {
    fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<InnerService,B> Service<Request<B>> for Logger<InnerService>
where
    InnerService: Service<Request<B>> + Clone + Send + 'static,
    B: Send + 'static ,
    // We shouldn't impose Unpin on InnerService because ServiceFn<_> which 
    // is created by hyper's serve_fn isn't Unpin, so those services won't work with our logger
    InnerService::Future : 'static + Send, 
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

        let log_id : u64 = rand::random();
        log::debug!("Received {method} {uri} request, dispatch log id: {log_id}");

       LoggerFuture { extra_service_info, log_id, f : self.inner.call(req) }
       
    }
}
#[derive(Clone)]
struct ServiceInfo {
    data: serde_json::Map<String, Value>
}


#[pin_project::pin_project]
struct LoggerFuture<InnerServiceFuture: Future> {
    extra_service_info : Option<ServiceInfo>,
    log_id: u64,
    #[pin]
    f: InnerServiceFuture
}

impl<F: Future> Future for LoggerFuture<F> {
    type Output = <F as Future>::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let log_id = self.log_id;
        // If our service is an HTTP service, we require this metadata
        let (mut metadata_init , mut method, mut uri) = (false, Value::default(), Value::default());
        if let Some(ref metadata) = self.extra_service_info {
        
            method = metadata.data.get("method").unwrap().clone();
                 uri =   metadata.data.get("uri").unwrap().clone();
                 metadata_init = true;
    
        }
        // If inner service is a HTTP service 
        let log_success_str = if metadata_init {
            format!("HTTP Request {method} {uri} processed successfully from logger id {log_id}")
        } else {
            format!("Request processed successfully from logger id {log_id}")
        };

        let inner = self.project().f; // Move occurs here so we must get the metadata before 
        
        if let Poll::Ready(fut) = inner.poll(cx) {
            log::debug!("{}", log_success_str);
            Poll::Ready(fut)
        } else { 
            Poll::Pending
        }
    }

}