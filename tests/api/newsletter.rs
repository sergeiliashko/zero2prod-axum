use crate::helpers::{assert_is_redirect_to, spawn_app, ConfirmationLinks, TestApp};
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn non_existing_user_is_rejected() {
    let app = spawn_app().await;

    let response = app.get_send_newsletter().await;

    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    // Arrange
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    });

    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    let test_cases = vec![
        (
            serde_json::json!({
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            }),
            "missing title",
        ),
        (
            serde_json::json!({"title": "Newsletter!"}),
            "missing content",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_newsletters(&invalid_body).await;
        // Assert
        assert_eq!(
            422,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    });

    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        // We assert that no request is fired at Postmark!
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text": "Newsletter body as plain text",
        "html": "<p>Newsletter body as HTML</p>",
    });

    let response = app.post_newsletters(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletter");

    let html_page = app.get_newsletter_html().await;
    assert!(html_page.contains("<p><i>Newsletter was sent successfully.</i></p>"));
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;

    create_confirmed_subscriber(&app).await;

    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    });

    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text": "Newsletter body as plain text",
        "html": "<p>Newsletter body as HTML</p>",
    });

    let response = app.post_newsletters(&newsletter_request_body).await;

    assert_is_redirect_to(&response, "/admin/newsletter");

    let html_page = app.get_newsletter_html().await;
    assert!(html_page.contains("<p><i>Newsletter was sent successfully.</i></p>"));
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=serj&email=serj%40rodrigess.com";
    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();

    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    app.get_confirmation_links(&email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    // We can then reuse the same helper and just add
    // an extra step to actually call the confirmation link!
    let confirmation_link = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

//#[tokio::test]
//async fn invalid_password_is_rejected() {
//    let app = spawn_app().await;
//    let username = &app.test_user.username;
//
//    let password = uuid::Uuid::new_v4().to_string();
//    assert_ne!(app.test_user.password, password);
//
//    let response = reqwest::Client::new()
//        .post(&format!("{}/newsletters", &app.address))
//        .basic_auth(username, Some(password))
//        .json(&serde_json::json!({
//                "title": "Newsletter title",
//                "content": {
//                    "text": "Newsletter body as plain text",
//                    "html": "<p>Newsletter body as HTML</p>",
//                }
//        }))
//        .send()
//        .await
//        .expect("Failed to execute request.");
//
//    assert_eq!(401, response.status().as_u16());
//    assert_eq!(
//        r#"Basic realm="publish""#,
//        response.headers()["WWW-Authenticate"]
//    );
//}

//#[tokio::test]
//async fn requests_missing_authorization_are_rejected() {
//    let app = spawn_app().await;
//
//    let response = reqwest::Client::new()
//        .post(&format!("{}/newsletters", &app.address))
//        .json(&serde_json::json!({
//            "title": "Newsletter title",
//            "content": {
//                "text": "Newsletter body as plain text",
//                "html": "<p>Newsletter body as HTML</p>",
//            }
//        }))
//        .send()
//        .await
//        .expect("Failed to execute request.");
//
//    assert_eq!(401, response.status().as_u16());
//    assert_eq!(
//        r#"Basic realm="publish""#,
//        response.headers()["WWW-Authenticate"]
//    );
//}
