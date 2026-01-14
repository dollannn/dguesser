//! Distance calculation utilities

use std::f64::consts::PI;

/// Earth's radius in kilometers
const EARTH_RADIUS_KM: f64 = 6371.0;

/// Calculate the haversine distance between two points on Earth
///
/// Returns distance in kilometers
pub fn haversine_distance(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    let lat1_rad = lat1 * PI / 180.0;
    let lat2_rad = lat2 * PI / 180.0;
    let delta_lat = (lat2 - lat1) * PI / 180.0;
    let delta_lng = (lng2 - lng1) * PI / 180.0;

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lng / 2.0).sin().powi(2);

    let c = 2.0 * a.sqrt().asin();

    EARTH_RADIUS_KM * c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_point() {
        let distance = haversine_distance(0.0, 0.0, 0.0, 0.0);
        assert!(distance.abs() < 0.001);
    }

    #[test]
    fn test_known_distance() {
        // New York to London is approximately 5570 km
        let distance = haversine_distance(40.7128, -74.0060, 51.5074, -0.1278);
        assert!((distance - 5570.0).abs() < 50.0);
    }
}
