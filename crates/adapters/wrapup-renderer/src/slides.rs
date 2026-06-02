use ab_glyph::{FontArc, PxScale};
use domain::errors::DomainError;
use domain::models::wrapup::WrapUpReport;
use image::{Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut};
use imageproc::rect::Rect;

const BG: Rgba<u8> = Rgba([26, 26, 36, 255]);
const GOLD: Rgba<u8> = Rgba([229, 192, 52, 255]);
const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);
const DIM: Rgba<u8> = Rgba([255, 255, 255, 140]);
const BAR_BG: Rgba<u8> = Rgba([50, 50, 65, 255]);

pub struct SlideRenderer {
    font: FontArc,
    logo: Option<RgbaImage>,
}

impl SlideRenderer {
    pub fn new(font_path: Option<&str>, logo_path: Option<&str>) -> Result<Self, DomainError> {
        let font = if let Some(path) = font_path {
            let bytes = std::fs::read(path)
                .map_err(|e| DomainError::InfrastructureError(format!("font load: {e}")))?;
            FontArc::try_from_vec(bytes)
                .map_err(|e| DomainError::InfrastructureError(format!("font parse: {e}")))?
        } else {
            load_system_font()?
        };

        let logo = if let Some(path) = logo_path {
            let img = image::open(path)
                .map_err(|e| DomainError::InfrastructureError(format!("logo load: {e}")))?;
            Some(img.to_rgba8())
        } else {
            None
        };

        Ok(Self { font, logo })
    }

    fn stamp_logo(&self, canvas: &mut RgbaImage) {
        if let Some(ref logo) = self.logo {
            let logo_size = 64u32;
            let resized = image::imageops::resize(
                logo,
                logo_size,
                logo_size,
                image::imageops::FilterType::Triangle,
            );
            let margin = 20i64;
            let x = canvas.width() as i64 - logo_size as i64 - margin;
            let y = canvas.height() as i64 - logo_size as i64 - margin;
            image::imageops::overlay(canvas, &resized, x, y);
        }
    }

    fn draw_centered(
        &self,
        canvas: &mut RgbaImage,
        text: &str,
        y: i32,
        scale: f32,
        color: Rgba<u8>,
    ) {
        let px = PxScale::from(scale);
        // approximate width: ~0.5 * scale * len
        let approx_w = (text.len() as f32 * scale * 0.45) as i32;
        let x = ((canvas.width() as i32 - approx_w) / 2).max(10);
        draw_text_mut(canvas, color, x, y, px, &self.font, text);
    }

    fn draw_left(
        &self,
        canvas: &mut RgbaImage,
        text: &str,
        x: i32,
        y: i32,
        scale: f32,
        color: Rgba<u8>,
    ) {
        draw_text_mut(canvas, color, x, y, PxScale::from(scale), &self.font, text);
    }

    pub fn render_hero(
        &self,
        report: &WrapUpReport,
        w: u32,
        h: u32,
    ) -> Result<Vec<u8>, DomainError> {
        let mut img = fill(w, h);

        let year_label = format!(
            "{} - {}",
            report.date_range.start.format("%b %Y"),
            report.date_range.end.format("%b %Y")
        );
        self.draw_centered(&mut img, &year_label, (h / 6) as i32, 48.0, DIM);
        self.draw_centered(
            &mut img,
            &report.total_movies.to_string(),
            (h / 3) as i32,
            160.0,
            GOLD,
        );
        self.draw_centered(
            &mut img,
            "movies watched",
            (h / 3 + 170) as i32,
            40.0,
            WHITE,
        );

        let hours = report.total_watch_time_minutes / 60;
        let mins = report.total_watch_time_minutes % 60;
        let time_str = format!("{}h {}m of watch time", hours, mins);
        self.draw_centered(&mut img, &time_str, (h / 2 + 60) as i32, 36.0, DIM);

        if let Some(ref month) = report.busiest_month {
            let s = format!("Busiest month: {month}");
            self.draw_centered(&mut img, &s, (h * 2 / 3) as i32, 32.0, DIM);
        }
        if let Some(ref dow) = report.busiest_day_of_week {
            let s = format!("Favorite day: {dow}");
            self.draw_centered(&mut img, &s, (h * 2 / 3 + 50) as i32, 32.0, DIM);
        }

        self.stamp_logo(&mut img);
        to_png(&img)
    }

