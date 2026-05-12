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
mod tests {
    use super::*;

    #[test]
    fn disabled_by_default() {
        assert!(ConversionConfig::from_vars(None, None).unwrap().is_none());
        assert!(ConversionConfig::from_vars(Some("false"), None).unwrap().is_none());
    }

    #[test]
    fn enabled_avif() {
        let cfg = ConversionConfig::from_vars(Some("true"), Some("avif")).unwrap().unwrap();
        assert_eq!(cfg.format, Format::Avif);
    }

    #[test]
    fn enabled_webp() {
        let cfg = ConversionConfig::from_vars(Some("true"), Some("webp")).unwrap().unwrap();
        assert_eq!(cfg.format, Format::Webp);
    }

    #[test]
    fn unknown_format_is_error() {
        assert!(ConversionConfig::from_vars(Some("true"), Some("gif")).is_err());
    }

    #[test]
    fn missing_format_when_enabled_is_error() {
        assert!(ConversionConfig::from_vars(Some("true"), None).is_err());
    }

    #[test]
    fn avif_extension() {
        assert_eq!(Format::Avif.extension(), ".avif");
    }

    #[test]
    fn webp_extension() {
        assert_eq!(Format::Webp.extension(), ".webp");
    }
}
