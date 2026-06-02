use ab_glyph::{FontArc, PxScale};
use domain::errors::DomainError;
use domain::models::wrapup::WrapUpReport;
use image::{DynamicImage, Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut};
use imageproc::rect::Rect;

fn decode_image(bytes: &[u8]) -> Result<DynamicImage, String> {
    image::load_from_memory(bytes).or_else(|_| {
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let input = dir.path().join("input");
        let output = dir.path().join("output.png");
        std::fs::write(&input, bytes).map_err(|e| e.to_string())?;
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-y", "-i",
                &input.to_string_lossy(),
                &output.to_string_lossy(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("ffmpeg conversion failed".into());
        }
        let png_bytes = std::fs::read(&output).map_err(|e| e.to_string())?;
        image::load_from_memory(&png_bytes).map_err(|e| e.to_string())
    })
}

const BG: Rgba<u8> = Rgba([26, 26, 36, 255]);
const GOLD: Rgba<u8> = Rgba([229, 192, 52, 255]);
const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);
const DIM: Rgba<u8> = Rgba([255, 255, 255, 140]);
const BAR_BG: Rgba<u8> = Rgba([50, 50, 65, 255]);
const GLASS: Rgba<u8> = Rgba([20, 20, 30, 180]);
const GLASS_PADDING: u32 = 30;

pub struct SlideRenderer {
    font: FontArc,
    logo: Option<RgbaImage>,
    backgrounds: Vec<RgbaImage>,
}

