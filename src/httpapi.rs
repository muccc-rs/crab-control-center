use std::{fmt::Display, sync::Arc};

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::*,
    Json,
};
use juniper_axum::{extract::JuniperRequest, response::JuniperResponse, ws};
use tokio::sync::RwLock;
use utoipa::OpenApi;
use utoipa_redoc::Servable as RedocServable;
use utoipa_scalar::Servable as ScalarServable;

use crate::{
    emotionmanager,
    logic::{self, Emotion},
};

const BIND_ADDR: &str = "0.0.0.0:8080";

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
async fn post_emotion(
    State(state): State<AppState>,
    Json(payload): Json<ApiEmotionMessage>,
) -> impl IntoResponse {
    match send_emotion_to_crab(state.emotion_ch_tx.clone(), payload.emotion).await {
        Ok(status) => status,
        Err(e) => e,
    }
}

async fn send_emotion_to_crab(
    emotion_ch_tx: tokio::sync::mpsc::Sender<emotionmanager::EmotionCommand>,
    emotion: Emotion,
) -> Result<StatusCode, StatusCode> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let set_emotion_result = emotion_ch_tx
        .send(emotionmanager::EmotionCommand::Set { emotion, resp: tx })
        .await;

    if let Err(_) = set_emotion_result {
        println!("Error sending emotion to crab");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Wait for the emotion to be set
    if let Err(_) = rx.await {
        println!("Error waiting for emotion to be set");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(StatusCode::OK)
}

async fn text_to_emotion(text: &str) -> Emotion {
    let text = text.to_lowercase();
    if text.contains("rust") || text.contains("rs") {
        Emotion::Happy
    } else if text.contains("golang") || text.contains("go") {
        Emotion::Angered
    } else if text.contains("cobol") {
        Emotion::Surprised
    } else {
        Emotion::Neutral
    }
}

#[derive(utoipa::ToSchema, serde::Deserialize)]
struct ApiTalkMessage {
    message: String,
}

#[utoipa::path(post,
    path = "/crab/talk",
    summary = "Talk to the crab!",
    request_body = ApiTalkMessage,
    responses(
        (status = 200, description = "Success!", body = ())
))]
async fn post_crab_talk(
    State(state): State<AppState>,
    Json(payload): Json<ApiTalkMessage>,
) -> impl IntoResponse {
    // TODO: Figure out what makes crabs feel things

    let text = payload.message;
    let emotion = text_to_emotion(&text).await;

    match send_emotion_to_crab(state.emotion_ch_tx.clone(), emotion).await {
        Ok(status) => status,
        Err(e) => e,
    }
}

async fn root(State(_): State<AppState>) -> impl IntoResponse {
    Html(include_str!("crab.html"))
}

async fn graphql(
    axum::Extension(schema): axum::Extension<Arc<crate::graphql::Schema>>,
    axum::Extension(context): axum::Extension<crate::graphql::Context>,
    JuniperRequest(req): JuniperRequest,
) -> JuniperResponse {
    JuniperResponse(req.execute(&*schema, &context).await)
}

async fn graphql_subscriptions(
    axum::Extension(schema): axum::Extension<Arc<crate::graphql::Schema>>,
    axum::Extension(context): axum::Extension<crate::graphql::Context>,
    ws: axum::extract::WebSocketUpgrade,
) -> axum::response::Response {
    ws.protocols(["graphql-transport-ws", "graphql-ws"])
        .on_upgrade(move |socket| {
            juniper_axum::subscriptions::serve_ws(
                socket,
                schema,
                juniper_graphql_ws::ConnectionConfig {
                    context,
                    max_in_flight_operations: 0,
                    keep_alive_interval: std::time::Duration::from_secs(15),
                },
            )
        })
}

fn app() -> axum::Router<AppState> {
    let routes: utoipa_axum::router::UtoipaMethodRouter<AppState> =
        utoipa_axum::routes!(post_emotion);
    let (router, api): (axum::Router<AppState>, utoipa::openapi::OpenApi) =
        utoipa_axum::router::OpenApiRouter::with_openapi(ApiDoc::openapi())
            .routes(routes)
            .routes(utoipa_axum::routes!(post_crab_talk))
            .route(
                "/graphql",
                on(MethodFilter::GET.or(MethodFilter::POST), graphql),
            )
            .route("/graphql-subscriptions", get(graphql_subscriptions))
            .route(
                "/graphiql",
                get(juniper_axum::graphiql("/graphql", "/graphql-subscriptions")),
            )
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

#[derive(Clone)]
struct AppState {
    pub emotion_ch_tx: tokio::sync::mpsc::Sender<emotionmanager::EmotionCommand>,
}

#[tokio::main]
pub async fn run_http_server(
    emotion_ch_tx: tokio::sync::mpsc::Sender<emotionmanager::EmotionCommand>,
    emotionmanager: emotionmanager::EmotionManager,
    graphql_context: crate::graphql::Context,
) {
    let state = AppState { emotion_ch_tx };
    let router = app()
        .with_state(state)
        .layer(axum::Extension(Arc::new(crate::graphql::schema())))
        .layer(axum::Extension(graphql_context));
    let listener = tokio::net::TcpListener::bind(BIND_ADDR).await.unwrap();

    let em = emotionmanager.run();

    axum::serve(listener, router).await.unwrap();
    em.await.unwrap();
}
