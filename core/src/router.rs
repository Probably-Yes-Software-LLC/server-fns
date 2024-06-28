use axum::Router;

pub type RouterFn<S> = fn() -> Router<S>;

pub trait ServerFnsRouter {
    type State: Clone + Send + Sync + 'static;

    fn route_server_fns() -> Router<Self::State>;
}

pub struct ServerFnsPlugin<S> {
    pub router_fn: RouterFn<S>
}

impl<S> ServerFnsPlugin<S> {
    pub const fn new(router_fn: RouterFn<S>) -> Self {
        Self { router_fn }
    }
}
