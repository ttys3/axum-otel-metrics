use axum::{routing::get, Router};
use axum::response::IntoResponse;

pub fn routes() -> Router<crate::SharedState> {
    Router::new()
        .route("/sub1", get(sub1))
        .route("/sub2", get(sub2))
}

async fn sub1() -> impl IntoResponse {
    "sub1"
}

async fn sub2() -> impl IntoResponse {
    "sub2"
}