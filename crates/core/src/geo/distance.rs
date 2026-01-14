//! Distance calculation utilities

const EARTH_RADIUS_METERS: f64 = 6_371_000.0;

/// Calculate the distance between two points using the Haversine formula.
/// Returns distance in meters.
pub fn haversine_distance(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lng = (lng2 - lng1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lng / 2.0).sin().powi(2);

    let c = 2.0 * a.sqrt().asin();

    EARTH_RADIUS_METERS * c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_point() {
        let dist = haversine_distance(51.5074, -0.1278, 51.5074, -0.1278);
        assert!(dist < 0.001);
    }

    #[test]
    fn test_london_to_paris() {
        // London to Paris is approximately 344 km
        let dist = haversine_distance(51.5074, -0.1278, 48.8566, 2.3522);
        assert!((dist - 344_000.0).abs() < 5000.0);
    }

    #[test]
    fn test_new_york_to_london() {
        // New York to London is approximately 5570 km
        let dist = haversine_distance(40.7128, -74.0060, 51.5074, -0.1278);
        assert!((dist - 5_570_000.0).abs() < 50_000.0);
    }
}
