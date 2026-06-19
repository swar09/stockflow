# Stockflow

Stockflow is an inventory management tool built for vendors. Designed to support high-throughput stock tracking, real-time catalog management, and multi-vendor segregation. The architecture is built to handle the operational scale required by large vendors, supply chains, and enterprise distributors.

## Tech Stack

- **Backend**: Rust using the **Axum** framework. Axum is a web framework built on the **Tokio** runtime, providing high-performance, asynchronous routing and memory safety. This repository serves as a practical guide for implementing CRUD services and RESTful APIs in Rust.
- **Database**: **PostgreSQL** relational database.
- **SQLx**: An async, compile-time query-validated SQL library for Rust. SQLx verifies that raw SQL queries are syntactically and structurally correct against the database schema during compilation.
- **Authentication**: Secured via **JSON Web Tokens (JWT)**. JWTs are generated using the `jsonwebtoken` crate signed with HMAC-SHA256 (`HS256`), carrying user claims, roles, and vendor scopes. These are verified by a custom Axum routing middleware.
- **Password Hashing**: Stored securely using the **Argon2id** algorithm, a state-of-the-art memory-hard password hashing function designed to prevent GPU brute-force and side-channel attacks.

---

## Setup & Running

### 1. Using Docker Compose
Runs both the PostgreSQL database and the Rust backend application inside containers.

1. Copy the example configuration:
   ```bash
   cp .env.example .env
   ```
2. Edit `.env` to supply secret values (e.g., custom user, passwords, and JWT secret).
3. Start the containers:
   ```bash
   docker compose up --build
   ```
4. The service will be available at `http://localhost:3000`.

   *Note: Database migrations located in `./migrations/` are applied automatically on startup.*

### 2. API Testing with Postman
The repository includes a pre-configured Postman collection: `stockflow.postman_collection.json`.

1. Open Postman and click **Import**.
2. Select the [stockflow.postman_collection.json](file:///home/eleven/c/r/backend/inventory-management-tool/stockflow.postman_collection.json) file.
3. Use the `/signup` and `/login` endpoints to authenticate.
4. Copy the returned `bearer` token from the login response.
5. In Postman, configure the Authorization tab to use **Bearer Token** and paste the token to authenticate protected routes.

---

## Code Implementation Overview 

Here is a breakdown of how the REST API and CRUD operations are structured:

### 1. Axum Routing Configuration (`src/main.rs`)
Axum maps HTTP requests to handler functions. Middlewares (like authentication) can be applied to specific route groups:

```rust
// Public routes
let app = Router::new()
    .route("/", get(home))
    .route("/api/health", get(health_check))
    .route("/login", post(login_handler))
    .route("/signup", post(signup_handler))
    // Protected routes merged in
    .merge(api_routes)
    .with_state(state.pool);
```

### 2. Authentication Middleware (`src/middleware.rs`)
Axum uses middleware layers to extract headers, decode JWTs, and inject user claims into the request lifecycle:

```rust
pub async fn auth_middleware(mut req: Request, next: Next) -> Result<Response, StatusCode> {
    let token = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    match verify_jwt(String::from(token)).await {
        (_, Some(user_id), Some(user_role), Some(vendor_id)) => {
            // Inject claims into request extensions for handlers to access
            req.extensions_mut().insert(Claims {
                user: user_id,
                vendor: vendor_id,
                role: user_role,
                exp: 3600000,
            });
            Ok(next.run(req).await)
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
```

### 3. CRUD: Reading Data (SQLx Select)
Handlers extract path variables, database state, and claims. Below is a read request query:

```rust
pub async fn get_vendor_by_id(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(vendor_id): Path<Uuid>,
) -> Result<Json<Vendor>, StatusCode> {
    // Role & tenancy checks
    if claims.role != UserRole::Sys_Admin && claims.vendor != vendor_id.to_string() {
        return Err(StatusCode::FORBIDDEN);
    }
    
    // Fetch a single row mapped directly to a Rust struct
    let result = sqlx::query_as::<_, Vendor>(
        "SELECT id, name, slug, status, email, hstore_to_jsonb(metadata) as metadata, created_at, updated_at 
         FROM vendor WHERE id = $1"
    )
    .bind(vendor_id)
    .fetch_one(&pool)
    .await;

    match result {
        Ok(vendor) => Ok(Json(vendor)),
        Err(sqlx::Error::RowNotFound) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
```

### 4. CRUD: Writing Data (SQLx Insert)
Handlers receive JSON payloads that map to structural models, performing validation and execution:

```rust
pub async fn add_new_item(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Path(vendor_id): Path<Uuid>,
    Json(payload): Json<ItemPayload>,
) -> Result<Json<bool>, StatusCode> {
    // Tenancy authorization
    if claims.role != UserRole::Sys_Admin
        && (claims.role < UserRole::Operator || claims.vendor != vendor_id.to_string())
    {
        return Err(StatusCode::FORBIDDEN);
    }
    
    // Insert statement using SQLx query
    let result = sqlx::query(
        "INSERT INTO item (vendor_id, sku, name, description, status, base_price, currency_code, category_ids, unit_of_measure, tags, attributes, image_urls, has_variants) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, hstore($11::jsonb), $12, $13);"
    )
    .bind(vendor_id)
    .bind(&payload.sku)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(&payload.status)
    .bind(payload.base_price)
    .bind(&payload.currency_code)
    .bind(&payload.catgeory_ids)
    .bind(&payload.uom)
    .bind(&payload.tags)
    .bind(sqlx::types::Json(&payload.attributes))
    .bind(&payload.image_urls)
    .bind(payload.has_variants)
    .execute(&pool)
    .await;

    match result {
        Ok(_) => Ok(Json(true)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
```
