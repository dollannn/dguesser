//! Street View URL parsing utilities.
//!
//! This module provides utilities for parsing Google Maps Street View URLs
//! to extract coordinates, panorama IDs, and other metadata.

use thiserror::Error;

/// Errors that can occur during Street View URL parsing.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum StreetViewUrlError {
    #[error("Invalid URL format: {0}")]
    InvalidFormat(String),

    #[error("Missing coordinates in URL")]
    MissingCoordinates,

    #[error("Invalid latitude: {0}")]
    InvalidLatitude(String),

    #[error("Invalid longitude: {0}")]
    InvalidLongitude(String),

    #[error("Not a Street View URL")]
    NotStreetViewUrl,
}

/// Parsed information from a Street View URL.
#[derive(Debug, Clone, PartialEq)]
pub struct StreetViewUrlInfo {
    /// Latitude coordinate
    pub lat: f64,
    /// Longitude coordinate
    pub lng: f64,
    /// Panorama ID (if present in URL)
    pub panorama_id: Option<String>,
    /// Initial heading/direction (if present)
    pub heading: Option<f64>,
    /// Field of view/zoom (if present)
    pub fov: Option<f64>,
    /// Pitch/tilt (if present)
    pub pitch: Option<f64>,
}

/// Parse a Google Maps Street View URL to extract location information.
///
/// Supports various URL formats:
/// - `https://www.google.com/maps/@48.8584,2.2945,3a,75y,90t/data=...`
/// - `https://www.google.com/maps/@48.8584,2.2945,17z/data=!3m1!4b1!4m5...`
/// - `https://maps.google.com/...`
/// - Short URLs with coordinates embedded
///
/// # Examples
///
/// ```
/// use dguesser_core::streetview::parse_streetview_url;
///
/// let url = "https://www.google.com/maps/@48.8584,2.2945,3a,75y,90t/data=!1s...";
/// let info = parse_streetview_url(url).unwrap();
/// assert!((info.lat - 48.8584).abs() < 0.0001);
/// assert!((info.lng - 2.2945).abs() < 0.0001);
/// ```
pub fn parse_streetview_url(url: &str) -> Result<StreetViewUrlInfo, StreetViewUrlError> {
    let url = url.trim();

    // Check if it's a Google Maps URL
    if !url.contains("google.com/maps") && !url.contains("maps.google.com") {
        return Err(StreetViewUrlError::NotStreetViewUrl);
    }

    // Try to extract coordinates from the @ symbol pattern
    // Format: @lat,lng,... or @lat,lng
    let coords = extract_coordinates_from_at(url)?;

    // Try to extract panorama ID from data parameter
    let panorama_id = extract_panorama_id(url);

    // Try to extract view parameters (heading, fov, pitch)
    let (heading, fov, pitch) = extract_view_params(url);

    Ok(StreetViewUrlInfo { lat: coords.0, lng: coords.1, panorama_id, heading, fov, pitch })
}

/// Extract coordinates from the @lat,lng pattern in a URL.
fn extract_coordinates_from_at(url: &str) -> Result<(f64, f64), StreetViewUrlError> {
    // Find the @ symbol and extract coordinates
    let at_pos = url.find('@').ok_or(StreetViewUrlError::MissingCoordinates)?;

    let after_at = &url[at_pos + 1..];

    // Split by comma and take first two parts
    let parts: Vec<&str> = after_at.split(',').collect();
    if parts.len() < 2 {
        return Err(StreetViewUrlError::MissingCoordinates);
    }

    // Parse latitude (first part)
    let lat_str = parts[0].trim();
    let lat: f64 =
        lat_str.parse().map_err(|_| StreetViewUrlError::InvalidLatitude(lat_str.to_string()))?;

    // Validate latitude range
    if !(-90.0..=90.0).contains(&lat) {
        return Err(StreetViewUrlError::InvalidLatitude(format!(
            "{lat} is out of range [-90, 90]"
        )));
    }

    // Parse longitude (second part, may have suffix like 'z' or 'a')
    let lng_part = parts[1];
    // Remove any letter suffix (like 'z', 'a', 'y', 't', etc.)
    let lng_str: String =
        lng_part.chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-').collect();

    let lng: f64 =
        lng_str.parse().map_err(|_| StreetViewUrlError::InvalidLongitude(lng_part.to_string()))?;

    // Validate longitude range
    if !(-180.0..=180.0).contains(&lng) {
        return Err(StreetViewUrlError::InvalidLongitude(format!(
            "{lng} is out of range [-180, 180]"
        )));
    }

    Ok((lat, lng))
}

