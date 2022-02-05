use futures::future::{ready, Ready, BoxFuture};
#[allow(unused)]
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::task::{Context, Poll};
use tower::Service;

#[tokio::main]
async fn main() {
    env_logger::init();
    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3000);
    let make_service =
        // the inner closure returns a Result of a tower service
        // service_fn is a re-export of tower::service_fn(f: T) -> ServiceFn<T>
        // ServiceFn<T> implements Service trait
        // where T: FnMut(Request) -> F,
        // F: Future<Output = Result<R, E>>
        make_service_fn(|_conn| async { Ok::<_, Infallible>(HelloWorld) });
    let server = Server::bind(&socket).serve(make_service);
    if let Err(e) = server.await {
        eprintln!("Server Error : {e}");
    }
}


struct HelloWorld;

// Implement for a regular HTTP request 
impl Service<Request<Body>> for HelloWorld {
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, req: Request<Body>) -> Self::Future {
        log::debug!("Received {method} {uri} request", method = req.method(), uri = req.uri().path());
        ready(Ok(Response::new(Body::from(
            "Hello World from immediate future",
        ))))
    }
}
