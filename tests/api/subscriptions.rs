use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn post_subscribe_returns_200_for_valid_form_data() {
    let test_app = spawn_app().await;

    let request_body = "name=serj&email=serj%40rodrigess.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    let response = test_app.post_subscriptions(request_body.into()).await;

    assert_eq!(hyper::StatusCode::OK, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    let test_app = spawn_app().await;
    let request_body = "name=serj&email=serj%40rodrigess.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(request_body.into()).await;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "serj@rodrigess.com");
    assert_eq!(saved.name, "serj");
    assert_eq!(saved.status, "pending_confirmation");
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

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    let test_app = spawn_app().await;
    let body = "name=serj&email=serj%40rodrigess.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.into()).await;
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let app = spawn_app().await;
    let body = "name=serj&email=serj%40rodrigess.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;
    app.post_subscriptions(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);

    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}

#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=serj&email=serj%40rodrigess.com";
    // Sabotage the database
    sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN subscription_token;",)
        .execute(&app.db_pool)
        .await
        .unwrap();
    // Act
    let response = app.post_subscriptions(body.into()).await;
    // Assert
    assert_eq!(response.status().as_u16(), 500);
}
