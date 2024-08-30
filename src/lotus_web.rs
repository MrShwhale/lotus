use axum::{routing::get, Router};
use lotus::{server, SERVER_HEADING};
use tokio;

#[tokio::main]
async fn main() {
    // Set up the recommender (optional, disabled for tests)
    // Otherwise, it will be run with the first thing that uses it

    lazy_static::initialize(&server::RECOMMENDER);

    eprintln!("{}Starting web server...", SERVER_HEADING);

    let app = Router::new()
        .route("/", get(server::root))
        .route("/rec", get(server::get_rec));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    eprintln!("{}Web server up!", SERVER_HEADING);
    axum::serve(listener, app).await.unwrap();
}
