use std::net::SocketAddr;

use zero2prod::app;

#[tokio::main]
async fn main() {

    let addr = SocketAddr::from(([127, 0, 0, 1], 7878));

    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app().await.into_make_service())
        .await
        .unwrap();

}

