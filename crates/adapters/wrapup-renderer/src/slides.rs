use domain::errors::DomainError;
use domain::models::wrapup::WrapUpReport;
use image::{Rgba, RgbaImage};

const BG_COLOR: Rgba<u8> = Rgba([26, 26, 36, 255]); // dark blue-gray
const _PRIMARY: Rgba<u8> = Rgba([229, 192, 52, 255]); // gold #e5c034
const _TEXT_COLOR: Rgba<u8> = Rgba([255, 255, 255, 255]); // white

fn to_png(img: &RgbaImage) -> Result<Vec<u8>, DomainError> {
    let mut buf = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut buf),
        image::ImageFormat::Png,
    )
    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
    Ok(buf)
}

fn fill_background(width: u32, height: u32) -> RgbaImage {
    RgbaImage::from_pixel(width, height, BG_COLOR)
}

pub fn render_hero_slide(
    _report: &WrapUpReport,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, DomainError> {
    let img = fill_background(width, height);
    // MVP: solid background. Text overlay added with font rendering later.
    to_png(&img)
}

pub fn render_ratings_slide(
    _report: &WrapUpReport,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, DomainError> {
    let img = fill_background(width, height);
    to_png(&img)
}

pub fn render_directors_slide(
    _report: &WrapUpReport,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, DomainError> {
    let img = fill_background(width, height);
    to_png(&img)
}

pub fn render_actors_slide(
    _report: &WrapUpReport,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, DomainError> {
    let img = fill_background(width, height);
    to_png(&img)
}

pub fn render_highlights_slide(
    _report: &WrapUpReport,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, DomainError> {
    let img = fill_background(width, height);
    to_png(&img)
}

pub fn render_mosaic_slide(
    posters: &[(String, Vec<u8>)],
    width: u32,
    height: u32,
) -> Result<Vec<u8>, DomainError> {
    let mut canvas = fill_background(width, height);

    let cols = 4u32;
    let thumb_w = width / cols;
    let thumb_h = (thumb_w * 3) / 2; // 2:3 poster ratio

    for (i, (_, bytes)) in posters.iter().enumerate() {
        let col = (i as u32) % cols;
        let row = (i as u32) / cols;
        let x = col * thumb_w;
        let y = row * thumb_h;
        if y + thumb_h > height {
            break;
        }

        if let Ok(poster) = image::load_from_memory(bytes) {
            let thumb =
                poster.resize_exact(thumb_w, thumb_h, image::imageops::FilterType::Triangle);
            image::imageops::overlay(&mut canvas, &thumb.to_rgba8(), x as i64, y as i64);
        }
    }

    to_png(&canvas)
}
