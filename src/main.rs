use axum::{
    extract::Query,
    http::header,
    response::{Html, IntoResponse, Redirect},
    routing::get,
    Router,
};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tower_http::services::ServeDir;

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

async fn handle_index(Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
    match params.get("v") {
        Some(v) if v.parse::<u64>().is_ok() => {
            let html = include_str!("../static/index.html").replace("{{VERSION}}", v);
            (
                [(header::CACHE_CONTROL, "no-store")],
                Html(html),
            ).into_response()
        }
        _ => {
            let url = format!("/?v={}", now_secs());
            Redirect::to(&url).into_response()
        }
    }
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
