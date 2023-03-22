use hyper;
use once_cell::sync::Lazy;
use pretty_assertions::assert_eq;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;

use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::startup::app;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

// Ensure that the `tracing` stack is only initialised once using `once_cell`
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    // We cannot assign the output of `get_subscriber` to a variable based on the
    // value TEST_LOG` because the sink is part of the type returned by
    // `get_subscriber`, therefore they are not the same type. We could work around
    // it, but this is the most straight-forward way of moving forward.
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

pub struct TestApp {
    pub address: String,
    pub db_pool: sqlx::postgres::PgPool,
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres ");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    // Migrate database
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres.");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}

async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    Lazy::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port");
    let addr = listener.local_addr().unwrap();

    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.database.database_name = uuid::Uuid::new_v4().to_string();
    let connection_pool = configure_database(&configuration.database).await;

    let app_instance = app(connection_pool.clone()).await;

    tokio::spawn(async move {
        hyper::Server::from_tcp(listener)
            .unwrap()
            .serve(app_instance.into_make_service())
            .await
            .unwrap();
    });

    TestApp {
        address: addr.to_string(),
        db_pool: connection_pool,
    }
}

#[tokio::test]
async fn get_health_check_returns_200() {
    let test_app = spawn_app().await;

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
