use zero2prod::configuration::get_configuration;
use zero2prod::startup;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration file.");

    let application = startup::Application::build(configuration).await?;
    match application.run_until_stopped().await {
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        Ok(_) => Ok(()),
    }
}
