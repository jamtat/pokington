//! Bounded HTTP feasibility probe. No cookies, browser, or persistent dataset.
use pokington::source::http_client;
use std::{error::Error, fs};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: fetch_official URL OUTPUT".into());
    }
    let client = http_client()?;
    let response = client
        .get(args.first().ok_or("missing URL")?)
        .send()
        .await?;
    let status = response.status();
    println!("{} {}", response.status(), response.uri());
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let body = response.bytes().await?;
    println!("{}; {} bytes", content_type, body.len());
    fs::write(args.get(1).ok_or("missing output path")?, body)?;
    if !status.is_success() {
        return Err(format!("HTTP {status}; response body preserved").into());
    }
    Ok(())
}
