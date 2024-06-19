use axum::Router;
use linkme::distributed_slice;

#[distributed_slice]
pub static COLLATED_ROUTES: [fn() -> Router];

pub struct ServerFnsRouter;

impl ServerFnsRouter {
    pub fn new() -> Router {
        let mut router = Router::new();

        for route in COLLATED_ROUTES {
            router = router.merge(route());
        }

        router
    }
}
