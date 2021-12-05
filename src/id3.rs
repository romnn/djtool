use anyhow::{anyhow, Result};
use std::path::Path;

pub fn embed_image<P: AsRef<Path>>(music_filename: P, image_filename: P) -> Result<()> {
    let mut tag = read_tag(music_filename.as_ref())?;
    let image = image::open(&image_filename.as_ref()).map_err(|e| {
        anyhow!(
            "Error reading image {}: {}",
            image_filename.as_ref().display(),
            e
        )
    })?;

    let mut encoded_image_bytes = Vec::new();
    image.write_to(&mut encoded_image_bytes, image::ImageOutputFormat::Jpeg(90))?;

    tag.add_picture(id3::frame::Picture {
        mime_type: "image/jpeg".to_string(),
        picture_type: id3::frame::PictureType::CoverFront,
        description: String::new(),
        data: encoded_image_bytes,
    });

    tag.write_to_path(music_filename.as_ref(), id3::Version::Id3v23)
        .map_err(|e| {
            anyhow!(
                "Error writing image to music file {}: {}",
                music_filename.as_ref().display(),
                e
            )
        })?;

    Ok(())
}

fn read_tag<P: AsRef<Path>>(path: P) -> Result<id3::Tag> {
    id3::Tag::read_from_path(path.as_ref()).or_else(|e| {
        eprintln!(
            "Warning: file metadata is corrupted, trying to read partial tag: {}",
            path.as_ref().display()
        );
        e.partial_tag.clone().ok_or_else(|| {
            anyhow!(
                "Error reading music file {}: {}",
                path.as_ref().display(),
                e
            )
        })
    })
}
