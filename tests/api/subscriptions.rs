use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let mut app = spawn_app().await;

    // Act
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/v3.1/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = app.post_subscriptions(body.into()).await;

    //Assert
    assert_eq!(200, response.status().as_u16());

    app.drop().await
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    let mut app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/v3.1/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "pending_confirmation");

    app.drop().await;
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    // Arrange
    let mut app = spawn_app().await;

    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_la_guin%40@gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = app.post_subscriptions(invalid_body.into()).await;

        //Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }

    app.drop().await
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty() {
    let mut app = spawn_app().await;

    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "name is empty"),
        ("name=Ursula&email=", "email is empty"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        let response = app.post_subscriptions(body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 BAD RESPONSE when the payload was {}.",
            description
        );
    }

    app.drop().await
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    let mut app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/v3.1/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    // Mock Asserts on drop

    app.drop().await;
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let mut app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/v3.1/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_link(email_request).await;

    assert_eq!(confirmation_links.html, confirmation_links.plain_text);

    app.drop().await;
}

#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    let mut app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    sqlx::query!("ALTER TABLE subscriptions DROP COLUMN email;")
        .execute(&app.db_pool)
        .await
        .expect("Failed to drop the subscriptions table");

    let response = app.post_subscriptions(body.into()).await;

    assert_eq!(response.status().as_u16(), 500);

    app.drop().await;
}
