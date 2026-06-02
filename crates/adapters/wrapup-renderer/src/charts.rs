use domain::errors::DomainError;
use domain::models::wrapup::WrapUpReport;
use plotters::prelude::*;

pub fn render_genre_chart(
    report: &WrapUpReport,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, DomainError> {
    let mut buf = vec![0u8; (width * height * 3) as usize];

    {
        let root =
            BitMapBackend::with_buffer(&mut buf, (width, height)).into_drawing_area();
        root.fill(&RGBColor(26, 26, 36))
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

        let max_count = report
            .top_genres
            .iter()
            .map(|g| g.count)
            .max()
            .unwrap_or(1);

        let mut chart = ChartBuilder::on(&root)
            .margin(40)
            .x_label_area_size(60)
            .y_label_area_size(60)
            .build_cartesian_2d(
                0u32..max_count + 1,
                (0..report.top_genres.len() as i32).into_segmented(),
            )
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

        chart
            .configure_mesh()
            .disable_mesh()
            .label_style(("sans-serif", 14, &WHITE))
            .axis_style(&RGBColor(100, 100, 100))
            .draw()
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

        chart
            .draw_series(report.top_genres.iter().enumerate().map(|(i, g)| {
                let color = RGBColor(229, 192, 52);
                Rectangle::new(
                    [
                        (0, SegmentValue::Exact(i as i32)),
                        (g.count, SegmentValue::Exact(i as i32 + 1)),
                    ],
                    color.filled(),
                )
            }))
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

        root.present()
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
    }

    // Convert raw RGB to PNG via image crate
    let img = image::RgbImage::from_raw(width, height, buf)
        .ok_or_else(|| DomainError::InfrastructureError("invalid image buffer".into()))?;
    let rgba = image::DynamicImage::ImageRgb8(img).to_rgba8();
    let mut png_buf = Vec::new();
    rgba.write_to(
        &mut std::io::Cursor::new(&mut png_buf),
        image::ImageFormat::Png,
    )
    .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
    Ok(png_buf)
}