/// Simple percent-decode for URL parameters.
fn percent_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2
                && let Ok(byte) = u8::from_str_radix(&hex, 16)
            {
                result.push(byte as char);
                continue;
            }
            result.push('%');
            result.push_str(&hex);
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }

    result
}

/// Extract panorama ID from the data parameter.
/// Panorama IDs are typically in the format: !1s{panorama_id}!2e...
fn extract_panorama_id(url: &str) -> Option<String> {
    // Look for !1s pattern followed by the panorama ID
    if let Some(start) = url.find("!1s") {
        let after_marker = &url[start + 3..];
        // Find the end of the panorama ID (next ! or end of string)
        let end = after_marker.find('!').unwrap_or(after_marker.len());
        let pano_id = &after_marker[..end];

        // Panorama IDs are typically alphanumeric with some special chars
        if !pano_id.is_empty() && pano_id.len() < 100 {
            // URL decode if necessary
            let decoded = percent_decode(pano_id);
            return Some(decoded);
        }
    }

    // Alternative format: pano= parameter
    if let Some(start) = url.find("pano=") {
        let after_marker = &url[start + 5..];
        let end = after_marker.find('&').unwrap_or(after_marker.len());
        let pano_id = &after_marker[..end];

        if !pano_id.is_empty() {
            let decoded = percent_decode(pano_id);
            return Some(decoded);
        }
    }

    None
}

/// Extract view parameters (heading, fov, pitch) from URL.
/// Format: ...3a,75y,90t... where:
/// - 3a = ??? (some kind of mode marker)
/// - 75y = field of view (75 degrees)
/// - 90t = pitch (tilt)
/// - h= heading
fn extract_view_params(url: &str) -> (Option<f64>, Option<f64>, Option<f64>) {
    let mut heading = None;
    let mut fov = None;
    let mut pitch = None;

    // Look for parameters in the URL path after @
    if let Some(at_pos) = url.find('@') {
        let after_at = &url[at_pos + 1..];
        let parts: Vec<&str> =
            after_at.split(&['/', '?', '!'][..]).next().unwrap_or("").split(',').collect();

        for part in parts.iter().skip(2) {
            // Skip lat,lng
            if part.ends_with('y') {
                // FOV parameter
                if let Ok(v) = part.trim_end_matches('y').parse::<f64>() {
                    fov = Some(v);
                }
            } else if part.ends_with('t') {
                // Pitch/tilt parameter
                if let Ok(v) = part.trim_end_matches('t').parse::<f64>() {
                    pitch = Some(v);
                }
            } else if part.ends_with('h') {
                // Heading parameter
                if let Ok(v) = part.trim_end_matches('h').parse::<f64>() {
                    heading = Some(v);
                }
            }
        }
    }

    // Also check for explicit h= parameter in query string
    if heading.is_none()
        && let Some(start) = url.find("h=")
    {
        let after = &url[start + 2..];
        let end = after.find(&['&', '!'][..]).unwrap_or(after.len());
        if let Ok(v) = after[..end].parse::<f64>() {
            heading = Some(v);
        }
    }

    (heading, fov, pitch)
}

/// Parse multiple Street View URLs, returning successful parses and errors.
pub fn parse_streetview_urls(urls: &[&str]) -> Vec<Result<StreetViewUrlInfo, StreetViewUrlError>> {
    urls.iter().map(|url| parse_streetview_url(url)).collect()
}

