//! Security headers middleware
//!
//! Adds defense-in-depth security headers to all API responses.
//! These headers help protect against common web vulnerabilities.

use axum::{
    body::Body,
    http::{Request, header::HeaderValue},
    middleware::Next,
    response::Response,
};

/// Security headers middleware
///
/// Adds the following headers to all responses:
/// - `X-Content-Type-Options: nosniff` - Prevents MIME type sniffing
/// - `X-Frame-Options: DENY` - Prevents clickjacking via framing
/// - `Referrer-Policy: strict-origin-when-cross-origin` - Controls referrer information
/// - `Content-Security-Policy: frame-ancestors 'none'` - Modern clickjacking protection
pub async fn security_headers(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    // Prevent MIME type sniffing attacks
    headers.insert("x-content-type-options", HeaderValue::from_static("nosniff"));

    // Prevent clickjacking by disallowing framing
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));

    // Control referrer information leakage
    headers.insert("referrer-policy", HeaderValue::from_static("strict-origin-when-cross-origin"));

    // Modern clickjacking protection (CSP frame-ancestors)
    headers.insert("content-security-policy", HeaderValue::from_static("frame-ancestors 'none'"));

    response
}
