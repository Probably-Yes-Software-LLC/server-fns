use inventory::Collect;

use crate::server_state::ServerState;

pub type RouterFn<S> = fn() -> axum::Router<S>;

pub trait ServerRouter: Collect {
    type State: ServerState<Router = Self>;

    fn router(&self) -> axum::Router<Self::State>;

    fn load_routes() -> axum::Router<Self::State> {
        let mut loaded = axum::Router::new();

        for next in inventory::iter::<Self> {
            loaded = loaded.merge(next.router());
        }

        loaded
    }
}
