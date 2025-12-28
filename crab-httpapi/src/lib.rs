use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::*,
};
use utoipa::OpenApi;

pub mod emotionmanager;
use emotionmanager::Emotion;
mod puzzle;
pub use puzzle::Puzzles;

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
    emotion: Emotion,
}

impl std::fmt::Display for ApiEmotionMessage {
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

#[derive(utoipa::ToSchema, serde::Deserialize)]
pub struct ApiPressureLimitsMessage {
    pub token: String,
    pub low_low: Option<f64>,
    pub low: Option<f64>,
    pub high: Option<f64>,
    pub high_high: Option<f64>,
}

#[utoipa::path(post,
    path = "/crab/set-pressure-limits",
    summary = "Set crab air pressure limits",
    request_body = ApiPressureLimitsMessage,
    responses(
        (status = 200, description = "Success!", body = ()),
        (status = 403, description = "Invalid token was sent", body = ()),
    ),
)]
async fn post_crab_set_pressure_limits(
    State(state): State<AppState>,
    Json(payload): Json<ApiPressureLimitsMessage>,
) -> impl IntoResponse {
    use sha1::Digest;

    let mut hasher = sha1::Sha1::new();
    hasher.update(payload.token.as_bytes());
    let res = hasher.finalize();

    // "Security"
    if res[..] != hex_literal::hex!("49203b5f12f55a6fe51a042b53a67d035f7971bb") {
        return Err(StatusCode::FORBIDDEN);
    }

    match state.pressure_limits_tx.send(payload).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(utoipa::ToSchema, serde::Deserialize)]
struct ApiTokenMessage {
    token: String,
}

#[utoipa::path(post,
    path = "/crab/inflate",
    summary = "Forcefully inflate the crab!",
    request_body = ApiTokenMessage,
    responses(
        (status = 200, description = "Success!", body = ()),
        (status = 403, description = "Invalid token was sent", body = ()),
    ),
)]
async fn post_crab_inflate(
    State(state): State<AppState>,
    Json(payload): Json<ApiTokenMessage>,
) -> impl IntoResponse {
    use sha1::Digest;

    let mut hasher = sha1::Sha1::new();
    hasher.update(payload.token.as_bytes());
    let res = hasher.finalize();

    // "Security"
    if res[..] != hex_literal::hex!("49203b5f12f55a6fe51a042b53a67d035f7971bb") {
        return Err(StatusCode::FORBIDDEN);
    }

    state
        .trigger_fan
        .store(true, std::sync::atomic::Ordering::SeqCst);

    Ok(StatusCode::OK)
}

#[utoipa::path(post,
    path = "/crab/sleep",
    summary = "Put the crab to sleep.",
    request_body = ApiTokenMessage,
    responses(
        (status = 200, description = "Success!", body = ()),
        (status = 403, description = "Invalid token was sent", body = ()),
    ),
)]
async fn post_crab_sleep(
    State(state): State<AppState>,
    Json(payload): Json<ApiTokenMessage>,
) -> impl IntoResponse {
    use sha1::Digest;

    let mut hasher = sha1::Sha1::new();
    hasher.update(payload.token.as_bytes());
    let res = hasher.finalize();

    // "Security"
    if res[..] != hex_literal::hex!("49203b5f12f55a6fe51a042b53a67d035f7971bb") {
        return Err(StatusCode::FORBIDDEN);
    }

    state
        .trigger_sleep
        .store(true, std::sync::atomic::Ordering::SeqCst);

    Ok(StatusCode::OK)
}

#[utoipa::path(post,
    path = "/crab/fault_reset",
    summary = "Reset faults of the crab controller",
    responses(
        (status = 200, description = "Success!", body = ())
))]
async fn post_crab_fault_reset(State(state): State<AppState>) -> impl IntoResponse {
    state
        .fault_reset
        .store(true, std::sync::atomic::Ordering::SeqCst)
}

#[derive(utoipa::ToSchema, serde::Deserialize)]
pub struct RfcGameNewMessage {}

#[derive(utoipa::ToSchema, serde::Serialize)]
pub struct RfcGameNewResponse {
    pub key: puzzle::PuzzleKey,
}

#[utoipa::path(post,
    path = "/crab/rfc-game/new",
    summary = "Create a new game session for the RFC game",
    request_body = RfcGameNewMessage,
    responses(
        (status = 200, description = "Success!", body = RfcGameNewResponse),
    ),
)]
async fn post_crab_rfc(
    State(state): State<AppState>,
    Json(RfcGameNewMessage {}): Json<RfcGameNewMessage>,
) -> impl IntoResponse {
    let timeout = std::time::Instant::now() + std::time::Duration::from_secs(300);
    let key = state.puzzles.lock().unwrap().generate(timeout);
    Json(RfcGameNewResponse { key })
}

