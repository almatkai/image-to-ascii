//! Fixed-origin service transport. Test-only constructors cannot enter the product.
use crate::input::{decode, MAX_RESULT};
use reqwest::{
    multipart::{Form, Part},
    Client,
};
use std::time::Duration;

pub const ENDPOINT: &str = "https://ascii-image-converter.zeabur.app/image-to-ascii";
#[derive(Clone)]
pub struct Upload {
    pub bytes: Vec<u8>,
    pub filename: String,
    pub complex: bool,
    pub width: Option<String>,
    pub height: Option<String>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    Known(&'static str),
    Unknown,
}
#[derive(Clone)]
pub struct Transport {
    client: Client,
    pub(crate) endpoint: String,
    deadline: Duration,
}
impl Transport {
    pub fn production() -> Result<Self, String> {
        Ok(Self {
            client: Self::builder()
                .build()
                .map_err(|_| "Could not prepare secure upload.")?,
            endpoint: ENDPOINT.into(),
            deadline: Duration::from_secs(30),
        })
    }
    fn builder() -> reqwest::ClientBuilder {
        Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(30))
            .referer(false)
            .user_agent("image-to-ascii-native/2.0.0")
            .retry(reqwest::retry::never())
    }
    #[cfg(test)]
    pub(crate) fn owned(endpoint: String, root: &[u8], deadline: Duration) -> Self {
        let client = Self::builder()
            .tls_built_in_root_certs(false)
            .add_root_certificate(reqwest::Certificate::from_pem(root).unwrap())
            .build()
            .unwrap();
        Self {
            client,
            endpoint,
            deadline,
        }
    }
    pub async fn send(&self, upload: &Upload) -> Result<String, Failure> {
        let fut = async {
            let mime = match std::path::Path::new(&upload.filename)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase()
                .as_str()
            {
                "jpg" | "jpeg" => "image/jpeg",
                "png" => "image/png",
                "bmp" => "image/bmp",
                "webp" => "image/webp",
                _ => "application/octet-stream",
            };
            let part = Part::bytes(upload.bytes.clone())
                .file_name(upload.filename.clone())
                .mime_str(mime)
                .map_err(|_| Failure::Known("Could not prepare upload."))?;
            let mut form = Form::new()
                .part("file", part)
                .text("complex", upload.complex.to_string());
            if let Some(w) = &upload.width {
                form = form.text("width", w.clone());
            }
            if let Some(h) = &upload.height {
                form = form.text("height", h.clone());
            }
            // No credentials, cookies, system proxy or automatic redirects/retries.
            let mut response = self
                .client
                .post(&self.endpoint)
                .multipart(form)
                .send()
                .await
                .map_err(|_| Failure::Unknown)?;
            if !response.status().is_success() {
                return Err(Failure::Known("Service did not accept the upload."));
            }
            if response
                .content_length()
                .is_some_and(|n| n > MAX_RESULT as u64)
            {
                return Err(Failure::Known("Service response exceeds 1,000,000 bytes."));
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|_| Failure::Unknown)? {
                if chunk.len() > MAX_RESULT - bytes.len() {
                    return Err(Failure::Known("Service response exceeds 1,000,000 bytes."));
                }
                bytes.extend_from_slice(&chunk);
            }
            decode(&bytes).map_err(Failure::Known)
        };
        tokio::time::timeout(self.deadline, fut)
            .await
            .map_err(|_| Failure::Unknown)?
    }
}
