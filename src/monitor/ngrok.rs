use ngrok::{Session, forwarder::Forwarder, prelude::*, tunnel::HttpTunnel};
use url::Url;

#[derive(Debug)]
pub enum NgrokError {
    InvalidToken,
    DomainInUse,
    Other(String),
}

fn classify_error(error: impl ngrok::Error, authtoken: &str) -> NgrokError {
    match error.error_code() {
        Some("ERR_NGROK_105" | "ERR_NGROK_106" | "ERR_NGROK_107") => NgrokError::InvalidToken,
        Some("ERR_NGROK_334") => NgrokError::DomainInUse,
        _ => NgrokError::Other(error.to_string().replace(authtoken, "****")),
    }
}

pub struct NgrokTunnel {
    session: Session,
    forwarder: Forwarder<HttpTunnel>,
    url: String,
}

impl NgrokTunnel {
    pub async fn start(authtoken: &str, domain: &str, port: u16) -> Result<Self, NgrokError> {
        let mut session = Session::builder()
            .authtoken(authtoken)
            .connect()
            .await
            .map_err(|error| classify_error(error, authtoken))?;
        let mut endpoint = session.http_endpoint();
        endpoint.domain(domain);
        let target = Url::parse(&format!("http://127.0.0.1:{port}")).unwrap();
        let forwarder = match endpoint.listen_and_forward(target).await {
            Ok(forwarder) => forwarder,
            Err(error) => {
                let error = classify_error(error, authtoken);
                let _ = session.close().await;
                return Err(error);
            }
        };
        Ok(Self {
            url: forwarder.url().to_string(),
            session,
            forwarder,
        })
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub async fn stop(mut self) {
        let _ = self.forwarder.close().await;
        self.forwarder.join().abort();
        let _ = self.session.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestError(&'static str);
    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "network error: secret-token")
        }
    }
    impl std::error::Error for TestError {}
    impl ngrok::Error for TestError {
        fn error_code(&self) -> Option<&str> {
            Some(self.0)
        }
    }

    #[test]
    fn classifies_errors_and_redacts_authtoken() {
        for code in ["ERR_NGROK_105", "ERR_NGROK_106", "ERR_NGROK_107"] {
            assert!(matches!(
                classify_error(TestError(code), "secret-token"),
                NgrokError::InvalidToken
            ));
        }
        assert!(matches!(
            classify_error(TestError("ERR_NGROK_334"), "secret-token"),
            NgrokError::DomainInUse
        ));
        let NgrokError::Other(message) = classify_error(TestError("other"), "secret-token") else {
            panic!("expected network error")
        };
        assert!(!message.contains("secret-token"));
    }
}
