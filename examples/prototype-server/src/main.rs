use axum::{
    extract::Request,
    middleware::Next,
    response::{Html, Response}
};
use server_fns::{get, middleware, server_state::ServerState, use_server_state};
use server_state::{AppState, InnerState};

use_server_state!(AppState);

#[tokio::main]
async fn main() {
    println!("after auto routes");

    let app = AppState {
        inner: InnerState {
            state: "fucking works bitch".to_string()
        }
    }
    .load_routes();

    println!("app {app:?}");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3333").await.unwrap();
    println!("bound to {listener:?}");
    axum::serve(listener, app).await.unwrap();
}

async fn test_middleware(request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    response
}

async fn test_middleware2(request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    response
}

#[middleware(axum::middleware::from_fn(test_middleware))]
#[middleware(axum::middleware::from_fn(test_middleware2))]
#[get(path = "/", embed = "$CARGO_MANIFEST_DIR/..")]
async fn index(#[state] AppState { inner }: AppState) -> Html<String> {
    let html = format!("<body>index and {inner:?}</body>");

    // let path = "asset/something/else.ts";

    let test = load_asset!("/test");

    Html(html)
}
