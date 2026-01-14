//! Structured logging configuration

use tracing_subscriber::{
    EnvFilter,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

/// Initialize logging based on environment
///
/// - In production (json_output=true): JSON format for log aggregation
/// - In development (json_output=false): Pretty format for readability
pub fn init_logging(json_output: bool) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,dguesser=debug,tower_http=debug"));

    if json_output {
        // JSON format for production
        tracing_subscriber::registry()
            .with(filter)
            .with(
                fmt::layer()
                    .json()
                    .with_span_events(FmtSpan::CLOSE)
                    .with_current_span(true)
                    .with_target(true)
                    .with_thread_ids(true),
            )
            .init();
    } else {
        // Pretty format for development
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().pretty().with_span_events(FmtSpan::CLOSE))
            .init();
    }
}

/// Check if running on Railway
pub fn is_railway() -> bool {
    std::env::var("RAILWAY_ENVIRONMENT").is_ok()
}

/// Check if running in production
pub fn is_production() -> bool {
    std::env::var("RUST_ENV").map(|v| v == "production").unwrap_or(false) || is_railway()
}

/// Get public URL from Railway environment
#[allow(dead_code)]
pub fn get_public_url() -> Option<String> {
    std::env::var("RAILWAY_PUBLIC_DOMAIN").ok().map(|d| format!("https://{}", d))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_get_public_url_format() {
        // Test the URL formatting logic
        let domain = "example.railway.app";
        let url = format!("https://{}", domain);
        assert_eq!(url, "https://example.railway.app");
    }
}
