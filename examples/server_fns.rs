use axum::{extract::State, response::Html};
use server_fns::{axum_router::ServerFnsRouter, server};

#[derive(Debug, Default, Clone)]
struct InnerState {
    state: String
}

#[derive(Debug, Default, Clone)]
struct AppState {
    inner: InnerState
}

#[tokio::main]
async fn main() {
    // let router = ServerFnsRouter::collect_routes();
}

#[server(path = "/", method = "GET")]
async fn index(#[state] AppState { inner }: AppState, body: String) -> Html<String> {
    let html = "<body>Index</body>";
    Html(html.to_string())
}

#[server]
pub async fn example(#[state] inner_state: InnerState, body: String) -> Result<(), ()> {
    // body
    Ok(())
}
