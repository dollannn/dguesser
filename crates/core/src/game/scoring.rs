//! Scoring algorithms

/// Scoring configuration
#[derive(Debug, Clone)]
pub struct ScoringConfig {
    /// Maximum points possible per round
    pub max_points: u32,
    /// Distance at which score becomes 0 (in meters)
    pub zero_score_distance: f64,
    /// Scoring curve exponent (higher = steeper dropoff)
    pub curve_exponent: f64,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            max_points: 5000,
            zero_score_distance: 5_000_000.0, // 5,000 km - roughly continent-scale
            curve_exponent: 1.5,              // Steeper dropoff for far guesses
        }
    }
}

/// Calculate score based on distance from target.
/// Uses exponential decay formula similar to GeoGuessr.
pub fn calculate_score(distance_meters: f64, config: &ScoringConfig) -> u32 {
    if distance_meters <= 0.0 {
        return config.max_points;
    }

    if distance_meters >= config.zero_score_distance {
        return 0;
    }

    // Exponential decay: score = max * (1 - (distance / max_distance)^exponent)
    let ratio = distance_meters / config.zero_score_distance;
    let decay = ratio.powf(config.curve_exponent);
    let score = (config.max_points as f64) * (1.0 - decay);

    score.round() as u32
}

/// Alternative scoring: logarithmic decay (more forgiving at close distances)
pub fn calculate_score_logarithmic(distance_meters: f64, config: &ScoringConfig) -> u32 {
    if distance_meters <= 1.0 {
        return config.max_points;
    }

    if distance_meters >= config.zero_score_distance {
        return 0;
    }

    let log_dist = distance_meters.ln();
    let log_max = config.zero_score_distance.ln();
    let ratio = log_dist / log_max;
    let score = (config.max_points as f64) * (1.0 - ratio);

    score.max(0.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_guess() {
        let config = ScoringConfig::default();
        assert_eq!(calculate_score(0.0, &config), 5000);
    }

    #[test]
    fn test_far_guess() {
        let config = ScoringConfig::default();
        assert_eq!(calculate_score(5_000_001.0, &config), 0);
    }

    #[test]
    fn test_close_guess() {
        let config = ScoringConfig::default();
        // Very close guess should get near max points
        let score = calculate_score(100.0, &config);
        assert!(score > 4900);
    }

    #[test]
    fn test_score_decreases_with_distance() {
        let config = ScoringConfig::default();
        // Use distances large enough to show meaningful score differences
        let close = calculate_score(1_000_000.0, &config); // 1000 km
        let far = calculate_score(4_000_000.0, &config); // 4000 km
        assert!(close > far);
    }

    #[test]
    fn test_logarithmic_perfect_guess() {
        let config = ScoringConfig::default();
        assert_eq!(calculate_score_logarithmic(0.5, &config), 5000);
    }

    #[test]
    fn test_logarithmic_far_guess() {
        let config = ScoringConfig::default();
        assert_eq!(calculate_score_logarithmic(5_000_001.0, &config), 0);
    }

    #[test]
    fn test_continent_scale_scoring() {
        let config = ScoringConfig::default();

        // Close guess (same city): near max points
        let same_city = calculate_score(50_000.0, &config); // 50 km
        assert!(same_city > 4900, "50km should give >4900 pts, got {same_city}");

        // Same country, different city: still good points
        let same_country = calculate_score(500_000.0, &config); // 500 km
        assert!(same_country > 4500, "500km should give >4500 pts, got {same_country}");

        // Same continent, far away: moderate points
        let same_continent = calculate_score(2_000_000.0, &config); // 2000 km
        assert!(
            same_continent > 3000 && same_continent < 4500,
            "2000km should give 3000-4500 pts, got {same_continent}"
        );

        // Different continent: low points
        let diff_continent = calculate_score(4_000_000.0, &config); // 4000 km
        assert!(diff_continent < 2000, "4000km should give <2000 pts, got {diff_continent}");

        // Other side of world: zero points
        let other_side = calculate_score(5_000_000.0, &config); // 5000 km
        assert_eq!(other_side, 0, "5000km+ should give 0 pts");
    }
}
