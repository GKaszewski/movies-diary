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
    pub fn from_env() -> anyhow::Result<Option<Self>> {
        Self::from_vars(
            std::env::var("IMAGE_CONVERSION_ENABLED").ok().as_deref(),
            std::env::var("IMAGE_CONVERSION_FORMAT").ok().as_deref(),
        )
    }

    fn from_vars(enabled: Option<&str>, format: Option<&str>) -> anyhow::Result<Option<Self>> {
        if enabled != Some("true") {
            return Ok(None);
        }

        let format_str = format.ok_or_else(|| {
            anyhow::anyhow!("IMAGE_CONVERSION_FORMAT required when IMAGE_CONVERSION_ENABLED=true")
        })?;

        let format = match format_str {
            "avif" => Format::Avif,
            "webp" => Format::Webp,
            other => anyhow::bail!(
                "Unknown IMAGE_CONVERSION_FORMAT: {other:?}. Valid values: avif, webp"
            ),
        };

        Ok(Some(Self { format }))
    }
}

#[cfg(test)]
#[path = "tests/config.rs"]
mod tests;
