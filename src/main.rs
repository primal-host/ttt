use axum::{routing::get, Router};
use tower_http::services::ServeDir;

async fn handle_index() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("../static/index.html"))
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(handle_index))
        .fallback_service(ServeDir::new("static"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}
