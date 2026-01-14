//! Scoring algorithms

use crate::geo::distance::haversine_distance;

/// Maximum possible score per round
pub const MAX_SCORE: u32 = 5000;

/// Calculate score based on distance from target
///
/// Uses an exponential decay formula similar to GeoGuessr
pub fn calculate_score(guess_lat: f64, guess_lng: f64, target_lat: f64, target_lng: f64) -> u32 {
    let distance_km = haversine_distance(guess_lat, guess_lng, target_lat, target_lng);

    // Perfect guess
    if distance_km < 0.025 {
        return MAX_SCORE;
    }

    // Exponential decay formula
    // Score decreases as distance increases
    let score = MAX_SCORE as f64 * (-distance_km / 2000.0).exp();

    score.round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_score() {
        let score = calculate_score(0.0, 0.0, 0.0, 0.0);
        assert_eq!(score, MAX_SCORE);
    }

    #[test]
    fn test_score_decreases_with_distance() {
        let close = calculate_score(0.0, 0.0, 0.1, 0.1);
        let far = calculate_score(0.0, 0.0, 10.0, 10.0);
        assert!(close > far);
    }
}
