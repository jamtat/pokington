//! Shared HTTP client, bounded response reads and source errors.
use std::{fmt, time::Duration};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceError(pub String);
impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for SourceError {}

/// Build the configured HTTP client.
///
/// # Errors
///
/// Returns the client builder's error when the configured client cannot be built.
pub fn http_client() -> Result<reqwest::Client, reqwest::Error> {
    http_client_builder().build()
}

/// The same client can target a local HTTP fixture by disabling HTTPS-only.
pub fn http_client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .https_only(true)
        .redirect(reqwest::redirect::Policy::limited(5))
        .no_proxy()
}

/// Read a response while limiting its decompressed body size, including chunked responses.
///
/// # Errors
/// Returns an error for failed body reads or a body exceeding the byte limit.
pub async fn read_limited(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<Vec<u8>, SourceError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| SourceError(error.to_string()))?
    {
        if chunk.len() > limit.saturating_sub(bytes.len()) {
            return Err(SourceError(format!("response body exceeds {limit} bytes")));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
