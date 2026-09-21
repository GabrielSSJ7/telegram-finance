use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::error::Problem;

#[derive(OpenApi)]
#[openapi(
    info(title = "finbot API", description = "Couple finance ledger: accounts, entries, goals, settings."),
    components(schemas(Problem)),
    modifiers(&BearerSecurity),
    tags(
        (name = "accounts"), (name = "budgets"), (name = "cards"), (name = "categories"), (name = "entries"),
        (name = "goals"), (name = "recurrences"), (name = "reports"), (name = "settings"), (name = "health"),
    )
)]
pub struct ApiDoc;

struct BearerSecurity;

impl Modify for BearerSecurity {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        let scheme = Http::builder().scheme(HttpAuthScheme::Bearer).bearer_format("fbk_…").build();
        components.add_security_scheme("api_key", SecurityScheme::Http(scheme));
    }
}
