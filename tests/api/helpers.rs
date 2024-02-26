use once_cell::sync::Lazy;
use secrecy::ExposeSecret;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup::{get_connection_pool, Application},
    telemetry::{get_subscriber, init_subscriber},
};

static TRACING: Lazy<()> = Lazy::new(|| {
    let use_log = std::env::var("USE_LOG").unwrap_or("false".into());

    if use_log.to_lowercase() == "true" {
        let subscriber = get_subscriber("test".into(), "debug".into(), std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber("test".into(), "debug".into(), std::io::sink);
        init_subscriber(subscriber);
    }
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub db_name: String,
    pub connection_string: String,
}

impl TestApp {
    pub async fn drop(&mut self) {
        let db_pool = self.db_pool.clone();
        db_pool.close().await;

        _ = PgConnection::connect(&self.connection_string)
            .await
            .expect("Failed to connect to the database")
            .execute(&*format!(r#"DROP DATABASE "{}""#, self.db_name))
            .await;

        tracing::info!("Dropped test database");
    }

    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request")
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration");
        c.database.database_name = Uuid::new_v4().to_string();
        c.application.port = 0;

        c
    };

    configure_database(&configuration.database).await;

    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application");
    let address = format!("http://127.0.0.1:{}", application.port());
    let _ = tokio::spawn(application.run_until_stopped());

    let db_pool = get_connection_pool(&configuration.database);
    let db_name = configuration.database.database_name.clone();
    let connection_string = configuration
        .database
        .connection_string_without_db()
        .expose_secret()
        .clone();

    return TestApp {
        address,
        db_pool,
        db_name,
        connection_string,
    };
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    tracing::info!("Connecting to postgres: {:?}", config.without_db());
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database");

    // MIGRATE DATABASE
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    return connection_pool;
}
