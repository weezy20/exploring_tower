use futures::future::{ready, Ready};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::task::{Context, Poll};
use tower::Service;

// Local imports
mod logger;
mod timeout;
mod async_fns;
mod hello_world;

use timeout::Timeout;
use logger::Logger;
use async_fns::*;
use hello_world::HelloWorld;

#[allow(unused)]
#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        let val = format!("{}=DEBUG", module_path!().split("::").next().unwrap());
        std::env::set_var("RUST_LOG", val );
    } else {
        println!("RUST_LOG Set");
    }
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
            let service = service_fn(quick_function);
            let service = service_fn(lazy_function);
            let service = Logger::new(service);
            let service = Logger::new(service);
            let service = Logger::new(service);
            let service = Logger::new(service);
            let service = Timeout::new(Logger::new(service));

            Ok::<_, Infallible>(service) });
    let server = Server::bind(&socket)
                    .serve(make_service)
                    .with_graceful_shutdown(shutdown_signal());
    if let Err(e) = server.await {
        eprintln!("Server Error : {e}");
    }
}

async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
    .await
    .expect("failed to install CTRL+C signal handler");
    log::warn!("Shutting down");
}

