#![allow(unused_imports)]
use futures::Future;
use futures::future::{FutureExt, ready, BoxFuture, Ready, self, Map};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use rand::rngs::adapter::ReseedingRng;
use serde_json::Value;
use tokio::time::Instant;
use std::convert::Infallible;
use std::marker::PhantomData;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::Service;

// Local imports
mod logger;
mod timeout;
use timeout::Timeout;
use logger::Logger;
async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
    .await
    .expect("failed to install CTRL+C signal handler");
    log::warn!("Shutting down");
}

async fn lazy_function(_req : Request<Body>) -> Result<Response<Body>, Infallible> {
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    Ok(
        Response::new(
            Body::from(
                "Hello from LAZY function"
            )
        )
    )
}


async fn quick_function(_req : Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(
        Response::new(
            Body::from(
                "Hello from QUICK function"
            )
        )
    )
}

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
            let service = service_fn(lazy_function);
            let service = service_fn(quick_function);
            let service = Timeout::new(Logger::new(service));
            // let service = Logger::new(service);
            // let service = Logger::new(service);
            // let service = Logger::new(service);
            // let service = Logger::new(service);

            Ok::<_, Infallible>(service) });
    let server = Server::bind(&socket)
                        .serve(make_service)
                        .with_graceful_shutdown(shutdown_signal());
    if let Err(e) = server.await {
        eprintln!("Server Error : {e}");
    }
}
// Doesn't contain any data so clones and copies are trivial
#[derive(Clone, Copy)]
struct HelloWorld;

// Implement for a regular HTTP request
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

