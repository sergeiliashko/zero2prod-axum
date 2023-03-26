use crate::helpers::spawn_app;

#[tokio::test]
async fn get_health_check_returns_200() {
    let test_app = spawn_app().await;
    dbg!(&test_app.address);

    let client = hyper::Client::new();

    let response = client
        .request(
            hyper::Request::builder()
                .method(hyper::Method::GET)
                .uri(format!("http://{}/health_check", &test_app.address))
                .body(hyper::body::Body::empty())
                .expect("Request builder should build request in tests"),
        )
        .await
        .unwrap();

    assert_eq!(hyper::StatusCode::OK, response.status());

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    assert_eq!(b"", &body[..]);
}
