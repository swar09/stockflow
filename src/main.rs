use std::time::Duration;

use axum::{
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
    get_skus_by_id, get_vendor_by_id, get_vendors, login_handler, put_item_by_id, put_vendor,
    set_sku_by_id, signup_handler,
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
        .route("/vendor/{vendor_id}", get(get_vendor_by_id))
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
        .route("/vendor/{vendor_id}", delete(delete_vendor))
        .route("/vendor", get(get_vendors))
        .route("/vendor/{vendor_id}", put(put_vendor))
        .route_layer(axum::middleware::from_fn(auth_middleware))
        .with_state(state.pool);
    /*

    DELETE /vendor/{id}/item/{id} system-must-not-allow-nonzero-stock-item-deletion, archive instead

    POST /vendor/{id}/item/XXXX add-items-by-config-file-CSV
    PUT  /vendor/{id}/item/XXXX update-items-by-config-file-CSV
    POST /vendor/{id}/item/{id}/variant add-variants-for-item , if vriant!=0 , has_variant=1

    fn which will verify the jwt and return the user_id , vendor_id

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
