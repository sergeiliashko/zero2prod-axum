use crate::helpers::spawn_app;

#[tokio::test]
async fn post_subscribe_returns_200_for_valid_form_data() {
    let test_app = spawn_app().await;

    let request_body = "name=serj&email=serj%40rodrigess.com";
    let response = test_app.post_subscriptions(request_body.into()).await;

    assert_eq!(hyper::StatusCode::OK, response.status().as_u16());

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
        let response = test_app.post_subscriptions(invalid_body.into()).await;
        assert_eq!(
            hyper::StatusCode::UNPROCESSABLE_ENTITY,
            response.status().as_u16()
        );

        let response_body = response
            .text()
            .await
            .expect("Cannot transform response body into text");

        assert_eq!(
            error_message, response_body,
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty() {
    let test_app = spawn_app().await;

    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (invalid_body, _description) in test_cases {
        let response = test_app.post_subscriptions(invalid_body.into()).await;
        assert_eq!(
            hyper::StatusCode::BAD_REQUEST,
            response.status().as_u16(),
            "The API did not return 400 Ok when the payload was {}.",
            invalid_body
        );
    }
}