impl SlideRenderer {
    pub fn new(
        font_path: Option<&str>,
        logo_path: Option<&str>,
        bg_dir: Option<&str>,
    ) -> Result<Self, DomainError> {
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

        let mut backgrounds = Vec::new();
        if let Some(dir) = bg_dir {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp") {
                        match image::open(&path) {
                            Ok(img) => backgrounds.push(img.to_rgba8()),
                            Err(e) => tracing::warn!("bg load {}: {e}", path.display()),
                        }
                    }
                }
            }
        }

        Ok(Self {
            font,
            logo,
            backgrounds,
        })
    }

    /// Pick a background for slide at `index`, resized to `w x h` with dark gradient overlay.
    fn pick_background(&self, index: usize, w: u32, h: u32) -> Option<RgbaImage> {
        if self.backgrounds.is_empty() {
            return None;
        }
        let bg = &self.backgrounds[index % self.backgrounds.len()];
        let resized = image::imageops::resize(bg, w, h, image::imageops::FilterType::Triangle);
        let mut out = resized;
        // darken top 40% and bottom 40% with gradient to ~70% black
        let top_cutoff = (h as f32 * 0.4) as u32;
        let bot_start = h - top_cutoff;
        for y in 0..h {
            let darken = if y < top_cutoff {
                // fade from 0.70 at top to 0.0 at cutoff
                0.70 * (1.0 - y as f32 / top_cutoff as f32)
            } else if y >= bot_start {
                // fade from 0.0 at bot_start to 0.70 at bottom
                0.70 * ((y - bot_start) as f32 / top_cutoff as f32)
            } else {
                0.0
            };
            if darken > 0.0 {
                let factor = 1.0 - darken;
                for x in 0..w {
                    let px = out.get_pixel_mut(x, y);
                    px[0] = (px[0] as f32 * factor) as u8;
                    px[1] = (px[1] as f32 * factor) as u8;
                    px[2] = (px[2] as f32 * factor) as u8;
                }
            }
        }
        Some(out)
    }

    /// Start a canvas: background image if available, else solid color.
    fn make_canvas(&self, slide_index: usize, w: u32, h: u32) -> RgbaImage {
        self.pick_background(slide_index, w, h)
            .unwrap_or_else(|| fill(w, h))
    }

    /// Draw a semi-transparent dark glass panel.
    fn draw_glass_panel(&self, canvas: &mut RgbaImage, x: i32, y: i32, pw: u32, ph: u32) {
        // clamp to canvas bounds
        let x0 = x.max(0) as u32;
        let y0 = y.max(0) as u32;
        let x1 = (x as u32 + pw).min(canvas.width());
        let y1 = (y as u32 + ph).min(canvas.height());
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        draw_filled_rect_mut(
            canvas,
            Rect::at(x0 as i32, y0 as i32).of_size(x1 - x0, y1 - y0),
            GLASS,
        );
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

    /// Draw a small thumbnail from raw image bytes, resized to `size x size`.
    fn draw_thumbnail(
        canvas: &mut RgbaImage,
        bytes: &[u8],
        x: i64,
        y: i64,
        tw: u32,
        th: u32,
    ) {
        if let Ok(img) = decode_image(bytes) {
            let thumb = img.resize_exact(tw, th, image::imageops::FilterType::Triangle);
            image::imageops::overlay(canvas, &thumb.to_rgba8(), x, y);
        }
    }

    /// Find cast photo bytes matching `name` (case-insensitive substring).
    fn find_cast_photo<'a>(
        name: &str,
        cast_images: &'a [(String, Vec<u8>)],
    ) -> Option<&'a [u8]> {
        let lower = name.to_lowercase();
        cast_images
            .iter()
            .find(|(n, _)| {
                let cn = n.to_lowercase();
                cn.contains(&lower) || lower.contains(&cn)
            })
            .map(|(_, b)| b.as_slice())
    }

    /// Find poster bytes matching a poster_path (compare filename stem).
    fn find_poster<'a>(
        poster_path: &str,
        poster_images: &'a [(String, Vec<u8>)],
    ) -> Option<&'a [u8]> {
        let target = std::path::Path::new(poster_path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(poster_path);
        poster_images
            .iter()
            .find(|(p, _)| {
                let fname = std::path::Path::new(p)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or(p);
                fname == target
            })
            .map(|(_, b)| b.as_slice())
    }

    // ── Slides ──────────────────────────────────────────────

    pub fn render_hero(
        &self,
        report: &WrapUpReport,
        w: u32,
        h: u32,
    ) -> Result<Vec<u8>, DomainError> {
        let mut img = self.make_canvas(0, w, h);

        // glass panel in center area
        let panel_x = GLASS_PADDING as i32;
        let panel_y = (h / 7) as i32;
        let panel_w = w - GLASS_PADDING * 2;
        let panel_h = h * 5 / 7;
        self.draw_glass_panel(&mut img, panel_x, panel_y, panel_w, panel_h);

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
        let mut img = self.make_canvas(1, w, h);

        // glass panel covering content area
        let panel_x = (GLASS_PADDING / 2) as i32;
        let panel_y = (h / 10) as i32;
        let panel_w = w - GLASS_PADDING;
        let panel_h = h * 4 / 5;
        self.draw_glass_panel(&mut img, panel_x, panel_y, panel_w, panel_h);

        self.draw_centered(&mut img, "Ratings", (h / 8) as i32, 56.0, GOLD);

        if let Some(avg) = report.avg_rating {
            let s = format!("{:.1} / 5", avg);
            self.draw_centered(&mut img, &s, (h / 4) as i32, 80.0, WHITE);
            self.draw_centered(&mut img, "average rating", (h / 4 + 90) as i32, 32.0, DIM);
        }

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

            draw_filled_rect_mut(
                &mut img,
                Rect::at(margin_x, y).of_size(max_bar_w, bar_h),
                BAR_BG,
            );
            let fill_w = ((count as f32 / max_count as f32) * max_bar_w as f32) as u32;
            if fill_w > 0 {
                draw_filled_rect_mut(&mut img, Rect::at(margin_x, y).of_size(fill_w, bar_h), GOLD);
            }
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
        cast_images: &[(String, Vec<u8>)],
        w: u32,
        h: u32,
    ) -> Result<Vec<u8>, DomainError> {
        let mut img = self.make_canvas(2, w, h);

        let margin = 80i32;
        let start_y = (h / 4) as i32;
        let row_h = 100i32;
        let panel_h = (report.top_directors.len().min(5) as u32) * row_h as u32 + GLASS_PADDING * 2;
        self.draw_glass_panel(
            &mut img,
            margin - GLASS_PADDING as i32,
            start_y - GLASS_PADDING as i32,
            w - (margin as u32 - GLASS_PADDING) * 2,
            panel_h,
        );

        self.draw_centered(&mut img, "Top Directors", (h / 8) as i32, 56.0, GOLD);

        let thumb_size = 60u32;
        // offset text right when cast photos present
        let text_offset = if cast_images.is_empty() { 60 } else { thumb_size as i32 + 20 };

        for (i, d) in report.top_directors.iter().take(5).enumerate() {
            let y = start_y + (i as i32) * row_h;

            // cast photo thumbnail
            if let Some(photo) = Self::find_cast_photo(&d.name, cast_images) {
                Self::draw_thumbnail(
                    &mut img,
                    photo,
                    margin as i64 + 40,
                    y as i64,
                    thumb_size,
                    thumb_size,
                );
            }

            let rank = format!("{}.", i + 1);
            self.draw_left(&mut img, &rank, margin, y + 10, 36.0, GOLD);
            self.draw_left(
                &mut img,
                &d.name,
                margin + text_offset,
                y + 10,
                36.0,
                WHITE,
            );
            let detail = format!("{} films  avg {:.1}\u{2605}", d.count, d.avg_rating);
            self.draw_left(
                &mut img,
                &detail,
                margin + text_offset,
                y + 54,
                24.0,
                DIM,
            );
        }

        self.stamp_logo(&mut img);
        to_png(&img)
    }

    pub fn render_actors(
        &self,
        report: &WrapUpReport,
        cast_images: &[(String, Vec<u8>)],
        w: u32,
        h: u32,
    ) -> Result<Vec<u8>, DomainError> {
        let mut img = self.make_canvas(3, w, h);

        let margin = 80i32;
        let start_y = (h / 4) as i32;
        let row_h = 100i32;
        let panel_h = (report.top_actors.len().min(5) as u32) * row_h as u32 + GLASS_PADDING * 2;
        self.draw_glass_panel(
            &mut img,
            margin - GLASS_PADDING as i32,
            start_y - GLASS_PADDING as i32,
            w - (margin as u32 - GLASS_PADDING) * 2,
            panel_h,
        );

        self.draw_centered(&mut img, "Top Actors", (h / 8) as i32, 56.0, GOLD);

        let thumb_size = 60u32;
        let text_offset = if cast_images.is_empty() { 60 } else { thumb_size as i32 + 20 };

        for (i, a) in report.top_actors.iter().take(5).enumerate() {
            let y = start_y + (i as i32) * row_h;

            if let Some(photo) = Self::find_cast_photo(&a.name, cast_images) {
                Self::draw_thumbnail(
                    &mut img,
                    photo,
                    margin as i64 + 40,
                    y as i64,
                    thumb_size,
                    thumb_size,
                );
            }

            let rank = format!("{}.", i + 1);
            self.draw_left(&mut img, &rank, margin, y + 10, 36.0, GOLD);
            self.draw_left(
                &mut img,
                &a.name,
                margin + text_offset,
                y + 10,
                36.0,
                WHITE,
            );
            let detail = format!("{} films  avg {:.1}\u{2605}", a.count, a.avg_rating);
            self.draw_left(
                &mut img,
                &detail,
                margin + text_offset,
                y + 54,
                24.0,
                DIM,
            );
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
        let mut img = self.make_canvas(4, w, h);

        let margin = 80i32;
        let start_y = (h / 4) as i32;
        let num_genres = report.top_genres.len().min(8) as u32;
        let panel_h = num_genres * 80 + GLASS_PADDING * 2 + 80;
        self.draw_glass_panel(
            &mut img,
            margin - GLASS_PADDING as i32,
            (h / 10) as i32,
            w - (margin as u32 - GLASS_PADDING) * 2,
            panel_h + (start_y as u32 - h / 10),
        );

        self.draw_centered(&mut img, "Genre Breakdown", (h / 8) as i32, 56.0, GOLD);

        let detail = format!("{} genres explored", report.genre_diversity);
        self.draw_centered(&mut img, &detail, (h / 8) as i32 + 64, 28.0, DIM);

        let bar_area_w = (w as i32 - margin * 2 - 200) as u32;
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
        poster_images: &[(String, Vec<u8>)],
        w: u32,
        h: u32,
    ) -> Result<Vec<u8>, DomainError> {
        let mut img = self.make_canvas(5, w, h);

        // glass panel behind highlights grid
        let panel_x = GLASS_PADDING as i32;
        let panel_y = (h / 10) as i32;
        let panel_w = w - GLASS_PADDING * 2;
        let panel_h = h * 4 / 5;
        self.draw_glass_panel(&mut img, panel_x, panel_y, panel_w, panel_h);

        self.draw_centered(&mut img, "Highlights", (h / 10) as i32 + 10, 56.0, GOLD);

        let col_w = w / 2;
        let start_y = (h / 5) as i32;
        let row_h = (h / 5) as i32;
        let left = 60i32;
        let right = col_w as i32 + 40;
        let poster_w = 60u32;
        let poster_h = 90u32;

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

            // poster thumbnail if available
            let text_x_offset = if let Some(m) = movie_ref {
                if let Some(ref pp) = m.poster_path {
                    if let Some(pb) = Self::find_poster(pp, poster_images) {
                        Self::draw_thumbnail(
                            &mut img,
                            pb,
                            x as i64,
                            (y + 30) as i64,
                            poster_w,
                            poster_h,
                        );
                        poster_w as i32 + 10
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                0
            };

            self.draw_left(&mut img, label, x, y, 28.0, GOLD);
            if let Some(m) = movie_ref {
                let title = if m.title.len() > 22 {
                    format!("{}...", &m.title[..19])
                } else {
                    m.title.clone()
                };
                self.draw_left(
                    &mut img,
                    &title,
                    x + text_x_offset,
                    y + 36,
                    26.0,
                    WHITE,
                );
                let sub = format!("({})", m.year);
                self.draw_left(
                    &mut img,
                    &sub,
                    x + text_x_offset,
                    y + 68,
                    22.0,
                    DIM,
                );
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
        let mut canvas = RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 255]));

        // poster aspect 2:3, calculate grid to fill entire frame
        // find cols that best tile the width, then rows to fill height
        let poster_ratio = 2.0_f32 / 3.0;
        // try col counts from 3..8, pick one that wastes least space
        let cols = (3..=8)
            .min_by_key(|&c| {
                let tw = w / c;
                let th = (tw as f32 / poster_ratio) as u32;
                let rows_needed = (h + th - 1) / th;
                let total = rows_needed * c;
                // prefer filling screen with fewer leftover pixels
                let waste_y = (rows_needed * th).saturating_sub(h);
                let shortage = total.saturating_sub(posters.len() as u32);
                waste_y + shortage * 100
            })
            .unwrap_or(4);

        let thumb_w = w / cols;
        let thumb_h = (thumb_w as f32 / poster_ratio) as u32;
        let total_rows = (h + thumb_h - 1) / thumb_h;
        let total_cells = (total_rows * cols) as usize;

        for i in 0..total_cells {
            if posters.is_empty() {
                break;
            }
            // tile/repeat if not enough posters
            let idx = i % posters.len();
            let (name, bytes) = &posters[idx];
            let col = (i as u32) % cols;
            let row = (i as u32) / cols;
            let x = col * thumb_w;
            let y = row * thumb_h;

            match decode_image(bytes) {
                Ok(poster) => {
                    let thumb = poster.resize_exact(
                        thumb_w,
                        thumb_h,
                        image::imageops::FilterType::Triangle,
                    );
                    image::imageops::overlay(&mut canvas, &thumb.to_rgba8(), x as i64, y as i64);
                }
                Err(e) => tracing::debug!("mosaic: skipped {name}: {e}"),
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
