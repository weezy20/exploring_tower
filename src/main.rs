#![allow(unused_imports)]
use futures::Future;
use futures::future::{FutureExt, ready, BoxFuture, Ready, self, Map};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
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
    InnerService::Future : 'static + Send + Unpin, 
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
        let mut inner = self.inner.clone();
        let (method, uri) = (req.method().clone(), req.uri().clone());
        log::debug!("Received {method} {uri} request");

       LoggerFuture {f : self.inner.call(req) }
       
    }
}
#[pin_project::pin_project]
struct LoggerFuture<InnerServiceFuture: Future> {
    #[pin]
    f: InnerServiceFuture
}

impl<F: Future + Unpin> Future for LoggerFuture<F> {
    type Output = <F as Future>::Output;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        
    }

}