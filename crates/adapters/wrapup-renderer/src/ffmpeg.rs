use domain::errors::DomainError;
use domain::ports::VideoRenderConfig;
use tokio::process::Command;

pub async fn stitch_slides(
    slides: &[Vec<u8>],
    config: &VideoRenderConfig,
) -> Result<Vec<u8>, DomainError> {
    let dir =
        tempfile::tempdir().map_err(|e| DomainError::InfrastructureError(e.to_string()))?;

    // Write slide PNGs
    for (i, png) in slides.iter().enumerate() {
        let path = dir.path().join(format!("slide_{:04}.png", i));
        std::fs::write(&path, png)
            .map_err(|e| DomainError::InfrastructureError(e.to_string()))?;
    }

    let output_path = dir.path().join("output.mp4");

    // -framerate 1/N makes each image last N seconds
    let framerate = format!("1/{}", config.slide_duration_secs);
    let (w, h) = config.resolution;

    let status = Command::new(&config.ffmpeg_path)
        .args([
            "-y",
            "-framerate",
            &framerate,
            "-i",
            &dir.path().join("slide_%04d.png").to_string_lossy(),
            "-vf",
            &format!("scale={}:{},format=yuv420p", w, h),
            "-c:v",
            "libx264",
            "-preset",
            "fast",
            "-crf",
            "23",
            "-movflags",
            "+faststart",
            &output_path.to_string_lossy(),
        ])
        .output()
        .await
        .map_err(|e| DomainError::InfrastructureError(format!("ffmpeg failed: {e}")))?;

    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr);
        return Err(DomainError::InfrastructureError(format!(
            "ffmpeg error: {stderr}"
        )));
    }

    std::fs::read(&output_path).map_err(|e| DomainError::InfrastructureError(e.to_string()))
}
