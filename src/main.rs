#![allow(unused_imports)]
use futures::future::{ready, BoxFuture, Ready};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::marker::PhantomData;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::task::{Context, Poll};
use tower::Service;

static mut LOG_ID : u32 = 0;

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
        // F: Future<Output = Result<R, E>>
        make_service_fn(|_conn| async { 
            // We change our service from HelloWorld to Logger<HelloWorld> to ensure
            // that HelloWorld is called through our Logging service

            // We can wrap one service in another to see how services tower
            // For this we get rid of the trait bounds in our previous commit
            let service = HelloWorld;
            let service = Logger::new(HelloWorld);
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
// the logging functionality
// We had to include PhantomData to include the trait bound on `Service` generic
#[derive(Clone, Copy)]
struct Logger<Service> {
    inner: Service,
}
impl<S> Logger<S> {
    fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S,B> Service<Request<B>> for Logger<S>
where
    S: Service<Request<B>> + Clone + Send + 'static,
    B: Send + 'static ,
    S::Future : 'static + Send,
    {
    // Logging takes the inner service, and just runs a timer to wait its completion, logs
    // the result and then returns the response of the inner service
    type Response = S::Response;
    type Error = S::Error;
    // type Future = S::Future; // Replace this line
    type Future = BoxFuture<'static , Result<S::Response, S::Error>>; // With this one

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        // since we are mutably borrowing self
        let mut inner = self.inner.clone();
        let (method, uri) = (req.method().clone(), req.uri().clone());
        Box::pin(async move {
            let log_id = unsafe { 
                let log_id = LOG_ID + 1;
                LOG_ID = log_id;
                log_id
             };
            log::debug!("Received {method} {uri} request from LOGGER ID {log_id}");
    
            // Now we have a problem. If we are logging before and after calling the inner service 
            // How do we return the future as a future? BoxFuture to the Rescue. We change type Future = S::Future to 
            // type Future = BoxFuture;
            // Next instead of returning the inner future we await it, then wrap the entire computation in a BoxFuture and
            // return it as the result of Logger::call
            let response = inner.call(req).await;
    
            log::debug!("Finished processing {method} {uri} request from LOGGER ID {log_id}");

            response
        })
        
    }
}
 