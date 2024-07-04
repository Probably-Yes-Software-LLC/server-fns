use axum::{
    extract::{FromRef, Request},
    middleware::Next,
    response::{Html, Response}
};
use server_fns::{get, middleware, server_state::ServerState, ServerState};

#[derive(Debug, Default, Clone)]
pub struct InnerState {
    state: String
}

#[derive(Debug, Default, Clone, ServerState)]
pub struct AppState {
    inner: InnerState
}

impl FromRef<AppState> for InnerState {
    fn from_ref(input: &AppState) -> Self {
        input.inner.clone()
    }
}

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
async fn index(#[state] AppState { inner }: AppState, body: String) -> Html<String> {
    let html = format!("<body>index and {inner:?}</body>");
    Html(html)
}

// #[server(path = "/example", method = "GET")]
// pub async fn example(
//     #[state] _app_state: AppState,
//     #[state] _inner_state: InnerState,
//     _body: String
// ) -> Html<String> {
//     // body
//     Html("this is the example route".to_string())
// }

// #[server(method = "GET")]
// pub async fn test() -> Html<String> {
//     Html("this is the test route".to_string())
// }
