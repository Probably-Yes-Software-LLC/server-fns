use axum::Router;
use linkme::distributed_slice;

#[distributed_slice]
pub static COLLATED_ROUTES: [fn(Box<dyn State>) -> Router];

pub struct ServerFnsRouter;

impl ServerFnsRouter {
    pub fn collect_routes(state: Box<dyn State>) -> Router {
        let mut router = Router::new();

        for route in COLLATED_ROUTES {
            router = router.merge(route(state.clone()));
        }

        router
    }
}

pub trait State {}