/// Validate a batch of URLs and return only the valid ones with their info.
pub fn validate_streetview_urls<'a>(urls: &'a [&'a str]) -> Vec<(&'a str, StreetViewUrlInfo)> {
    urls.iter().filter_map(|url| parse_streetview_url(url).ok().map(|info| (*url, info))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_url() {
        let url = "https://www.google.com/maps/@48.8584,2.2945,3a,75y,90t/data=!3m7";
        let result = parse_streetview_url(url).unwrap();

        assert!((result.lat - 48.8584).abs() < 0.0001);
        assert!((result.lng - 2.2945).abs() < 0.0001);
    }

    #[test]
    fn test_parse_url_with_zoom() {
        let url = "https://www.google.com/maps/@40.7128,-74.0060,17z";
        let result = parse_streetview_url(url).unwrap();

        assert!((result.lat - 40.7128).abs() < 0.0001);
        assert!((result.lng - (-74.006)).abs() < 0.0001);
    }

    #[test]
    fn test_parse_negative_coordinates() {
        let url = "https://www.google.com/maps/@-33.8688,151.2093,15z";
        let result = parse_streetview_url(url).unwrap();

        assert!((result.lat - (-33.8688)).abs() < 0.0001);
        assert!((result.lng - 151.2093).abs() < 0.0001);
    }

    #[test]
    fn test_extract_fov_and_pitch() {
        let url = "https://www.google.com/maps/@48.8584,2.2945,3a,75y,90t/data=!3m7";
        let result = parse_streetview_url(url).unwrap();

        assert_eq!(result.fov, Some(75.0));
        assert_eq!(result.pitch, Some(90.0));
    }

    #[test]
    fn test_invalid_url_not_google() {
        let url = "https://www.example.com/maps/@48.8584,2.2945";
        let result = parse_streetview_url(url);

        assert!(matches!(result, Err(StreetViewUrlError::NotStreetViewUrl)));
    }

    #[test]
    fn test_invalid_url_no_coordinates() {
        let url = "https://www.google.com/maps/place/Paris";
        let result = parse_streetview_url(url);

        assert!(matches!(result, Err(StreetViewUrlError::MissingCoordinates)));
    }

    #[test]
    fn test_latitude_out_of_range() {
        let url = "https://www.google.com/maps/@91.0,2.2945,3a";
        let result = parse_streetview_url(url);

        assert!(matches!(result, Err(StreetViewUrlError::InvalidLatitude(_))));
    }

    #[test]
    fn test_longitude_out_of_range() {
        let url = "https://www.google.com/maps/@48.8584,181.0,3a";
        let result = parse_streetview_url(url);

        assert!(matches!(result, Err(StreetViewUrlError::InvalidLongitude(_))));
    }

    #[test]
    fn test_parse_multiple_urls() {
        let urls = vec![
            "https://www.google.com/maps/@48.8584,2.2945,3a",
            "https://invalid.com/test",
            "https://www.google.com/maps/@40.7128,-74.0060,17z",
        ];

        let results = parse_streetview_urls(&urls);

        assert!(results[0].is_ok());
        assert!(results[1].is_err());
        assert!(results[2].is_ok());
    }

    #[test]
    fn test_validate_urls() {
        let urls = vec![
            "https://www.google.com/maps/@48.8584,2.2945,3a",
            "https://invalid.com/test",
            "https://www.google.com/maps/@40.7128,-74.0060,17z",
        ];

        let valid = validate_streetview_urls(&urls);

        assert_eq!(valid.len(), 2);
    }

    #[test]
    fn test_extract_panorama_id() {
        let url = "https://www.google.com/maps/@48.8584,2.2945,3a,75y/data=!1sABCD1234!2e0";
        let result = parse_streetview_url(url).unwrap();

        assert_eq!(result.panorama_id, Some("ABCD1234".to_string()));
    }
}
