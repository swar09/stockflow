use std::time::Duration;

use axum::http::StatusCode;
use axum::{
    http::Uri,
    routing::{get, post, put},
    Json, Router,
};

use serde::Serialize;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod middleware;
mod routes;
mod types;
use crate::middleware::auth_middleware;
use crate::routes::{
    add_new_item, archive_item_by_id, delete_api_key, delete_cat_by_id, delete_vendor,
    get_api_key, get_cat_by_id, get_cats_by_id, get_item_by_id, get_items_by_id, get_skus_by_id,
    get_stock_record_by_id, get_variant_by_id, get_vendor_by_id, get_vendors, login_handler,
    post_csv_items, post_csv_vendors, put_cat_by_id, put_item_by_id, put_variant_by_id, put_vendor,
    set_sku_by_id, signup_handler, update_stock_by_id,
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
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=debug,sqlx=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Initializing Stockflow...");

    let database_url = dotenv::var("DATABASE_URL").expect("Unable to access .env file");

    tracing::info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_url)
        .await
        .expect("Failed to connect to the database");

    tracing::info!("Running database migrations...");
    let result = sqlx::migrate!("./migrations").run(&pool).await;
    match result {
        Ok(o) => {
            tracing::info!("Migrations worked successfully {:?}", o);
        }
        Err(e) => {
            tracing::error!("Migrations failed: {:?}", e);
        }
    }

    let state = AppState { pool };

    // Routes requiring authentication
    let api_routes = Router::new()
        .route("/vendor", get(get_vendors).post(post_csv_vendors))
        .route(
            "/vendor/{vendor_id}",
            get(get_vendor_by_id).put(put_vendor).delete(delete_vendor),
        )
        .route(
            "/vendor/{vendor_id}/item",
            get(get_items_by_id).post(post_csv_items),
        )
        .route(
            "/vendor/{vendor_id}/item/{item_id}",
            get(get_item_by_id).post(put_item_by_id),
        )
        .route("/vendor/{vendor_id}/sku", get(get_skus_by_id))
        .route("/vendor/{vendor_id}/item/{item_id}/sku", post(set_sku_by_id))
        .route("/vendor/{vendor_id}/item/{item_id}/archive", post(archive_item_by_id))
        .route("/vendor/{vendor_id}/item/new", post(add_new_item))
        .route("/vendor/{vendor_id}/item/{item_id}/variant", get(get_variant_by_id))
        .route("/vendor/{vendor_id}/item/{item_id}/variant/{variant_id}", put(put_variant_by_id))
        .route("/vendor/{vendor_id}/item/{item_id}/category", get(get_cats_by_id))
        .route(
            "/vendor/{vendor_id}/item/{item_id}/category/{category_id}",
            get(get_cat_by_id).put(put_cat_by_id).delete(delete_cat_by_id),
        )
        .route(
            "/vendor/{vendor_id}/item/{item_id}/stock",
            get(get_stock_record_by_id).post(update_stock_by_id),
        )
        .route("/api-key", post(get_api_key).delete(delete_api_key))
        .route_layer(axum::middleware::from_fn(auth_middleware));

    // Public routes and merging with private routes
    let app = Router::new()
        .route("/", get(home))
        .route("/api/health", get(health_check))
        .route("/login", post(login_handler))
        .route("/signup", post(signup_handler))
        .merge(api_routes)
        .fallback(fallback)
        .layer(TraceLayer::new_for_http())
        .with_state(state.pool);

    let addr = "0.0.0.0:3000";
    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
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
        message: String::from("Welcome to Stockflow"),
    };
    Json(response)
}

async fn fallback(uri: Uri) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, format!("No route for {uri}"))
}
