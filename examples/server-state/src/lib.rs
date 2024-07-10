#[cfg(feature = "server")]
use axum::extract::FromRef;
#[cfg(feature = "server")]
use server_fns::ServerState;

#[derive(Debug, Default, Clone)]
pub struct InnerState {
    pub state: String,
}

#[derive(Debug, Default, Clone, ServerState, FromRef)]
pub struct AppState {
    pub inner: InnerState,
}
