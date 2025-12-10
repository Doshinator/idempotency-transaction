use actix_web::{App, HttpResponse, HttpServer, Result, get, post, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, prelude::FromRow};
use uuid::Uuid;

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

#[derive(Debug, Deserialize)]
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
    body: web::Json<CreatePaymentRequest>,
) -> Result<HttpResponse> {
    let id = Uuid::new_v4();
    let user_session_id = Uuid::new_v4();

    let query = sqlx::query_as::<_, Payment>(
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

    Ok(HttpResponse::Created().json(query))
}

struct AppState {
    db: PgPool,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
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
