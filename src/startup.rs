use axum::{
    body::{Body, BoxBody},
    http::{HeaderName, Request, Response},
    routing::{get, post},
    Router,
};
use std::time::Duration;
use tracing::Span;

use tower_http::cors::{Any, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;

use crate::{
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    routes,
};

#[derive(Clone)]
pub struct ApplicationBaseUrl(pub String);

// In axum, we have only one state type
#[derive(Clone)]
struct AppState {
    email_client: EmailClient,
    connection_pool: sqlx::PgPool,
    base_url: ApplicationBaseUrl,
}

impl axum::extract::FromRef<AppState> for ApplicationBaseUrl {
    fn from_ref(app_state: &AppState) -> ApplicationBaseUrl {
        app_state.base_url.clone()
    }
}
impl axum::extract::FromRef<AppState> for EmailClient {
    fn from_ref(app_state: &AppState) -> EmailClient {
        app_state.email_client.clone()
    }
}
impl axum::extract::FromRef<AppState> for sqlx::PgPool {
    fn from_ref(app_state: &AppState) -> sqlx::PgPool {
        app_state.connection_pool.clone()
    }
}

pub struct Application {
    tcp_listener: std::net::TcpListener,
    app: Router,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, std::io::Error> {
        let connection_pool = get_connection_pool(&configuration.database);

        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address.");

        let timeout = configuration.email_client.timeout();

        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.authorization_token,
            timeout,
        );
        let app = app(
            connection_pool,
            email_client,
            configuration.application.base_url,
        )
        .await;

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let tcp_listener = std::net::TcpListener::bind(address)?;

        Ok(Self { tcp_listener, app })
    }

    pub fn port(&self) -> u16 {
        self.tcp_listener.local_addr().unwrap().port()
    }

    pub fn address(&self) -> String {
        format!("{}", self.tcp_listener.local_addr().unwrap())
    }

    // A more expressive name that makes it clear that
    // this function only returns when the application is stopped.
    pub async fn run_until_stopped(self) -> Result<(), hyper::Error> {
        axum::Server::from_tcp(self.tcp_listener)?
            .serve(self.app.into_make_service())
            .await
    }
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> sqlx::postgres::PgPool {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}

pub async fn app(
    connection_pool: sqlx::PgPool,
    email_client: EmailClient,
    base_url: String,
) -> Router {
    let x_request_id = HeaderName::from_static("x-request-id");
    let state = AppState {
        email_client,
        connection_pool,
        base_url: ApplicationBaseUrl(base_url),
    };

    Router::new()
        .route("/health_check", get(routes::healt_check))
        .route("/subscriptions", post(routes::subscribe))
        .route("/subscriptions/confirm", get(routes::confirm))
        .route("/newsletters", post(routes::publish_newsletter))
        .layer(CorsLayer::new().allow_origin(Any))
        .layer(
            // from https://docs.rs/tower-http/0.2.5/tower_http/request_id/index.html#using-trace
            tower::ServiceBuilder::new()
                .layer(SetRequestIdLayer::new(
                    x_request_id.clone(),
                    MakeRequestUuid,
                ))
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(|_request: &Request<Body>| {
                            tracing::info_span!(
                                "http-request",
                                status_code = tracing::field::Empty,
                                request_id = format!("{}", uuid::Uuid::new_v4()),
                            )
                        })
                        .on_response(
                            |response: &Response<BoxBody>, _latency: Duration, span: &Span| {
                                span.record(
                                    "status_code",
                                    &tracing::field::display(response.status()),
                                );
                                tracing::info!("response generated")
                            },
                        ), //.make_span_with(
                           //    DefaultMakeSpan::new()
                           //        .include_headers(true)
                           //        .level(tracing::Level::INFO),
                           //)
                           //.on_response(DefaultOnResponse::new().include_headers(true))
                )
                //.set_x_request_id(MakeRequestUuid)
                .layer(PropagateRequestIdLayer::new(x_request_id)),
        )
        //.propagate_x_request_id())
        //.with_state(connection_pool)
        .with_state(state)
    //.with_state(email_client)
}
