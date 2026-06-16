use std::time::Duration;

use axum::http::StatusCode;
use axum::{
    http::Uri,
    routing::{delete, get, post, put},
    Json, Router,
};

use inventory_management_tool::middleware::auth_middleware;
use serde::Serialize;
mod middleware;
mod routes;
mod types;
use crate::routes::{
    add_new_item, archive_item_by_id, delete_vendor, get_item_by_id, get_items_by_id,
    get_skus_by_id, get_vendor_by_id, get_vendors, login_handler, post_csv_items, post_csv_vendors,
    put_item_by_id, put_vendor, set_sku_by_id, signup_handler,
};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
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

    // Consider instead setting
    let result = sqlx::migrate!("./migrations").run(&pool).await;
    match result {
        Ok(o) => {
            println!("Migrations worked {:?}", o);
        }
        Err(e) => {
            println!("Migrations failed {:?}", e);
        }
    }

    let state = AppState { pool };

    // build our application with a single route
    let app = Router::new()
        .route("/", get(home))
        .route("/api/health", get(health_check))
        .route("/login", post(login_handler))
        .route("/signup", post(signup_handler))
        .route("/vendor", get(get_vendors))
        .route("/vendor/{vendor_id}", delete(delete_vendor))
        .route("/vendor/{vendor_id}", put(put_vendor))
        .route("/vendor/{vendor_id}", get(get_vendor_by_id))
        .route("/vendor", post(post_csv_vendors))
        .route("/vendor/{vendor_id}/item/{item_id}", get(get_item_by_id))
        .route("/vendor/{vendor_id}/item", get(get_items_by_id))
        .route("/vendor/{vendor_id}/sku", get(get_skus_by_id))
        .route(
            "/vendor/{vendor_id}/item/{item_id}/sku",
            post(set_sku_by_id),
        )
        .route(
            "/vendor/{vendor_id}/item/{item_id}/archive",
            post(archive_item_by_id),
        )
        .route("/vendor/{vendor_id}/item/{item_id}", post(put_item_by_id))
        .route("/vendor/{vendor}/item/new", post(add_new_item))
        .route("/vendor/{vendor_id}/item", post(post_csv_items))
        .route("/vendor/add", post(post_csv_vendors))
        .fallback(fallback)
        .route_layer(axum::middleware::from_fn(auth_middleware))
        .with_state(state.pool);

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

async fn fallback(uri: Uri) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, format!("No route for {uri}"))
}