/// This makes <iframe src="/crab/rfc-game/rfc/{key}"></iframe> work (so we don't open ourselves to
/// XSS attacks by serving arbitrary HTML)
#[utoipa::path(get,
    path = "/crab/rfc-game/rfc/{key}",
    summary = "Grab the rendered HTML of an RFC",
    responses(
        (status = OK, description = "Success", body = str, content_type = "text/html"),
        (status = NOT_FOUND, description = "Bad game key", body = str, content_type = "text/html")
    )
)]
async fn get_crab_rfc_html(
    State(state): State<AppState>,
    axum::extract::Path(key): axum::extract::Path<puzzle::PuzzleKey>,
) -> impl IntoResponse {
    let html = {
        let puzzles = state.puzzles.lock().unwrap();
        puzzles.get_html(&key).map(String::from)
    };

    if let Some(html) = html {
        (axum::http::StatusCode::OK, axum::response::Html(html))
    } else {
        (
            axum::http::StatusCode::NOT_FOUND,
            axum::response::Html("Not Found".to_string()),
        )
    }
}

#[derive(utoipa::ToSchema, serde::Deserialize)]
pub struct RfcGameAttemptMessage {
    pub key: puzzle::PuzzleKey,
    pub attempt: puzzle::Judgement,
}

#[derive(utoipa::ToSchema, serde::Serialize)]
pub struct RfcGameSuccessResponse {
    pub key: puzzle::PuzzleKey,
    pub success: bool,
    pub solution: Option<puzzle::Judgement>,
}

#[utoipa::path(post,
    path = "/crab/rfc-game/solve",
    summary = "Attempt to solve an RFC game session, ending it",
    request_body = RfcGameAttemptMessage,
    responses(
        (status = 200, description = "Success!", body = RfcGameSuccessResponse),
    ),
)]
async fn post_crab_rfc_solve(
    State(state): State<AppState>,
    Json(RfcGameAttemptMessage { key, attempt }): Json<RfcGameAttemptMessage>,
) -> impl IntoResponse {
    let (success, solution) = state.puzzles.lock().unwrap().solve(&key, attempt);

    let emotion = success.then_some(Emotion::Happy).unwrap_or(Emotion::Sad);
    let status = match send_emotion_to_crab(state.emotion_ch_tx.clone(), emotion).await {
        Ok(status) => status,
        Err(e) => e,
    };

    (
        status,
        Json(RfcGameSuccessResponse {
            key,
            success,
            solution,
        }),
    )
}

async fn root(State(_): State<AppState>) -> impl IntoResponse {
    Html(include_str!("crab.html"))
}

fn setup_metrics_recorder() -> metrics_exporter_prometheus::PrometheusHandle {
    let recorder_handle = metrics_exporter_prometheus::PrometheusBuilder::new()
        .install_recorder()
        .unwrap();

    let upkeep_handle = recorder_handle.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            upkeep_handle.run_upkeep();
        }
    });

    recorder_handle
}

fn app() -> axum::Router<AppState> {
    let (router, api): (axum::Router<AppState>, utoipa::openapi::OpenApi) =
        utoipa_axum::router::OpenApiRouter::with_openapi(ApiDoc::openapi())
            .routes(utoipa_axum::routes!(post_emotion))
            .routes(utoipa_axum::routes!(post_crab_talk))
            .routes(utoipa_axum::routes!(post_crab_inflate))
            .routes(utoipa_axum::routes!(post_crab_sleep))
            .routes(utoipa_axum::routes!(post_crab_fault_reset))
            .routes(utoipa_axum::routes!(post_crab_set_pressure_limits))
            .routes(utoipa_axum::routes!(post_crab_rfc))
            .routes(utoipa_axum::routes!(get_crab_rfc_html))
            .routes(utoipa_axum::routes!(post_crab_rfc_solve))
            .split_for_parts();

    let router = router.route("/", get(root)).merge(
        utoipa_swagger_ui::SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api.clone()),
    );

    let recorder_handle = setup_metrics_recorder();
    let router = router.route(
        "/metrics",
        get(move || std::future::ready(recorder_handle.render())),
    );

    router
}

#[derive(Clone)]
pub struct AppState {
    pub emotion_ch_tx: tokio::sync::mpsc::Sender<emotionmanager::EmotionCommand>,
    pub fault_reset: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub trigger_fan: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub trigger_sleep: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub pressure_limits_tx: tokio::sync::mpsc::Sender<ApiPressureLimitsMessage>,
    pub puzzles: std::sync::Arc<std::sync::Mutex<puzzle::Puzzles>>,
}

#[tokio::main(flavor = "current_thread")]
pub async fn run_http_server(
    state: AppState,
    emotionmanager: emotionmanager::EmotionManager,
    graphql_router: Option<axum::Router<AppState>>,
) {
    let mut router = app();
    if let Some(graphql_router) = graphql_router {
        router = router.merge(graphql_router);
    }

    let puzzle_reap = state.puzzles.clone();
    let router = router.with_state(state);

    let listener = tokio::net::TcpListener::bind(BIND_ADDR).await.unwrap();

    let em = emotionmanager.run();

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            let now = tokio::time::Instant::now();
            puzzle_reap.lock().unwrap().reap(now.into_std());
        }
    });

    axum::serve(listener, router).await.unwrap();
    em.await.unwrap();
}
