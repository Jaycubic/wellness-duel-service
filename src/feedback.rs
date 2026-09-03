use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::state::AppState;

pub async fn submit_feedback(
    state: web::Data<AppState>,
    body: web::Json<SubmitFeedbackReq>,
) -> AppResult<HttpResponse> {
    let name = body.name.trim().to_string();
    if name.is_empty() || name.len() > 50 {
        return Err(AppError::BadRequest(
            "name must be between 1 and 50 characters".into(),
        ));
    }

    let rating = body.rating;
    if !(1..=5).contains(&rating) {
        return Err(AppError::BadRequest(
            "rating must be between 1 and 5".into(),
        ));
    }

    let message = body.message.trim().to_string();
    if message.is_empty() || message.len() > 1000 {
        return Err(AppError::BadRequest(
            "message must be between 1 and 1000 characters".into(),
        ));
    }

    let id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO feedback (id, name, rating, message) VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(&name)
    .bind(rating)
    .bind(&message)
    .execute(&state.pool)
    .await?;

    let row = sqlx::query_as::<_, FeedbackRow>(
        "SELECT id, name, rating, message, created_at FROM feedback WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;

    Ok(HttpResponse::Ok().json(FeedbackView {
        id: row.id,
        name: row.name,
        rating: row.rating,
        message: row.message,
        created_at: row.created_at,
    }))
}

pub async fn list_feedback(
    state: web::Data<AppState>,
) -> AppResult<HttpResponse> {
    let rows = fetch_all_feedback(&state.pool).await?;

    let views: Vec<FeedbackView> = rows
        .into_iter()
        .map(|r| FeedbackView {
            id: r.id,
            name: r.name,
            rating: r.rating,
            message: r.message,
            created_at: r.created_at,
        })
        .collect();

    Ok(HttpResponse::Ok().json(views))
}

async fn fetch_all_feedback(pool: &PgPool) -> AppResult<Vec<FeedbackRow>> {
    let rows = sqlx::query_as::<_, FeedbackRow>(
        "SELECT id, name, rating, message, created_at FROM feedback ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
