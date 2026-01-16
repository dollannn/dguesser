//! API middleware

pub mod client_ip;
pub mod rate_limit;

pub use client_ip::extract_ip_from_headers;
pub use rate_limit::rate_limit;
