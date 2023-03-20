use std::net::SocketAddr;
use std::time::Duration;
use secrecy::ExposeSecret;
use sqlx::postgres::{PgPool, PgPoolOptions};

use zero2prod::startup;
use zero2prod::configuration::get_configuration;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> Result<(), std::io::Error>{
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    
    let configuration = get_configuration().expect("Failed to read configuration file.");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        //.connect_timeout(Duration::from_secs(3))
        .connect(&configuration.database.connection_string().expose_secret())
        .await
        .expect("can't connect to database");

    let addr = SocketAddr::from(([127, 0, 0, 1], configuration.application_port));


    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(startup::app(pool).await.into_make_service())
        .await
        .unwrap();
    Ok(())
}
