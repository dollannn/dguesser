//! Shared spread-ranking helpers for round location selection.

use rand::RngExt;

use crate::geo::distance::haversine_distance;

use super::SelectionConstraints;

/// Candidates within 90% of the best spread score are considered near-best.
const SHORTLIST_SCORE_RATIO: f64 = 0.9;

/// Limit randomness to a small pool of top spread candidates.
const MAX_SHORTLIST_CANDIDATES: usize = 4;

/// Result of selecting a spread-ranked candidate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpreadSelection {
    /// Index into the original candidate slice.
    pub selected_index: usize,
    /// Minimum distance from the chosen candidate to previous round locations.
    pub spread_score_meters: f64,
    /// Whether the chosen candidate satisfied the optional hard minimum distance.
    pub met_min_distance: bool,
    /// Number of candidates in the near-best random shortlist.
    pub shortlisted_candidates: usize,
}

/// Select a candidate using relative spread ranking with a bit of randomness.
///
/// Candidates are scored by their minimum distance to any previous round location.
/// The best spread candidates are preferred, but the final choice is randomized
/// among a small near-best shortlist to avoid deterministic repetition.
pub fn select_spread_candidate(
    candidates: &[(f64, f64)],
    constraints: &SelectionConstraints,
) -> Option<SpreadSelection> {
    if candidates.is_empty() || constraints.previous_locations.is_empty() {
        return None;
    }

    let mut scored: Vec<(usize, f64, bool)> = candidates
        .iter()
        .enumerate()
        .map(|(idx, (lat, lng))| {
            let spread_score =
                min_distance_to_previous(*lat, *lng, &constraints.previous_locations)
                    .unwrap_or(0.0);
            let meets_min_distance = !constraints.has_hard_min_distance()
                || spread_score >= constraints.min_distance_meters;

            (idx, spread_score, meets_min_distance)
        })
        .collect();

    if constraints.has_hard_min_distance() && scored.iter().any(|(_, _, ok)| *ok) {
        scored.retain(|(_, _, ok)| *ok);
    }

    scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let best_score = scored.first()?.1;
    let shortlist_threshold = best_score * SHORTLIST_SCORE_RATIO;

    let mut shortlist: Vec<(usize, f64)> = scored
        .iter()
        .filter(|(_, score, _)| *score >= shortlist_threshold)
        .take(MAX_SHORTLIST_CANDIDATES)
        .map(|(idx, score, _)| (*idx, *score))
        .collect();

    if shortlist.is_empty() {
        shortlist.push((scored[0].0, scored[0].1));
    }

    let mut rng = rand::rng();
    let selected = shortlist[rng.random_range(0..shortlist.len())];

    Some(SpreadSelection {
        selected_index: selected.0,
        spread_score_meters: selected.1,
        met_min_distance: !constraints.has_hard_min_distance()
            || selected.1 >= constraints.min_distance_meters,
        shortlisted_candidates: shortlist.len(),
    })
}

fn min_distance_to_previous(lat: f64, lng: f64, previous_locations: &[(f64, f64)]) -> Option<f64> {
    previous_locations
        .iter()
        .map(|(prev_lat, prev_lng)| haversine_distance(lat, lng, *prev_lat, *prev_lng))
        .min_by(|a, b| a.total_cmp(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_spread_candidate_prefers_clear_best() {
        let constraints = SelectionConstraints::with_previous_locations(vec![(0.0, 0.0)]);
        let candidates = [(0.2, 0.0), (4.0, 0.0), (1.0, 0.0)];

        let selection = select_spread_candidate(&candidates, &constraints).unwrap();

        assert_eq!(selection.selected_index, 1);
        assert!(selection.spread_score_meters > 400_000.0);
        assert_eq!(selection.shortlisted_candidates, 1);
    }

    #[test]
    fn test_select_spread_candidate_randomizes_within_near_best_pool() {
        let constraints = SelectionConstraints::with_previous_locations(vec![(0.0, 0.0)]);
        let candidates = [(0.0, 10.0), (0.0, 9.5), (0.0, 1.0)];

        let mut seen = std::collections::HashSet::new();
        for _ in 0..200 {
            let selection = select_spread_candidate(&candidates, &constraints).unwrap();
            assert!(selection.selected_index == 0 || selection.selected_index == 1);
            seen.insert(selection.selected_index);
        }

        assert!(seen.contains(&0));
        assert!(seen.contains(&1));
        assert!(!seen.contains(&2));
    }

    #[test]
    fn test_select_spread_candidate_respects_hard_min_distance() {
        let constraints = SelectionConstraints::with_min_distance(vec![(0.0, 0.0)], 500.0);
        let candidates = [(0.0, 6.0), (0.0, 3.0)];

        for _ in 0..20 {
            let selection = select_spread_candidate(&candidates, &constraints).unwrap();
            assert_eq!(selection.selected_index, 0);
            assert!(selection.met_min_distance);
        }
    }

    #[test]
    fn test_select_spread_candidate_falls_back_to_near_best_when_hard_min_unreachable() {
        let constraints = SelectionConstraints::with_min_distance(vec![(0.0, 0.0)], 1_500.0);
        let candidates = [(0.0, 10.0), (0.0, 9.3), (0.0, 3.0)];

        let mut seen = std::collections::HashSet::new();
        for _ in 0..200 {
            let selection = select_spread_candidate(&candidates, &constraints).unwrap();
            assert!(!selection.met_min_distance);
            assert!(selection.selected_index == 0 || selection.selected_index == 1);
            seen.insert(selection.selected_index);
        }

        assert!(seen.contains(&0));
        assert!(seen.contains(&1));
        assert!(!seen.contains(&2));
    }
}
