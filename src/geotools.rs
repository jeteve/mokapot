use h3o::{CellIndex, LatLng, Resolution};

// Average edge lengths (meters) for H3 resolutions 0..=15.
// Source: https://h3geo.org/docs/core-library/restable/
const EDGE_LENGTHS: [f64; 16] = [
    1_107_712.59,
    418_676.01,
    158_244.62,
    59_810.86,
    22_606.38,
    8_544.41,
    3_229.48,
    1_220.63,
    461.36,
    174.38,
    65.91,
    24.91,
    9.42,
    3.56,
    1.35,
    0.51,
];

/// Selects the coarsest resolution where the edge length is small enough
/// to fit `target_k` times within the given radius.
///
/// - `radius_m`: The search radius in meters.
/// - `target_k`: The minimum grid distance desired (controls granularity).
fn choose_resolution(radius_m: u64, target_k: u32) -> Resolution {
    if target_k == 0 {
        return Resolution::Zero;
    }

    let target_edge = radius_m as f64 / target_k as f64;

    let res_index = EDGE_LENGTHS
        .iter()
        .position(|&edge| edge <= target_edge)
        .unwrap_or(15); // Fallback to finest if radius is extremely small

    Resolution::try_from(res_index as u8).unwrap()
}

/// Generates a set of H3 cells covering a circular area.
/// The resolution is automatically adapted based on the radius.
///
/// - `lat`, `lng`: Center point coordinates.
/// - `radius_m`: Radius in meters.
/// - `target_k`: Desired density (approximate ring count).
///    ~4 is a good balance for shape accuracy vs performance.
pub(crate) fn get_adaptive_covering(
    lat: f64,
    lng: f64,
    radius_m: u64,
    target_k: u32,
) -> Vec<CellIndex> {
    let center = LatLng::new(lat, lng).expect("Invalid coordinates");
    let res = choose_resolution(radius_m, target_k);
    let edge_len = res.edge_length_m();

    // Calculate grid radius (k).
    // We add a buffer (+1) to account for grid distortion and edge cases.
    let k = (radius_m as f64 / edge_len).ceil() as u32 + 1;

    center
        .to_cell(res)
        .grid_disk::<Vec<_>>(k)
        .into_iter()
        .filter(|cell| {
            let cell_center = LatLng::from(*cell);
            center.distance_m(cell_center) <= radius_m as f64
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tiny_radius_selects_finest_resolution() {
        // 1m radius / 4 = 0.25m edge target.
        // Smallest H3 edge (Res 15) is ~0.51m.
        // 0.51 is > 0.25, so it should fall through to fallback (Res 15).
        let res = choose_resolution(1, 4);
        assert_eq!(res, Resolution::Fifteen);
    }

    #[test]
    fn test_huge_radius_selects_coarse_resolution() {
        // 1000km radius / 4 = 250km edge target.
        // Res 1 is ~418km (too big).
        // Res 2 is ~158km (fits!).
        // Expect Res 2.
        let res = choose_resolution(1_000_000, 4);
        assert_eq!(res, Resolution::Two);
    }

    #[test]
    fn test_medium_radius_city_scale() {
        // 500m radius / 4 = 125m edge target.
        // Res 9 is ~174m (too big).
        // Res 10 is ~65.9m (fits!).
        // Expect Res 10.
        let res = choose_resolution(500, 4);
        assert_eq!(res, Resolution::Ten);
    }

    #[test]
    fn test_zero_target_k_returns_safe_default() {
        // Should not panic or crash
        let res = choose_resolution(1000, 0);
        assert_eq!(res, Resolution::Zero);
    }

    #[test]
    fn test_covering_generates_cells() {
        // Integration test: Ensure we actually get cells back
        // Center of London, 500m radius
        let cells = get_adaptive_covering(54.35499723397377, 18.662987684795226, 50_000, 2);

        assert!(!cells.is_empty());
        println!(
            "The cells: {}",
            cells
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );

        // With k=4, we expect a decent number of cells (roughly > 30, < 100 usually)
        // Exact count varies by grid alignment, but shouldn't be 1.
        assert!(cells.len() > 1);
    }
}
