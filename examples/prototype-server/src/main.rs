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
    let router = AppState::load_routes();

    println!("after auto routes");

    let app = router.with_state(AppState {
        inner: InnerState {
            state: "fucking works bitch".to_string()
        }
    });

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
#[get(path = "/")]
async fn index(#[state] AppState { inner }: AppState) -> Html<String> {
    let html = format!("<body>index and {inner:?}</body>");
    Html(html)
}
