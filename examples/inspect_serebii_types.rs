//! Print explicit type panels from an already captured source page.
use pokington::serebii::parse_type_evidence;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: inspect_serebii_types HTML_CAPTURE")?;
    let html: String = std::fs::read(path)?.into_iter().map(char::from).collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&parse_type_evidence(&html)?)?
    );
    Ok(())
}
