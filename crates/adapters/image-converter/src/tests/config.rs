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
