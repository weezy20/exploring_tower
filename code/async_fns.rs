use super::*;

pub(crate) async fn lazy_function(_req : Request<Body>) -> Result<Response<Body>, Infallible> {
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    Ok(
        Response::new(
            Body::from(
                "Hello from LAZY function"
            )
        )
    )

}
pub(crate) async fn quick_function(_req : Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(
        Response::new(
            Body::from(
                "Hello from QUICK function"
            )
        )
    )
}
    