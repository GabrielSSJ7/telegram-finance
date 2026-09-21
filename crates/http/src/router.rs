use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::http::StatusCode;
use axum::middleware;
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use utoipa_swagger_ui::SwaggerUi;

use crate::auth::require_api_key;
use crate::openapi::ApiDoc;
use crate::routes::v1_routes;
use crate::state::ApiState;

/// Token bucket per client IP (read from `X-Forwarded-For` set by Caddy).
#[derive(Debug, Clone, Copy)]
pub struct RateLimit {
    pub replenish_every: Duration,
    pub burst: u32,
}

#[derive(Debug, Clone)]
pub struct RouterOptions {
    pub swagger: bool,
    pub rate_limit: Option<RateLimit>,
    pub request_timeout: Duration,
    pub body_limit_bytes: usize,
}

impl Default for RouterOptions {
    fn default() -> Self {
        let rate_limit = Some(RateLimit { replenish_every: Duration::from_millis(100), burst: 30 });
        Self {
            swagger: false,
            rate_limit,
            request_timeout: Duration::from_secs(10),
            body_limit_bytes: 64 * 1024,
        }
    }
}

/// The whole HTTP surface: `/healthz`, authenticated `/api/v1`, and
/// optionally Swagger UI at `/docs`.
///
/// ```ignore
/// let app = api_router(state, &RouterOptions::default());
/// axum::serve(listener, app).await?;
/// ```
pub fn api_router(state: ApiState, options: &RouterOptions) -> Router {
    let protected =
        v1_routes().route_layer(middleware::from_fn_with_state(state.clone(), require_api_key));
    let (router, openapi) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/v1", protected)
        .routes(routes!(crate::health::healthz))
        .split_for_parts();
    let router = with_swagger(router, openapi, options.swagger).with_state(state);
    with_limits(router, options)
}

fn with_swagger(
    router: Router<ApiState>,
    openapi: utoipa::openapi::OpenApi,
    enabled: bool,
) -> Router<ApiState> {
    if !enabled {
        return router;
    }
    router.merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", openapi))
}

fn with_limits(router: Router, options: &RouterOptions) -> Router {
    let router = router
        .layer(RequestBodyLimitLayer::new(options.body_limit_bytes))
        .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, options.request_timeout))
        .layer(TraceLayer::new_for_http());
    match options.rate_limit {
        Some(limit) => with_rate_limit(router, limit),
        None => router,
    }
}

fn with_rate_limit(router: Router, limit: RateLimit) -> Router {
    let millis = u64::try_from(limit.replenish_every.as_millis()).unwrap_or(u64::MAX).max(1);
    let config = GovernorConfigBuilder::default()
        .per_millisecond(millis)
        .burst_size(limit.burst.max(1))
        .key_extractor(SmartIpKeyExtractor)
        .finish();
    // `finish` only fails for a zero period or burst, both clamped above.
    let Some(config) = config else {
        tracing::warn!(?limit, "rate limit disabled: invalid configuration");
        return router;
    };
    router.layer(GovernorLayer::new(Arc::new(config)))
}
