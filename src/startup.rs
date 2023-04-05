use async_fred_session::RedisSessionStore;
use axum_sessions::SessionLayer;

use axum::{
    body::{Body, BoxBody},
    http::{HeaderName, Request, Response},
    routing::{get, post},
    Router,
};
use axum_extra::extract::cookie::Key;
use fred::{types::RedisConfig, pool::RedisPool};
use secrecy::ExposeSecret;
use std::time::Duration;
use tracing::Span;

use tower_http::cors::{Any, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;

use crate::{
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    routes, authentication::reject_anonymous_users,
};

#[derive(Clone)]
pub struct ApplicationBaseUrl(pub String);

#[derive(Clone)]
pub struct HmacSecret(pub secrecy::Secret<String>);
// In axum, we have only one state type
#[derive(Clone)]
struct AppState {
    email_client: EmailClient,
    connection_pool: sqlx::PgPool,
    base_url: ApplicationBaseUrl,
    hmac_secret: Key,
}

impl axum::extract::FromRef<AppState> for ApplicationBaseUrl {
    fn from_ref(app_state: &AppState) -> ApplicationBaseUrl {
        app_state.base_url.clone()
    }
}
impl axum::extract::FromRef<AppState> for axum_extra::extract::cookie::Key {
    fn from_ref(app_state: &AppState) -> axum_extra::extract::cookie::Key {
        app_state.hmac_secret.clone()
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
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
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
            configuration.application.hmac_secret,
            configuration.redis_uri,
        )
        .await?;

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
    hmac_secret: secrecy::Secret<String>,
    redis_uri: secrecy::Secret<String>
) -> Result<Router, anyhow::Error> {

    let cfg = RedisConfig::from_url(redis_uri.expose_secret()).unwrap();
    let rds_pool = RedisPool::new(cfg, 6).unwrap();
    rds_pool.connect(None);
    rds_pool.wait_for_connect().await.unwrap();

    let redis_session_store = RedisSessionStore::from_pool(rds_pool, Some("zero2prod-sessions/".into()));
    let session_layer = SessionLayer::new(redis_session_store, hmac_secret.expose_secret().as_bytes()).with_secure(false);


    let x_request_id = HeaderName::from_static("x-request-id");
    let state = AppState {
        email_client,
        connection_pool,
        base_url: ApplicationBaseUrl(base_url),
        hmac_secret: Key::from(hmac_secret.expose_secret().as_bytes()),
    };

    let router = Router::new()
        .route("/health_check", get(routes::healt_check))
        .route("/subscriptions", post(routes::subscribe))
        .route("/subscriptions/confirm", get(routes::confirm))
        .route("/newsletters", post(routes::publish_newsletter))
        .route("/", get(routes::home))
        .route("/login", get(routes::login_form).post(routes::login))
        .merge(
            Router::new()
                .nest("/admin", 
                    Router::new()
                        .route("/dashboard", get(routes::admin_dashboard))
                        .route("/password", get(routes::change_password_form).post(routes::change_password))
                        .route("/logout", post(routes::log_out))
                        .layer(axum::middleware::from_fn(reject_anonymous_users))
                ))
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
        .layer(session_layer)
        //.propagate_x_request_id())
        //.with_state(connection_pool)
        .with_state(state);
    //.with_state(email_client)
    Ok(router)
}
