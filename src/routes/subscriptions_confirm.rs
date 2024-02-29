use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscription", skip(parameters, pool))]
pub async fn confirm(parameters: web::Query<Parameters>, pool: web::Data<PgPool>) -> HttpResponse {
    let id = match get_subscriber_id_from_token(&pool, &parameters.subscription_token).await {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    match id {
        None => HttpResponse::BadRequest().finish(),
        Some(subscriber_id) => {
            if confirm_subscriber(&pool, subscriber_id).await.is_ok() {
                HttpResponse::Ok().finish()
            } else {
                HttpResponse::InternalServerError().finish()
            }
        }
    }
}

#[tracing::instrument(name = "Mark a subscriber as confirmed", skip(subscriber_id, pool))]
pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!(
            "Failed to update status of subscriber {}: {:?}",
            subscriber_id,
            e
        );
        e
    })?;

    Ok(())
}

#[tracing::instrument(
    name = "Retrieving subscriber ID by token",
    skip(pool, subscription_token)
)]
pub async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        subscription_token,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to retrieve subscriber ID: {:?}", e);
        e
    })?;

    Ok(result.map(|r| r.subscriber_id))
}
