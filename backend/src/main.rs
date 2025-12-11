use actix_web::{App, HttpResponse, HttpServer, Result, get, post, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, prelude::FromRow};
use uuid::Uuid;
use md5;

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct Payment {
    id: Uuid,
    user_session_id: Uuid,
    name: String,
    amount: f64,
    email: String,
    description: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CreatePaymentRequest {
    name: String,
    amount: f64,
    email: String,
    description: String,
}

#[get("/payments")]
async fn get_payments(state: web::Data<AppState>) -> Result<HttpResponse> {
    let payments = sqlx::query_as::<_, Payment>(
        "SELECT * FROM transactions ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        eprintln!("Failed to fetch payments: {}", e);
    actix_web::error::ErrorInternalServerError("Failed to fetch payments")
    })?;

    Ok(HttpResponse::Ok().json(payments))
}

#[post("/payments")]
async fn create_payment(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    body: web::Json<CreatePaymentRequest>,
) -> Result<HttpResponse> {
    // Extract idem P key
    let idempotency_key = req
        .headers()
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(actix_web::error::ErrorBadRequest("Missing header Idempotency-Key"))?;

    // 2. Hash the request body
    let body_json = serde_json::to_string(&body)?;
    let request_hash = format!("{:x}", md5::compute(&body_json));

    // 3. Check existing idempotency record
    if let Some(existing) = sqlx::query!(
        r#"
        SELECT 
            request_hash, 
            response_body AS "response_body: sqlx::types::Json<serde_json::Value>"
        FROM idempotency_keys 
        WHERE idempotency_key = $1
        "#,
        idempotency_key
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        eprintln!("Idempotency lookup failed: {}", e);
        actix_web::error::ErrorInternalServerError("Failed checking idempotency key")
    })?
    {
        if existing.request_hash != request_hash {
            return Err(actix_web::error::ErrorConflict("Request body does not match previous request for this key"));
        }

        if let Some(resp) = existing.response_body {
            return Ok(HttpResponse::Ok()
                .content_type("application/json")
                .body(resp.to_string()));
        }

        return Err(actix_web::error::ErrorTooManyRequests("Request is still being processed"));
    }

    // 4. Store "processing" record
    let id = Uuid::new_v4();
    let user_session_id = Uuid::new_v4();

    // 5. Create payment
    sqlx::query!(
        "INSERT INTO idempotency_keys (
            idempotency_key, 
            user_session_id,
            request_hash
        )
        VALUES ($1, $2, $3)
        ",
        idempotency_key,
        user_session_id,
        request_hash
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        eprintln!("Failed inserting idempotency record: {}", e);
        actix_web::error::ErrorInternalServerError("Failed inserting idempotency key")
    })?;

    let payment = sqlx::query_as::<_, Payment>(
        r#"
        INSERT INTO transactions (
            id,
            user_session_id,
            name,
            amount,
            email,
            description
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#
    )
    .bind(id)
    .bind(user_session_id)
    .bind(&body.name)
    .bind(body.amount)
    .bind(&body.email)
    .bind(&body.description)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        eprintln!("Failed to create payment: {}", e);
        actix_web::error::ErrorInternalServerError("Failed to create payment")
    })?;

    // Cache the result
    let response_json = sqlx::types::Json(serde_json::to_value(&payment)?);
    sqlx::query!(
        r#"
        UPDATE idempotency_keys 
        SET response_body = $1, updated_at = NOW() 
        WHERE idempotency_key = $2
        "#,
        response_json.0,
        idempotency_key
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        eprintln!("Database error: {}", e);
        actix_web::error::ErrorInternalServerError("DB error")
    })?;

    Ok(HttpResponse::Created().json(payment))
}

struct AppState {
    db: PgPool,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // export DATABASE_URL="postgres://postgres:postgres@localhost/transactions"
    // psql -h localhost -p 5432 -U postgres -d transactions
    let database_url = "postgres://postgres:postgres@localhost/transactions";

    let pool = PgPool::connect(database_url)
        .await
        .expect("Failed connecting to db");
    let app_state = web::Data::new(AppState { db: pool});

    println!("Starting Idempotency Transaction checker API on localhost port 8080");

    HttpServer::new(move || {
        App::new()
        .app_data(app_state.clone())
        .service(get_payments)
        .service(create_payment)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
