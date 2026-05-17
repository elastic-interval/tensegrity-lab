//! 1D k-center on |bend angles|: pick K manufactured magnitudes that minimise
//! the worst-case snap error. Tabs are flippable, so signed candidates ±m
//! both come from the same physical part — we only store magnitudes ≥ 0.

/// Whole-degree magnitudes (sorted, ≥ 0) minimising max ||x_i| − nearest m_j|.
/// Empty when `k == 0` or no input. K is clamped to `ideal_degrees.len()`.
pub fn optimize_magnitudes(ideal_degrees: &[f32], k: usize) -> Vec<f32> {
    if k == 0 || ideal_degrees.is_empty() {
        return Vec::new();
    }

    let mut abs_vals: Vec<f32> = ideal_degrees.iter().map(|x| x.abs()).collect();
    abs_vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = abs_vals.len();
    let k = k.min(n);

    // f[i][kk] = min max-cluster-radius over partitions of abs_vals[0..i] into kk clusters.
    let inf = f32::INFINITY;
    let mut f = vec![vec![inf; k + 1]; n + 1];
    let mut split = vec![vec![0usize; k + 1]; n + 1];
    f[0][0] = 0.0;

    for i in 1..=n {
        let max_kk = k.min(i);
        for kk in 1..=max_kk {
            for j in (kk - 1)..i {
                if f[j][kk - 1].is_finite() {
                    let cluster_radius = (abs_vals[i - 1] - abs_vals[j]) / 2.0;
                    let val = f[j][kk - 1].max(cluster_radius);
                    if val < f[i][kk] {
                        f[i][kk] = val;
                        split[i][kk] = j;
                    }
                }
            }
        }
    }

    let mut boundaries = Vec::with_capacity(k + 1);
    boundaries.push(n);
    let mut current_i = n;
    let mut current_k = k;
    while current_k > 0 {
        let j = split[current_i][current_k];
        boundaries.push(j);
        current_i = j;
        current_k -= 1;
    }
    boundaries.reverse();

    // Round each cluster midpoint to a whole degree, dedup collisions.
    let mut magnitudes: Vec<f32> = Vec::with_capacity(k);
    for c in 0..k {
        let lo = abs_vals[boundaries[c]];
        let hi = abs_vals[boundaries[c + 1] - 1];
        let mid = ((lo + hi) / 2.0).round();
        if !magnitudes.iter().any(|&m| (m - mid).abs() < 0.5) {
            magnitudes.push(mid);
        }
    }
    magnitudes
}

/// Snap to the nearest of `±m for m in magnitudes`. Returns `(signed_snap, |err|)`.
/// Empty magnitudes returns the ideal unchanged with zero error.
pub fn snap_to_magnitudes(ideal_degrees: f32, magnitudes: &[f32]) -> (f32, f32) {
    if magnitudes.is_empty() {
        return (ideal_degrees, 0.0);
    }

    let mut best_signed = f32::NAN;
    let mut best_err = f32::INFINITY;

    for &m in magnitudes {
        let plus_err = (ideal_degrees - m).abs();
        if plus_err < best_err {
            best_err = plus_err;
            best_signed = m;
        }
        if m != 0.0 {
            let minus = -m;
            let minus_err = (ideal_degrees - minus).abs();
            if minus_err < best_err {
                best_err = minus_err;
                best_signed = minus;
            }
        }
    }

    (best_signed, best_err)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn empty_input_returns_empty() {
        assert!(optimize_magnitudes(&[], 3).is_empty());
    }

    #[test]
    fn k_zero_returns_empty() {
        assert!(optimize_magnitudes(&[10.0, 20.0], 0).is_empty());
    }

    #[test]
    fn k1_picks_midpoint_of_abs_range() {
        let ideals = [10.0, 20.0, 30.0, 40.0, 50.0];
        let mags = optimize_magnitudes(&ideals, 1);
        assert_eq!(mags.len(), 1);
        assert!(
            approx(mags[0], 30.0, 1e-3),
            "expected midpoint 30, got {}",
            mags[0]
        );
        // Max error must equal half the range.
        let max_err = ideals
            .iter()
            .map(|&x| snap_to_magnitudes(x, &mags).1)
            .fold(0.0_f32, f32::max);
        assert!(approx(max_err, 20.0, 1e-3));
    }

    #[test]
    fn k_geq_unique_count_zero_error() {
        let ideals = [-10.0, 10.0, -20.0, 20.0, 30.0]; // 3 unique magnitudes
        let mags = optimize_magnitudes(&ideals, 5);
        for &x in &ideals {
            let (_, err) = snap_to_magnitudes(x, &mags);
            assert!(err < 1e-3, "expected zero error but got {} for {}", err, x);
        }
    }

    #[test]
    fn snap_respects_sign() {
        let mags = [0.0, 30.0];
        let (snapped, err) = snap_to_magnitudes(-25.0, &mags);
        assert!(approx(snapped, -30.0, 1e-3), "expected -30, got {}", snapped);
        assert!(approx(err, 5.0, 1e-3));
    }

    #[test]
    fn snap_to_zero_when_close() {
        let mags = [0.0, 30.0];
        let (snapped, err) = snap_to_magnitudes(5.0, &mags);
        assert!(approx(snapped, 0.0, 1e-3));
        assert!(approx(err, 5.0, 1e-3));
    }

    #[test]
    fn empty_magnitudes_returns_ideal_unchanged() {
        let (snapped, err) = snap_to_magnitudes(42.0, &[]);
        assert_eq!(snapped, 42.0);
        assert_eq!(err, 0.0);
    }

    #[test]
    fn larger_k_never_increases_max_error() {
        let ideals = [-60.0, -45.0, -20.0, 0.0, 15.0, 35.0, 55.0, 75.0];
        let max_err_for = |k: usize| {
            let mags = optimize_magnitudes(&ideals, k);
            ideals
                .iter()
                .map(|&x| snap_to_magnitudes(x, &mags).1)
                .fold(0.0_f32, f32::max)
        };
        let e1 = max_err_for(1);
        let e2 = max_err_for(2);
        let e3 = max_err_for(3);
        let e4 = max_err_for(4);
        assert!(e2 <= e1 + 1e-4, "k=2 ({}) should not exceed k=1 ({})", e2, e1);
        assert!(e3 <= e2 + 1e-4, "k=3 ({}) should not exceed k=2 ({})", e3, e2);
        assert!(e4 <= e3 + 1e-4, "k=4 ({}) should not exceed k=3 ({})", e4, e3);
    }

    #[test]
    fn optimal_set_matches_dp_radius() {
        // The actual max error after snapping must equal the DP's claim
        // (within float tolerance). We don't expose the DP value directly,
        // but we can sanity-check by trying small instances against brute force.
        let ideals = [10.0, 12.0, 50.0, 55.0, 90.0];
        let mags = optimize_magnitudes(&ideals, 2);
        // Best 2-cluster split on sorted [10,12,50,55,90]:
        //   {10,12} and {50,55,90}: max radius = max(1, 20) = 20
        //   {10,12,50} and {55,90}: max radius = max(20, 17.5) = 20
        //   {10,12,50,55} and {90}: max radius = max(22.5, 0) = 22.5
        // So optimal max error is 20.
        let max_err = ideals
            .iter()
            .map(|&x| snap_to_magnitudes(x, &mags).1)
            .fold(0.0_f32, f32::max);
        assert!(
            approx(max_err, 20.0, 1e-3),
            "expected max error 20, got {}",
            max_err
        );
    }
}
