use actix_web::{App, HttpServer, web};
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
}

#[derive(Debug, Deserialize)]
struct CreateTransactionRequest {
    name: String,
    amount: f64,
    email: String,
    description: String,
}

struct AppState {
    db: PgPool,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let database_url = "posgres://postgres:postgres@localhost/transactions";
    let pool = PgPool::connect(database_url)
        .await
        .expect("Failed connecting to db");
    let app_state = web::Data::new(AppState { db: pool});

    println!("Starting Idempotency Transaction checker API on localhost port 8080");

    HttpServer::new(move || {
        App::new()
        .app_data(app_state.clone())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
