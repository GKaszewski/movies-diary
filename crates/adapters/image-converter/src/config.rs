#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Format {
    Avif,
    Webp,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Format::Avif => ".avif",
            Format::Webp => ".webp",
        }
    }
}

pub struct ConversionConfig {
    pub format: Format,
}

impl ConversionConfig {
    pub fn from_config(config: &::config::ImageConversionConfig) -> anyhow::Result<Option<Self>> {
        if !config.enabled {
            return Ok(None);
        }

        let format = match config.format.as_deref() {
            Some("avif") => Format::Avif,
            Some("webp") => Format::Webp,
            Some(other) => anyhow::bail!(
                "Unknown IMAGE_CONVERSION_FORMAT: {other:?}. Valid values: avif, webp"
            ),
            None => {
                anyhow::bail!("IMAGE_CONVERSION_FORMAT required when IMAGE_CONVERSION_ENABLED=true")
            }
        };

        Ok(Some(Self { format }))
    }
}
