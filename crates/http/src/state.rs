use std::sync::Arc;

use app::services::ServiceSet;

use crate::health::HealthProbe;

/// Shared by every handler.
#[derive(Clone)]
pub struct ApiState {
    pub services: ServiceSet,
    pub health: Arc<dyn HealthProbe>,
}
