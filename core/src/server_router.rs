use inventory::Collect;

pub type RouterFn<S> = fn() -> axum::Router<S>;

pub trait ServerRouter: Collect {
    type State: Clone + Send + Sync;

    fn router(&self) -> axum::Router<Self::State>;

    fn load_routes() -> axum::Router<Self::State> {
        let mut loaded = axum::Router::new();

        for next in inventory::iter::<Self> {
            loaded = loaded.merge(next.router());
        }

        loaded
    }
}
