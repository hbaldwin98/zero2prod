use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let mut app = spawn_app().await;

    // Act
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = app.post_subscriptions(body.into()).await;

    //Assert
    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");

    app.drop().await
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