use std::net::TcpListener;
use hyper;
use pretty_assertions::assert_eq;

use zero2prod::app;


fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .expect("Failed to bind to random port");
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        hyper::Server::from_tcp(listener)
            .unwrap()
            .serve(app().await.into_make_service())
            .await
            .unwrap();
    });

    addr.to_string()
}

#[tokio::test]
async fn get_health_check_returns_200() {

    let app_address = spawn_app();

    let client = hyper::Client::new();

    let response = client
        .request(
            hyper::Request::builder()
                .method(hyper::Method::GET)
                .uri(format!("http://{}/health_check", app_address))
                .body(hyper::body::Body::empty())
                .expect("Request builder should build request in tests"),
        )
        .await
        .unwrap();

    assert_eq!(hyper::StatusCode::OK, response.status() );

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    assert_eq!( b"", &body[..]);
}
