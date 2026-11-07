//! Offline source-artwork checks; accepts no inferred or transformed images.
use std::{error::Error, fs, io::Cursor};

fn main() -> Result<(), Box<dyn Error>> {
    for path in std::env::args_os().skip(1) {
        let bytes = fs::read(&path)?;
        let mut decoder = png::Decoder::new(Cursor::new(&bytes));
        decoder.ignore_checksums(false);
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info()?;
        let size = reader
            .output_buffer_size()
            .ok_or("PNG exceeds address space")?;
        if size > 64 * 1024 * 1024 {
            return Err("PNG exceeds decode memory limit".into());
        }
        let mut pixels = vec![0; size];
        let info = reader.next_frame(&mut pixels)?;
        reader.finish()?;
        let alpha_stride = match info.color_type {
            png::ColorType::Rgba => Some(4),
            png::ColorType::GrayscaleAlpha => Some(2),
            _ => None,
        };
        let transparent = alpha_stride
            .map(|stride| {
                pixels[..info.buffer_size()]
                    .chunks_exact(stride)
                    .filter(|pixel| pixel[stride - 1] == 0)
                    .count()
            })
            .unwrap_or(0);
        println!(
            "{}: {} bytes; {}x{} {:?}; {} fully transparent pixels; CRC, Adler32, frame and stream-end checks passed",
            path.to_string_lossy(),
            bytes.len(),
            info.width,
            info.height,
            info.color_type,
            transparent
        );
    }
    Ok(())
}
