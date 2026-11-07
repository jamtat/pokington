//! Sample only artwork referenced by an already captured main-site detail page.
use std::{error::Error, fs, io::Cursor, path::Path};

use http_body_util::{BodyExt, Limited};
#[allow(dead_code)]
#[path = "support/official_source.rs"]
mod official_source;
use official_source::{http_client, parse_index_page, parse_species_detail, parse_species_links};
use pokington::source;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 5 {
        return Err(
            "usage: inspect_artwork INDEX_HTML API_JSON SPECIES_ID DETAIL_HTML OUTPUT_DIR".into(),
        );
    }
    let page = parse_index_page(&fs::read_to_string(&args[0])?)?;
    let links = parse_species_links(&fs::read_to_string(&args[1])?, &page)?;
    let id: u32 = args[2].parse()?;
    let link = links
        .iter()
        .find(|link| link.id == id)
        .ok_or("species absent from source index")?;
    let species = parse_species_detail(&fs::read_to_string(&args[3])?, link)?;
    let directory = Path::new(&args[4]);
    fs::create_dir_all(directory)?;
    let client = http_client()?;
    for form in species.forms {
        let response = client
            .get(&form.artwork_url)
            .send()
            .await?
            .error_for_status()?;
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        let final_url = response.uri().to_string();
        let bytes = Limited::new(response, 16 * 1024 * 1024)
            .collect()
            .await?
            .to_bytes();
        let filename = form
            .artwork_url
            .rsplit('/')
            .next()
            .ok_or("missing artwork filename")?;
        // Preserve this evidence response even if PNG validation rejects it.
        fs::write(directory.join(format!("{filename}.response")), &bytes)?;
        println!(
            "{}: {} bytes; {}; final URL {}",
            form.display_name,
            bytes.len(),
            content_type,
            final_url
        );
        let mut decoder = png::Decoder::new(Cursor::new(&bytes));
        decoder.ignore_checksums(false);
        let mut reader = decoder.read_info()?;
        let size = reader
            .output_buffer_size()
            .ok_or("PNG exceeds address space")?;
        if size > 64 * 1024 * 1024 {
            return Err("PNG exceeds inspection memory limit".into());
        }
        let mut pixels = vec![0; size];
        let info = reader.next_frame(&mut pixels)?;
        reader.finish()?;
        fs::write(directory.join(filename), &bytes)?;
        println!(
            "{}: {}x{}, {:?}, {:?}; full decode, CRC/Adler32 and stream finish passed",
            filename, info.width, info.height, info.color_type, info.bit_depth
        );
    }
    Ok(())
}
