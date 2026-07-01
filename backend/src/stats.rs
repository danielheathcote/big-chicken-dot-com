//! Small statistics helpers for turning ensemble members into percentiles.

/// Linear-interpolation percentile over a slice that is already sorted ascending.
///
/// `q` is a quantile in `[0.0, 1.0]` (e.g. `0.5` for the median). Uses the same
/// "linear" method as NumPy's default, so `percentile(xs, 0.5)` is the median.
/// Returns `None` for an empty slice.
pub fn percentile(sorted: &[f64], q: f64) -> Option<f64> {
    match sorted.len() {
        0 => None,
        1 => Some(sorted[0]),
        n => {
            let q = q.clamp(0.0, 1.0);
            let rank = q * (n - 1) as f64;
            let lo = rank.floor() as usize;
            let hi = rank.ceil() as usize;
            let frac = rank - lo as f64;
            Some(sorted[lo] + (sorted[hi] - sorted[lo]) * frac)
        }
    }
}

/// Sort a collection of (possibly missing) values ascending, dropping `NaN`/missing.
pub fn sorted_finite(values: impl IntoIterator<Item = Option<f64>>) -> Vec<f64> {
    let mut v: Vec<f64> = values
        .into_iter()
        .flatten()
        .filter(|x| x.is_finite())
        .collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_none() {
        assert_eq!(percentile(&[], 0.5), None);
    }

    #[test]
    fn single_value() {
        assert_eq!(percentile(&[7.0], 0.25), Some(7.0));
    }

    #[test]
    fn median_odd() {
        assert_eq!(percentile(&[1.0, 2.0, 3.0], 0.5), Some(2.0));
    }

    #[test]
    fn median_even_interpolates() {
        // Between 2.0 and 3.0 -> 2.5
        assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0], 0.5), Some(2.5));
    }

    #[test]
    fn quartiles() {
        let xs = [0.0, 1.0, 2.0, 3.0, 4.0];
        assert_eq!(percentile(&xs, 0.0), Some(0.0));
        assert_eq!(percentile(&xs, 0.25), Some(1.0));
        assert_eq!(percentile(&xs, 0.75), Some(3.0));
        assert_eq!(percentile(&xs, 1.0), Some(4.0));
    }

    #[test]
    fn monotonic_percentiles() {
        let xs = [0.1, 0.4, 0.4, 0.9, 2.0, 5.5];
        let p25 = percentile(&xs, 0.25).unwrap();
        let p50 = percentile(&xs, 0.50).unwrap();
        let p75 = percentile(&xs, 0.75).unwrap();
        assert!(p25 <= p50 && p50 <= p75, "expected p25<=p50<=p75, got {p25} {p50} {p75}");
    }

    #[test]
    fn sorted_finite_drops_missing() {
        let out = sorted_finite([Some(3.0), None, Some(1.0), Some(f64::NAN), Some(2.0)]);
        assert_eq!(out, vec![1.0, 2.0, 3.0]);
    }
}
