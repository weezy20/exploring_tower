//! Hello world service implementation
//! Doesn't contain any data so clones and copies are trivial

use super::*;
#[derive(Clone, Copy)]
pub struct HelloWorld;

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
        ready(Ok(Response::new(Body::from(
            "Hello World from immediate future",
        ))))
    }
}
