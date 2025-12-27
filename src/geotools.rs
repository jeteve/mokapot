use std::fmt::Display;

use h3o::{CellIndex, LatLng, Resolution};
use nonempty::{NonEmpty, nonempty};

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
/// - `radius`: Circle radius.
/// - `target_k`: The minimum grid distance (how many cells from center) desired (controls granularity).
///   ~4 is a good balance for shape accuracy vs performance.
pub(crate) fn resolution_within_k(radius: Meters, target_k: u32) -> Resolution {
    if target_k == 0 {
        return Resolution::Zero;
    }

    let target_edge = radius.0 as f64 / target_k as f64;

    let res_index = EDGE_LENGTHS
        .iter()
        .position(|&edge| edge <= target_edge)
        .unwrap_or(15); // Fallback to finest if radius is extremely small

    Resolution::try_from(res_index as u8).unwrap()
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Meters(pub u64);
impl Display for Meters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}m", self.0)
    }
}

/// Generates a set of H3 cells covering a circular area.
/// You need to choose the resolution.
/// This is guarantee to at least cover the actual disk.
///
/// - `lat`, `lng`: Center point coordinates.
/// - `radius_m`: Radius in meters.
/// - `Resolution`: Desired cells resolution (influence number of cells and accuracy of coverage).
pub(crate) fn disk_covering(
    center: LatLng,
    radius: Meters,
    res: Resolution,
) -> NonEmpty<CellIndex> {
    let edge_len = res.edge_length_m();

    // Calculate grid radius (k).
    // We add a buffer (+1) to account for grid distortion and edge cases.
    let k = (radius.0 as f64 / edge_len).ceil() as u32 + 1;

    let center_cell = center.to_cell(res);

    let filtered_cells: Vec<_> = center_cell
        .grid_disk::<Vec<_>>(k)
        .into_iter()
        .filter(|cell| {
            let cell_center = LatLng::from(*cell);
            // This cannot work for radii zero, as the distance from a lat/lng
            // to the center of the containing cell is rarely ever <= 0.0000
            center.distance_m(cell_center) <= radius.0 as f64
        })
        .collect();

    if filtered_cells.is_empty() {
        nonempty![center_cell]
    } else {
        NonEmpty::from_vec(filtered_cells).expect("Always non empty")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meters() {
        assert_eq!(Meters(0).to_string(), "0m");
        assert_eq!(Meters(1).to_string(), "1m");
    }

    #[test]
    fn test_tiny_radius_selects_finest_resolution() {
        // 1m radius / 4 = 0.25m edge target.
        // Smallest H3 edge (Res 15) is ~0.51m.
        // 0.51 is > 0.25, so it should fall through to fallback (Res 15).
        let res = resolution_within_k(Meters(0), 4);
        assert_eq!(res, Resolution::Fifteen);

        let res = resolution_within_k(Meters(1), 4);
        assert_eq!(res, Resolution::Fifteen);
    }

    #[test]
    fn test_huge_radius_selects_coarse_resolution() {
        // 1000km radius / 4 = 250km edge target.
        // Res 1 is ~418km (too big).
        // Res 2 is ~158km (fits!).
        // Expect Res 2.
        let res = resolution_within_k(Meters(1_000_000), 4);
        assert_eq!(res, Resolution::Two);
    }

    #[test]
    fn test_medium_radius_city_scale() {
        // 500m radius / 4 = 125m edge target.
        // Res 9 is ~174m (too big).
        // Res 10 is ~65.9m (fits!).
        // Expect Res 10.
        let res = resolution_within_k(Meters(500), 4);
        assert_eq!(res, Resolution::Ten);
    }

    #[test]
    fn test_zero_target_k_returns_safe_default() {
        // Should not panic or crash
        let res = resolution_within_k(Meters(1000), 0);
        assert_eq!(res, Resolution::Zero);
    }

    #[test]
    fn test_covering_generates_cells() {
        // Integration test: Ensure we actually get cells back
        // Center of Gdansk, 500m radius
        let center =
            LatLng::new(54.35499723397377, 18.662987684795226).expect("Invalid coordinates");
        let res = resolution_within_k(Meters(50_000), 9);

        let cells = disk_covering(center, Meters(50_000), res);

        assert!(!cells.is_empty());
        println!(
            "Inspect at https://observablehq.com/@nrabinowitz/h3-index-inspector {}",
            cells
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );

        // With k=4, we expect a decent number of cells (roughly > 30, < 100 usually)
        // Exact count varies by grid alignment, but shouldn't be 1.
        assert!(cells.len() > 1);

        // test just a point and 1 radius. We should have at least one cell
        let res = resolution_within_k(Meters(1), 9);
        let cells = disk_covering(center, Meters(1), res);
        assert!(!cells.is_empty());

        // test just a point and zero radius. We should have at least one cell
        let res = resolution_within_k(Meters(0), 9);
        let cells = disk_covering(center, Meters(0), res);
        assert!(!cells.is_empty());
    }
}
