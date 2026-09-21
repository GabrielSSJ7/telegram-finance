use std::net::SocketAddr;
use std::sync::Arc;

use app::services::ServiceSet;
use http_api::{ApiState, HealthProbe, RouterOptions, api_router};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

/// Serves the REST API on `listener` until `shutdown` is cancelled, then
/// lets in-flight requests finish.
pub async fn serve_http(
    listener: TcpListener,
    services: ServiceSet,
    health: Arc<dyn HealthProbe>,
    swagger: bool,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let options = RouterOptions { swagger, ..RouterOptions::default() };
    let router = api_router(ApiState { services, health }, &options);
    tracing::info!(address = %listener.local_addr()?, swagger, "http api listening");
    let service = router.into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, service).with_graceful_shutdown(shutdown.cancelled_owned()).await?;
    tracing::info!("http api stopped");
    Ok(())
}
