mod logic;
mod timers;

use std::fmt::Display;

use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::*,
    Json,
};
use utoipa::OpenApi;
use utoipa_redoc::Servable as RedocServable;
use utoipa_scalar::Servable as ScalarServable;

const BIND_ADDR: &str = "0.0.0.0:80";

#[derive(utoipa::OpenApi)]
#[openapi(info(
    title = "Crab Emotion API",
    version = "0.1.0",
    description = "Make the crab feel things"
))]
struct ApiDoc;

#[derive(Debug, utoipa::ToSchema, serde::Deserialize)]
struct ApiEmotionMessage {
    emotion: logic::Emotion,
}

impl Display for ApiEmotionMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.emotion)
    }
}

#[utoipa::path(post,
    path = "/crab/emotion",
    summary = "Crab Emotion API",
    request_body = ApiEmotionMessage,
    responses(
        (status = 200, description = "Success!", body = ())
))]
async fn post_emotion(Json(payload): Json<ApiEmotionMessage>) -> impl IntoResponse {
    // todo!("add call to crab code");
    (
        StatusCode::OK,
        "oh crab: ".to_string() + &payload.to_string(),
    )
}

async fn root() -> impl IntoResponse {
    Html(
        r#"
        <!DOCTYPE html>
        <html>
            <head>
                <title>Crab Emotion API</title>
            </head>
            <body>
                <h1>Crab Emotion API</h1>
                <p>POST /crab/emotion</p>
                <p>GET /swagger-ui</p>
            </body>
        </html>
        "#,
    )
}

fn app() -> axum::Router {
    let routes: utoipa_axum::router::UtoipaMethodRouter = utoipa_axum::routes!(post_emotion);
    let (router, api): (axum::Router, utoipa::openapi::OpenApi) =
        utoipa_axum::router::OpenApiRouter::with_openapi(ApiDoc::openapi())
            .routes(routes)
            .route("/", get(root))
            .split_for_parts();

    let router = router
        .merge(
            utoipa_swagger_ui::SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", api.clone()),
        )
        .merge(utoipa_redoc::Redoc::with_url("/redoc", api.clone()))
        .merge(utoipa_rapidoc::RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
        .merge(utoipa_scalar::Scalar::with_url("/scalar", api));

    router
}

#[tokio::main]
pub async fn run_http_server() {
    let app = app();
    let listener = tokio::net::TcpListener::bind(BIND_ADDR).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

fn main() {
    run_http_server();
}