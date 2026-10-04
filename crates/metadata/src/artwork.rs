use image::{DynamicImage, ImageReader, codecs::jpeg::JpegEncoder, imageops::FilterType};
use sha2::{Digest, Sha256};
use std::{
    io::Cursor,
    path::{Path, PathBuf},
};

fn decode(bytes: &[u8]) -> Result<DynamicImage, String> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    reader
        .decode()
        .map_err(|e| format!("Cannot read cover artwork: {e}"))
}

fn jpeg(image: DynamicImage) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, 90)
        .encode_image(&image.to_rgb8())
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}

pub fn cache_cover(bytes: &[u8], directory: &Path) -> Result<PathBuf, String> {
    let bytes = jpeg(decode(bytes)?.thumbnail(500, 500))?;
    let path = directory.join(format!("{:x}.jpg", Sha256::digest(&bytes)));
    if !path.is_file() {
        openbox_storage::write_atomic(&path, &bytes)?;
    }
    path.canonicalize().map_err(|e| e.to_string())
}

pub fn device_covers(path: &Path) -> Result<[Vec<u8>; 2], String> {
    let image = decode(&std::fs::read(path).map_err(|e| e.to_string())?)?;
    Ok([
        jpeg(image.resize_to_fill(80, 80, FilterType::Lanczos3))?,
        jpeg(image.resize_to_fill(240, 240, FilterType::Lanczos3))?,
    ])
}
