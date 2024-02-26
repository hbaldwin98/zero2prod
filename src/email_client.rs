use base64::{
    engine::general_purpose::{self},
    Engine,
};
use reqwest::Client;
use secrecy::{ExposeSecret, Secret};

use crate::domain::SubscriberEmail;

pub struct EmailClient {
    http_client: Client,
    base_url: String,
    sender: SubscriberEmail,
    api_key_public: Secret<String>,
    api_key_private: Secret<String>,
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        api_key_public: Secret<String>,
        api_key_private: Secret<String>,
        timeout: std::time::Duration,
    ) -> Self {
        Self {
            http_client: Client::builder().timeout(timeout).build().unwrap(),
            base_url,
            sender,
            api_key_public,
            api_key_private,
        }
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_part: &str,
        text_part: &str,
    ) -> Result<(), reqwest::Error> {
        let url = format!("{}/v3.1/send", self.base_url);
        let request_body = SendEmailRequest {
            from: SendEmailFrom {
                email: self.sender.as_ref(),
                name: self.sender.as_ref(),
            },
            to: vec![SendEmailTo {
                email: recipient.as_ref(),
                name: recipient.as_ref(),
            }],
            subject,
            text_part,
            html_part,
        };

        let credentials: String = general_purpose::STANDARD.encode(format!(
            "{}:{}",
            self.api_key_public.expose_secret(),
            self.api_key_private.expose_secret()
        ));

        let auth_header_value = format!("Basic {}", credentials);

        self.http_client
            .post(&url)
            .header("Authorization", auth_header_value)
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: SendEmailFrom<'a>,
    to: Vec<SendEmailTo<'a>>,
    subject: &'a str,
    text_part: &'a str,
    html_part: &'a str,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailFrom<'a> {
    email: &'a str,
    name: &'a str,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailTo<'a> {
    email: &'a str,
    name: &'a str,
}

#[cfg(test)]
mod tests {
    use claims::{assert_err, assert_ok};
    use fake::{
        faker::{
            internet::en::SafeEmail,
            lorem::{en::Paragraph, en::Sentence},
        },
        Fake, Faker,
    };
    use secrecy::Secret;
    use wiremock::{
        matchers::{any, header, header_exists, method, path},
        Mock, MockServer, Request, ResponseTemplate,
    };

    use crate::{domain::SubscriberEmail, email_client::EmailClient};

    struct SendEmailBodyMatcher;
    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("TextPart").is_some()
                    && body.get("HtmlPart").is_some()
                    && body.get("To").unwrap().as_array().is_some()
                    && body.get("To").unwrap().as_array().unwrap().len() == 1
                    && body.get("To").unwrap().as_array().unwrap()[0]
                        .get("Email")
                        .is_some()
                    && body.get("To").unwrap().as_array().unwrap()[0]
                        .get("Name")
                        .is_some()
                    && body.get("From").unwrap().get("Email").is_some()
                    && body.get("From").unwrap().get("Name").is_some()
            } else {
                false
            }
        }
    }

    fn subject() -> String {
        Sentence(1..2).fake()
    }

    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn email_client(base_url: String) -> EmailClient {
        EmailClient::new(
            base_url,
            email(),
            Secret::new(Faker.fake()),
            Secret::new(Faker.fake()),
            std::time::Duration::from_millis(200),
        )
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_a_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        let response = ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(180));

        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        // ARRANGE
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("Authorization"))
            .and(header("Content-Type", "application/json"))
            .and(path("/v3.1/send"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        // ACT
        let _ = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // ASSERT (the Mock::given will assert at the end)
    }
}
