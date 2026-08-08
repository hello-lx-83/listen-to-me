use crate::core::models::ModelRoute;

#[derive(Debug, Default)]
pub struct ModelPolicy {
    route: ModelRoute,
}

impl ModelPolicy {
    pub fn route(&self) -> ModelRoute {
        self.route
    }
}
