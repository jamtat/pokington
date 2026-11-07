//! Offline inspection of a captured source response; never adopts it for bootstrap.
use std::{error::Error, fs, io::Cursor};

#[allow(dead_code)]
#[path = "support/portal_source.rs"]
mod portal_source;
use portal_source::parse_portal_index;

fn main() -> Result<(), Box<dyn Error>> {
    let mut paths = std::env::args_os().skip(1);
    let html_path = paths
        .next()
        .ok_or("usage: inspect_source INDEX.html [ART.png ...]")?;
    let index = parse_portal_index(&fs::read_to_string(html_path)?)?;
    println!(
        "source bounds: {:?}; species: {}; form entries: {}",
        index.bounds(),
        index.species_count(),
        index.entries.len()
    );
    match index.form_keys() {
        Ok(keys) => println!("unambiguous candidate form keys: {}", keys.len()),
        Err(error) => println!("form-key blocker: {error}"),
    }
    println!(
        "WARNING: structural parsing is not proof of complete official coverage; this source is unadopted."
    );
    for path in paths {
        let bytes = fs::read(&path)?;
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
        println!(
            "{}: {} bytes, {}x{}, {:?}, {:?}, full PNG decode passed",
            path.to_string_lossy(),
            bytes.len(),
            info.width,
            info.height,
            info.color_type,
            info.bit_depth
        );
    }
    Ok(())
}
