use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_tracing(default_directives: &str) {
    tracing_subscriber::registry()
        .with(log_filter(default_directives))
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// `RUST_LOG` names the crates to trace in detail; anything it doesn't name
/// still reports warnings and errors. Without a baseline, a filter like
/// `http_axum=debug` silently drops adapter errors (e.g. the sqlx detail
/// behind "Database operation failed").
fn log_filter(default: &str) -> tracing_subscriber::EnvFilter {
    let directives = std::env::var("RUST_LOG").unwrap_or_else(|_| default.to_string());
    let has_baseline = directives
        .split(',')
        .any(|d| !d.trim().is_empty() && !d.contains('='));
    if has_baseline {
        tracing_subscriber::EnvFilter::new(directives)
    } else {
        tracing_subscriber::EnvFilter::new(format!("warn,{directives}"))
    }
}
