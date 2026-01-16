//! API middleware

pub mod client_ip;
pub mod rate_limit;
pub mod security_headers;

pub use client_ip::extract_ip_from_headers;
pub use rate_limit::{rate_limit, rate_limit_auth, rate_limit_game};
pub use security_headers::security_headers;
