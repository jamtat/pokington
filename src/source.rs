//! Shared HTTP profile and errors for selected Serebii discovery.
use std::{fmt, time::Duration};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceError(pub String);
impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for SourceError {}

pub fn http_client() -> Result<wreq::Client, wreq::Error> {
    wreq::Client::builder()
        .emulation(
            wreq_util::Emulation::builder()
                .profile(wreq_util::Profile::Chrome147)
                .platform(wreq_util::Platform::MacOS)
                .build(),
        )
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .https_only(true)
        .redirect(wreq::redirect::Policy::limited(5))
        .no_proxy()
        .build()
}
