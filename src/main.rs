use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

use zero2prod::configuration::get_configuration;
use zero2prod::email_client::EmailClient;
use zero2prod::startup;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration file.");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(3))
        .connect_lazy_with(configuration.database.without_db());
    //.expect("can't connect to database");
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
    let ipadr = std::net::IpAddr::from_str(&configuration.application.host)
        .expect("Failed to parse app host from config");
    //let addr = SocketAddr::from((ipadr, configuration.application.port));

    let port: u16 = match std::env::var("PORT") {
        Ok(port) => port.parse().expect("expect to get existing yandex port"),
        Err(_) => configuration.application.port,
    };
    //let addr = SocketAddr::new(ipadr, configuration.application.port);
    let addr = SocketAddr::new(ipadr, port);

    println!("Try to bind to - {}", &addr);

    axum::Server::bind(&addr)
        .serve(startup::app(pool, email_client).await.into_make_service())
        .await
        .unwrap();
    Ok(())
}
