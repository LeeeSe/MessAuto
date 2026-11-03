use axum::{
    Json, Router,
    extract::{FromRequest, Query, Request, State},
    http::{StatusCode, header::AUTHORIZATION},
    routing::post,
};
use serde::{Deserialize, Serialize};
use std::{io, sync::Arc};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};

#[derive(Deserialize)]
struct WebhookPayload {
    value: String,
}

#[derive(Serialize)]
struct WebhookResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Deserialize)]
struct TokenQuery {
    token: Option<String>,
}

#[derive(Clone)]
struct AppState {
    token: Arc<str>,
    on_code: Arc<dyn Fn(String) + Send + Sync>,
}

pub fn router(token: String, on_code: impl Fn(String) + Send + Sync + 'static) -> Router {
    Router::new()
        .route("/webhook", post(handle))
        .route("/", post(handle))
        .with_state(AppState {
            token: token.into(),
            on_code: Arc::new(on_code),
        })
}

async fn handle(
    State(state): State<AppState>,
    request: Request,
) -> (StatusCode, Json<WebhookResponse>) {
    let query = Query::<TokenQuery>::try_from_uri(request.uri()).ok();
    let token = match request.headers().get(AUTHORIZATION) {
        Some(header) => header.to_str().ok().and_then(|s| s.strip_prefix("Bearer ")),
        None => query.as_ref().and_then(|q| q.token.as_deref()),
    };
    if state.token.is_empty() || !token.is_some_and(|t| token_eq(t, &state.token)) {
        return response(StatusCode::UNAUTHORIZED, None, Some("unauthorized".into()));
    }

    // 鉴权成功后才读取、解析请求体。
    let payload = match Json::<WebhookPayload>::from_request(request, &state).await {
        Ok(Json(payload)) => payload,
        Err(error) => return response(StatusCode::BAD_REQUEST, None, Some(error.body_text())),
    };
    match crate::parser::extract_verification_code(&payload.value) {
        Some(code) => {
            let action_code = code.clone();
            tokio::task::spawn_blocking(move || (state.on_code)(action_code));
            response(StatusCode::OK, Some(code), None)
        }
        None => response(
            StatusCode::OK,
            None,
            Some("no verification code found".into()),
        ),
    }
}

fn response(
    status: StatusCode,
    code: Option<String>,
    error: Option<String>,
) -> (StatusCode, Json<WebhookResponse>) {
    (
        status,
        Json(WebhookResponse {
            ok: code.is_some(),
            code,
            error,
        }),
    )
}

fn token_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |diff, (a, b)| diff | (a ^ b))
        == 0
}

pub struct WebhookServer {
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
    addr: std::net::SocketAddr,
}

impl WebhookServer {
    pub async fn start(host: &str, port: u16, token: String) -> io::Result<Self> {
        let listener = TcpListener::bind((host, port)).await?;
        let addr = listener.local_addr()?;
        let (shutdown, stop) = oneshot::channel();
        let app = router(token, |code| {
            crate::action::handle_verification_code(&code, "Webhook")
        });
        let task = tokio::spawn(async move {
            if let Err(error) = axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = stop.await;
                })
                .await
            {
                log::error!("Webhook server failed: {}", error);
                crate::status::update(|s| {
                    s.webhook = crate::status::ServiceState::Failed(error.to_string())
                });
            }
        });
        log::info!("Webhook listening on {}", addr);
        Ok(Self {
            shutdown: Some(shutdown),
            task,
            addr,
        })
    }

    pub fn addr(&self) -> std::net::SocketAddr {
        self.addr
    }

    pub async fn stop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        // 不让尚未发送完整请求的连接阻塞重启。
        if tokio::time::timeout(std::time::Duration::from_secs(2), &mut self.task)
            .await
            .is_err()
        {
            self.task.abort();
            let _ = (&mut self.task).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tower::ServiceExt;

    async fn request(
        app: Router,
        path: &str,
        auth: Option<&str>,
        body: &str,
    ) -> (StatusCode, String) {
        let mut request = Request::post(path).header("content-type", "application/json");
        if let Some(auth) = auth {
            request = request.header(AUTHORIZATION, auth);
        }
        let response = app
            .oneshot(request.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn bearer_query_and_both_paths_deliver_code() {
        for (path, auth) in [
            ("/webhook", Some("Bearer secret")),
            ("/webhook?token=wrong", Some("Bearer secret")),
            ("/?token=secret", None),
            ("/webhook?token=secret", None),
        ] {
            let (tx, rx) = oneshot::channel();
            let tx = std::sync::Mutex::new(Some(tx));
            let app = router("secret".into(), move |code| {
                let _ = tx.lock().unwrap().take().unwrap().send(code);
            });
            let (status, body) = request(app, path, auth, r#"{"value":"验证码 131964"}"#).await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(body, r#"{"ok":true,"code":"131964"}"#);
            assert_eq!(
                tokio::time::timeout(std::time::Duration::from_secs(2), rx)
                    .await
                    .unwrap()
                    .unwrap(),
                "131964"
            );
        }
    }

    #[tokio::test]
    async fn unauthorized_requests_do_not_read_body_or_run_actions() {
        let calls = Arc::new(AtomicUsize::new(0));
        for (path, auth) in [
            ("/webhook", None),
            ("/webhook", Some("Bearer wrong")),
            ("/?token=wrong", None),
            ("/?token=secret", Some("Bearer wrong")),
            ("/?token=secret", Some("Basic secret")),
        ] {
            let count = calls.clone();
            let app = router("secret".into(), move |_| {
                count.fetch_add(1, Ordering::SeqCst);
            });
            assert_eq!(
                request(app, path, auth, "not json").await.0,
                StatusCode::UNAUTHORIZED
            );
        }
        let app = router("secret".into(), |_| panic!("unauthorized action"));
        let unreadable = Body::new(
            http_body_util::Full::new(axum::body::Bytes::from_static(b"bad"))
                .map_frame(|_| panic!("unauthorized body read")),
        );
        let response = app
            .oneshot(Request::post("/webhook").body(unreadable).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn malformed_and_missing_value_are_bad_requests() {
        for body in ["not json", "{}", r#"{"value":42}"#] {
            let app = router("secret".into(), |_| panic!("invalid payload action"));
            assert_eq!(
                request(app, "/", Some("Bearer secret"), body).await.0,
                StatusCode::BAD_REQUEST
            );
        }
    }

    #[tokio::test]
    async fn no_code_has_no_action() {
        let app = router("secret".into(), |_| panic!("no code action"));
        let (status, body) = request(
            app,
            "/webhook",
            Some("Bearer secret"),
            r#"{"value":"hello"}"#,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, r#"{"ok":false,"error":"no verification code found"}"#);
    }

    #[test]
    fn token_comparison() {
        assert!(token_eq("abc", "abc"));
        assert!(!token_eq("abc", "abd"));
        assert!(!token_eq("abc", "abcd"));
    }

    #[tokio::test]
    async fn lifecycle_binds_exclusively_and_releases_port() {
        let mut server = WebhookServer::start("127.0.0.1", 0, "secret".into())
            .await
            .unwrap();
        let port = server.addr().port();
        assert!(
            WebhookServer::start("127.0.0.1", port, "secret".into())
                .await
                .is_err()
        );
        server.stop().await;
        let listener = TcpListener::bind(("127.0.0.1", port)).await.unwrap();
        drop(listener);
    }
}
