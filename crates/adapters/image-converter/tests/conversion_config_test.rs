use image_converter::{ConversionConfig, Format};

fn section(enabled: bool, format: Option<&str>) -> config::ImageConversionConfig {
    config::ImageConversionConfig {
        enabled,
        format: format.map(str::to_owned),
    }
}

fn convert(enabled: bool, format: Option<&str>) -> anyhow::Result<Option<ConversionConfig>> {
    ConversionConfig::from_config(&section(enabled, format))
}

#[test]
fn is_disabled_by_default() {
    assert!(
        ConversionConfig::from_config(&config::ImageConversionConfig::default())
            .unwrap()
            .is_none()
    );
    assert!(convert(false, Some("webp")).unwrap().is_none());
}

#[test]
fn enabled_with_avif_format() {
    let cfg = convert(true, Some("avif")).unwrap().unwrap();
    assert_eq!(cfg.format, Format::Avif);
}

#[test]
fn enabled_with_webp_format() {
    let cfg = convert(true, Some("webp")).unwrap().unwrap();
    assert_eq!(cfg.format, Format::Webp);
}

#[test]
fn unknown_format_is_rejected() {
    assert!(convert(true, Some("gif")).is_err());
}

#[test]
fn missing_format_when_enabled_is_rejected() {
    assert!(convert(true, None).is_err());
}

#[test]
fn avif_extension_is_dot_avif() {
    assert_eq!(Format::Avif.extension(), ".avif");
}

#[test]
fn webp_extension_is_dot_webp() {
    assert_eq!(Format::Webp.extension(), ".webp");
}
