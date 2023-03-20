use std::net::SocketAddr;
use std::time::Duration;
use sqlx::postgres::{PgPool, PgPoolOptions};

use zero2prod::startup;
use zero2prod::configuration::get_configuration;

#[tokio::main]
async fn main() -> Result<(), std::io::Error>{
    
    let configuration = get_configuration().expect("Failed to read configuration file.");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        //.connect_timeout(Duration::from_secs(3))
        .connect(&configuration.database.connection_string())
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