    pub fn render_ratings(
        &self,
        report: &WrapUpReport,
        w: u32,
        h: u32,
    ) -> Result<Vec<u8>, DomainError> {
        let mut img = fill(w, h);
        self.draw_centered(&mut img, "Ratings", (h / 8) as i32, 56.0, GOLD);

        if let Some(avg) = report.avg_rating {
            let s = format!("{:.1} / 5", avg);
            self.draw_centered(&mut img, &s, (h / 4) as i32, 80.0, WHITE);
            self.draw_centered(&mut img, "average rating", (h / 4 + 90) as i32, 32.0, DIM);
        }

        // rating distribution bars
        let max_count = report
            .rating_distribution
            .iter()
            .copied()
            .max()
            .unwrap_or(1)
            .max(1);
        let bar_area_top = (h / 2) as i32;
        let bar_h = 36u32;
        let bar_gap = 16u32;
        let margin_x = 120i32;
        let max_bar_w = (w as i32 - margin_x * 2) as u32;

        for (i, &count) in report.rating_distribution.iter().enumerate() {
            let label = format!("{}\u{2605}", i + 1);
            let y = bar_area_top + (i as i32) * (bar_h as i32 + bar_gap as i32);
            self.draw_left(&mut img, &label, margin_x - 60, y + 2, 28.0, GOLD);

            // background bar
            draw_filled_rect_mut(
                &mut img,
                Rect::at(margin_x, y).of_size(max_bar_w, bar_h),
                BAR_BG,
            );
            // filled bar
            let fill_w = ((count as f32 / max_count as f32) * max_bar_w as f32) as u32;
            if fill_w > 0 {
                draw_filled_rect_mut(&mut img, Rect::at(margin_x, y).of_size(fill_w, bar_h), GOLD);
            }
            // count label
            let count_s = count.to_string();
            self.draw_left(
                &mut img,
                &count_s,
                margin_x + fill_w as i32 + 10,
                y + 2,
                24.0,
                DIM,
            );
        }

        self.stamp_logo(&mut img);
        to_png(&img)
    }

    pub fn render_directors(
        &self,
        report: &WrapUpReport,
        w: u32,
        h: u32,
    ) -> Result<Vec<u8>, DomainError> {
        let mut img = fill(w, h);
        self.draw_centered(&mut img, "Top Directors", (h / 8) as i32, 56.0, GOLD);

        let margin = 80i32;
        let start_y = (h / 4) as i32;
        for (i, d) in report.top_directors.iter().take(5).enumerate() {
            let y = start_y + (i as i32) * 90;
            let rank = format!("{}.", i + 1);
            self.draw_left(&mut img, &rank, margin, y, 36.0, GOLD);
            self.draw_left(&mut img, &d.name, margin + 60, y, 36.0, WHITE);
            let detail = format!("{} films  avg {:.1}\u{2605}", d.count, d.avg_rating);
            self.draw_left(&mut img, &detail, margin + 60, y + 44, 24.0, DIM);
        }

        self.stamp_logo(&mut img);
        to_png(&img)
    }

    pub fn render_actors(
        &self,
        report: &WrapUpReport,
        w: u32,
        h: u32,
    ) -> Result<Vec<u8>, DomainError> {
        let mut img = fill(w, h);
        self.draw_centered(&mut img, "Top Actors", (h / 8) as i32, 56.0, GOLD);

        let margin = 80i32;
        let start_y = (h / 4) as i32;
        for (i, a) in report.top_actors.iter().take(5).enumerate() {
            let y = start_y + (i as i32) * 90;
            let rank = format!("{}.", i + 1);
            self.draw_left(&mut img, &rank, margin, y, 36.0, GOLD);
            self.draw_left(&mut img, &a.name, margin + 60, y, 36.0, WHITE);
            let detail = format!("{} films  avg {:.1}\u{2605}", a.count, a.avg_rating);
            self.draw_left(&mut img, &detail, margin + 60, y + 44, 24.0, DIM);
        }

        self.stamp_logo(&mut img);
        to_png(&img)
    }

    pub fn render_genres(
        &self,
        report: &WrapUpReport,
        w: u32,
        h: u32,
    ) -> Result<Vec<u8>, DomainError> {
        let mut img = fill(w, h);
        self.draw_centered(&mut img, "Genre Breakdown", (h / 8) as i32, 56.0, GOLD);

        let detail = format!("{} genres explored", report.genre_diversity);
        self.draw_centered(&mut img, &detail, (h / 8) as i32 + 64, 28.0, DIM);

        let margin = 80i32;
        let bar_area_w = (w as i32 - margin * 2 - 200) as u32;
        let start_y = (h / 4) as i32;
        let max_count = report.top_genres.first().map(|g| g.count).unwrap_or(1).max(1);

        for (i, g) in report.top_genres.iter().take(8).enumerate() {
            let y = start_y + (i as i32) * 80;
            self.draw_left(&mut img, &g.genre, margin, y, 30.0, WHITE);
            let count_str = format!("{}", g.count);
            self.draw_left(&mut img, &count_str, w as i32 - margin - 40, y, 30.0, DIM);

            let bar_y = y + 38;
            let bar_w = (g.count as f64 / max_count as f64 * bar_area_w as f64) as u32;
            draw_filled_rect_mut(
                &mut img,
                Rect::at(margin, bar_y).of_size(bar_area_w, 12),
                BAR_BG,
            );
            if bar_w > 0 {
                draw_filled_rect_mut(
                    &mut img,
                    Rect::at(margin, bar_y).of_size(bar_w, 12),
                    GOLD,
                );
            }
        }

        if let Some(ref best) = report.highest_rated_genre {
            let text = format!("Highest rated: {best}");
            self.draw_centered(&mut img, &text, h as i32 - 180, 28.0, WHITE);
        }
        if let Some(ref worst) = report.lowest_rated_genre {
            let text = format!("Lowest rated: {worst}");
            self.draw_centered(&mut img, &text, h as i32 - 140, 28.0, DIM);
        }

        self.stamp_logo(&mut img);
        to_png(&img)
    }

