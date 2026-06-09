use std::time::Duration;

use axum::{
    Json, Router,
    routing::{get, post},
};
use serde::Serialize;
mod middleware;
mod routes;
mod types;
use crate::routes::login_handler;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
#[derive(Serialize)]
struct Health {
    check: String,
}

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}
#[derive(Serialize)]
struct ResponseHomePage {
    message: String,
}

#[tokio::main]
async fn main() {
    let database_url = dotenv::var("DATABASE_URL").expect("Unable to acess .env file");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_url)
        .await
        .expect("Failed to connect to the database");

    let state = AppState { pool };

    // build our application with a single route
    let app = Router::new()
        .route("/", get(home))
        .route("/api/health", get(health_check))
        .route("/login", post(login_handler))
        .with_state(state);
    /*

    POST /login  user-login
    POST /vendor create-new-vendor
    GET /vendor/{id} get-vendor
    DELETE /vendor/{id} suspend-vendor
    PUT /vendor/{id} update-vendor-info

    middleware check API key exp?
    admin can list and manage all vendors

    POST /vendor/{id}/item add-new-item
    GET /vendor/{id}/item/{id} get-item-by-id
    PUT /vendor/{id}/item/{id} edit-item-by-id
    GET /vendor/{id}/item/ get-all-item-by-vendor
    POST /vendor/{id}/item/{id}?archive=true archive0-item-by-id , archived-items-will-notbe-shown

    POST /vendor/{id}/item/{id}/sku add-sku-code-to-item , there-is-no-same-skucode-in-entire-vendors-workspace
    GET /vendor/{id}/sku get-all-sku-by-vendor
    GET /vendor/{id}/item?x= filter-items-by-x where x-can-be-any-possible-or-multiple-filters

    DELETE /vendor/{id}/item/{id} system-must-not-allow-nonzero-stock-item-deletion, archive instead

    POST /vendor/{id}/item/XXXX add-items-by-config-file-CSV
    PUT  /vendor/{id}/item/XXXX update-items-by-config-file-CSV
    POST /vendor/{id}/item/{id}/variant add-variants-for-item , if vriant!=0 , has_variant=1





    */

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> Json<Health> {
    let heal = Health {
        check: String::from("ok"),
    };
    Json(heal)
}

async fn home() -> Json<ResponseHomePage> {
    let response = ResponseHomePage {
        message: String::from("Welcome to Inventory Management Tool"),
    };
    Json(response)
}
