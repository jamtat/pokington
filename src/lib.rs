//! Source-linked Serebii catalogue and recoverable local storage.
pub mod catalogue;
#[cfg(unix)]
pub mod inline_image;
pub mod serebii;
pub mod source;
#[cfg(unix)]
pub mod store;
