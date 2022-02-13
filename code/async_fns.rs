use super::*;
pub(crate) async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
    .await
    .expect("failed to install CTRL+C signal handler");
    log::warn!("Shutting down");
}

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
    