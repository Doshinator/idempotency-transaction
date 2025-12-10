use actix_web::{App, HttpResponse, HttpServer, Result, get, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, prelude::FromRow};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct Transaction {
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
struct CreateTransactionRequest {
    name: String,
    amount: f64,
    email: String,
    description: String,
}

#[get("/transactions")]
async fn get_transactions(state: web::Data<AppState>) -> Result<HttpResponse> {
    let transactions = sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        eprintln!("Failed to fetch transactions: {}", e);
    actix_web::error::ErrorInternalServerError("Failed to fetch transactions")
    })?;

    Ok(HttpResponse::Ok().json(transactions))
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
        .service(get_transactions)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