    pub fn render_highlights(
        &self,
        report: &WrapUpReport,
        w: u32,
        h: u32,
    ) -> Result<Vec<u8>, DomainError> {
        let mut img = fill(w, h);
        self.draw_centered(&mut img, "Highlights", (h / 10) as i32, 56.0, GOLD);

        // 2-column layout of notable movies
        let col_w = w / 2;
        let start_y = (h / 5) as i32;
        let row_h = (h / 5) as i32;
        let left = 60i32;
        let right = col_w as i32 + 40;

        let items: Vec<(&str, Option<&domain::models::wrapup::MovieRef>)> = vec![
            ("Highest Rated", report.highest_rated_movie.as_ref()),
            ("Lowest Rated", report.lowest_rated_movie.as_ref()),
            ("Oldest Film", report.oldest_movie.as_ref()),
            ("Newest Film", report.newest_movie.as_ref()),
            ("Longest", report.longest_movie.as_ref()),
            ("Shortest", report.shortest_movie.as_ref()),
            ("First Watched", report.first_movie_of_period.as_ref()),
            ("Last Watched", report.last_movie_of_period.as_ref()),
        ];

        for (i, (label, movie_ref)) in items.iter().enumerate() {
            let col = i % 2;
            let row = i / 2;
            let x = if col == 0 { left } else { right };
            let y = start_y + (row as i32) * row_h;

            self.draw_left(&mut img, label, x, y, 28.0, GOLD);
            if let Some(m) = movie_ref {
                let title = if m.title.len() > 22 {
                    format!("{}...", &m.title[..19])
                } else {
                    m.title.clone()
                };
                self.draw_left(&mut img, &title, x, y + 36, 26.0, WHITE);
                let sub = format!("({})", m.year);
                self.draw_left(&mut img, &sub, x, y + 68, 22.0, DIM);
            } else {
                self.draw_left(&mut img, "-", x, y + 36, 26.0, DIM);
            }
        }

        // Rewatches
        if report.total_rewatches > 0 {
            let rewatch_y = start_y + 4 * row_h + 20;
            let s = format!("{} rewatches", report.total_rewatches);
            self.draw_centered(&mut img, &s, rewatch_y, 30.0, DIM);
            if let Some(ref m) = report.most_rewatched_movie {
                let s2 = format!("Most rewatched: {}", m.title);
                self.draw_centered(&mut img, &s2, rewatch_y + 40, 26.0, WHITE);
            }
        }

        self.stamp_logo(&mut img);
        to_png(&img)
    }

    pub fn render_mosaic(
        &self,
        posters: &[(String, Vec<u8>)],
        w: u32,
        h: u32,
    ) -> Result<Vec<u8>, DomainError> {
        let mut canvas = fill(w, h);

        let cols = 4u32;
        let thumb_w = w / cols;
        let thumb_h = (thumb_w * 3) / 2;

        for (i, (_, bytes)) in posters.iter().enumerate() {
            let col = (i as u32) % cols;
            let row = (i as u32) / cols;
            let x = col * thumb_w;
            let y = row * thumb_h;
            if y + thumb_h > h {
                break;
            }
            if let Ok(poster) = image::load_from_memory(bytes) {
                let thumb =
                    poster.resize_exact(thumb_w, thumb_h, image::imageops::FilterType::Triangle);
                image::imageops::overlay(&mut canvas, &thumb.to_rgba8(), x as i64, y as i64);
            }
        }

        self.stamp_logo(&mut canvas);
        to_png(&canvas)
    }
}

fn fill(w: u32, h: u32) -> RgbaImage {
    RgbaImage::from_pixel(w, h, BG)
}

fn to_png(img: &RgbaImage) -> Result<Vec<u8>, DomainError> {
    let mut buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
    Ok(buf)
}

fn load_system_font() -> Result<FontArc, DomainError> {
    let candidates = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf",
        "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/System/Library/Fonts/Helvetica.ttc",
    ];
    for path in &candidates {
        if let Ok(bytes) = std::fs::read(path)
            && let Ok(font) = FontArc::try_from_vec(bytes)
        {
            tracing::info!("loaded system font: {path}");
            return Ok(font);
        }
    }
    Err(DomainError::InfrastructureError(
        "no system font found; set font_path in VideoRenderConfig or WRAPUP_FONT_PATH env"
            .to_string(),
    ))
}
