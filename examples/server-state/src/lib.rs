#[cfg(feature = "server")]
use axum::extract::FromRef;
#[cfg(feature = "server")]
use server_fns::ServerState;

#[derive(Debug, Default, Clone)]
pub struct InnerState {
    pub state: String
}

#[cfg_attr(feature = "server", derive(ServerState, FromRef))]
#[derive(Debug, Default, Clone)]
pub struct AppState {
    pub inner: InnerState
}
