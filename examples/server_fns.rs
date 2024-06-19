use axum::response::Html;
use server_fns::{axum_router::ServerFnsRouter, server};

#[tokio::main]
async fn main() {
    let router = ServerFnsRouter::collect_routes();
}

#[server(path = "/", method = "GET")]
async fn index() -> Html<String> {
    let html = "<body>Index</body>";
    Html(html)
}

#[server]
pub async fn example() -> Result<(), ()> {
    // body
    Ok(())
}
