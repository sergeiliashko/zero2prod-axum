use crate::helpers::spawn_app;

#[tokio::test]
async fn post_subscribe_returns_200_for_valid_form_data() {
    let test_app = spawn_app().await;

    let client = hyper::Client::new();

    let request_body = "name=serj&email=serj%40rodrigess.com";
    let response = client
        .request(
            hyper::Request::builder()
                .method(hyper::Method::POST)
                .uri(format!("http://{}/subscriptions", &test_app.address))
                .header(
                    hyper::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(hyper::body::Body::from(request_body))
                .expect("Hyper request builder should build request"),
        )
        .await
        .unwrap();
    assert_eq!(hyper::StatusCode::OK, response.status());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "serj@rodrigess.com");
    assert_eq!(saved.name, "serj");
}

#[tokio::test]
async fn post_subscribe_returns_400_when_any_data_is_missing() {
    let test_app = spawn_app().await;

    let client = hyper::Client::new();

    let test_cases = vec![
        (
            "name=serj",
            "Failed to deserialize form body: missing field `email`",
        ),
        (
            "email=serj%40rodrigess.com",
            "Failed to deserialize form body: missing field `name`",
        ),
        ("", "Failed to deserialize form body: missing field `email`"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .request(
                hyper::Request::builder()
                    .method(hyper::Method::POST)
                    .uri(format!("http://{}/subscriptions", &test_app.address))
                    .header(
                        hyper::header::CONTENT_TYPE,
                        "application/x-www-form-urlencoded",
                    )
                    .body(hyper::body::Body::from(invalid_body))
                    .expect("Hyper request builder should build request"),
            )
            .await
            .unwrap();

        assert_eq!(hyper::StatusCode::UNPROCESSABLE_ENTITY, response.status());

        let body = hyper::body::to_bytes(response.into_body())
            .await
            .expect("hyper can consume response body to bytes");
        let body = String::from_utf8(body.into_iter().collect())
            .expect("String can consume response body in bytes to create a String");

        assert_eq!(
            error_message, body,
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty() {
    let test_app = spawn_app().await;
    let client = hyper::Client::new();

    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (invalid_body, _description) in test_cases {
        let response = client
            .request(
                hyper::Request::builder()
                    .method(hyper::Method::POST)
                    .uri(format!("http://{}/subscriptions", &test_app.address))
                    .header(
                        hyper::header::CONTENT_TYPE,
                        "application/x-www-form-urlencoded",
                    )
                    .body(hyper::body::Body::from(invalid_body))
                    .expect("Hyper request builder should build request"),
            )
            .await
            .unwrap();

        assert_eq!(
            hyper::StatusCode::BAD_REQUEST,
            response.status(),
            "The API did not return 400 Ok when the payload was {}.",
            invalid_body
        );
    }
}
