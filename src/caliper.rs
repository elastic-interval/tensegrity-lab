use crate::units::Meters;

/// Calibration data points: (input_length_mm, caliper_reading_mm)
/// The caliper holds a cord that is almost folded in half, so the reading
/// is approximately half the input length, but not exactly.
const CALIBRATION_MM: &[(f32, f32)] = &[
    (21.3, 12.0),
    (27.7, 15.0),
    (46.7, 25.0),
    (97.0, 50.0),
    (196.0, 100.0),
];

/// A model strut is a brass tube with the same screw in each end, adding
/// this much from the tube's end to the ring's centre.
const MODEL_SCREW_END_MM: f32 = 25.0;

/// The brass tube to cut for a model strut whose rings are this far apart.
pub fn brass_tube_mm(ring_to_ring_mm: f32) -> f32 {
    ring_to_ring_mm - 2.0 * MODEL_SCREW_END_MM
}

/// Convert an interval length to the expected caliper reading.
/// Uses linear interpolation between calibration data points.
/// Returns the reading as a string in millimeters with one decimal place.
pub fn caliper_reading(length: Meters) -> String {
    let input_mm = length.to_mm();

    if CALIBRATION_MM.is_empty() {
        // Fallback: approximate as half
        let reading = round_to_one_decimal(input_mm / 2.0);
        return format!("{:.1}", reading);
    }

    // Find the two calibration points to interpolate between
    let mut lower = None;
    let mut upper = None;

    for &(cal_input, cal_output) in CALIBRATION_MM {
        if cal_input <= input_mm {
            lower = Some((cal_input, cal_output));
        }
        if cal_input >= input_mm && upper.is_none() {
            upper = Some((cal_input, cal_output));
        }
    }

    let result = match (lower, upper) {
        (Some((x0, y0)), Some((x1, y1))) if (x1 - x0).abs() > f32::EPSILON => {
            // Linear interpolation
            let t = (input_mm - x0) / (x1 - x0);
            y0 + t * (y1 - y0)
        }
        (Some((_, y)), None) => {
            // Extrapolate beyond last point using the slope from last two points
            let n = CALIBRATION_MM.len();
            if n >= 2 {
                let (x0, y0) = CALIBRATION_MM[n - 2];
                let (x1, y1) = CALIBRATION_MM[n - 1];
                let slope = (y1 - y0) / (x1 - x0);
                y1 + slope * (input_mm - x1)
            } else {
                y
            }
        }
        (None, Some((_, y))) => y,    // Below first point, use first value
        (Some((_, y)), Some(_)) => y, // Same point
        (None, None) => input_mm / 2.0, // Fallback
    };

    format!("{:.1}", round_to_one_decimal(result))
}

fn round_to_one_decimal(value: f32) -> f32 {
    (value * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_caliper_reading_at_calibration_points() {
        assert_eq!(caliper_reading(Meters(0.0213)), "12.0");
        assert_eq!(caliper_reading(Meters(0.0277)), "15.0");
        assert_eq!(caliper_reading(Meters(0.0467)), "25.0");
        assert_eq!(caliper_reading(Meters(0.097)), "50.0");
        assert_eq!(caliper_reading(Meters(0.196)), "100.0");
    }

    #[test]
    fn test_caliper_reading_interpolation() {
        // Midway between 97 mm and 196 mm
        assert_eq!(caliper_reading(Meters(0.1465)), "75.0");
    }

    #[test]
    fn test_caliper_reading_below_knot_minimum() {
        // 12 mm is the shortest reading a knot can be tied at
        assert_eq!(caliper_reading(Meters(0.015)), "12.0");
    }

    #[test]
    fn test_caliper_reading_extrapolation() {
        // Slope of the last two points: 50/99; at 220 mm: 100 + 24 * 50/99
        assert_eq!(caliper_reading(Meters(0.220)), "112.1");
    }

    #[test]
    fn test_brass_tube() {
        // Measured: a 100 mm tube with its screws puts the rings 150 mm apart
        assert_eq!(brass_tube_mm(150.0), 100.0);
    }

    #[test]
    fn test_round_to_one_decimal() {
        assert_eq!(round_to_one_decimal(48.54), 48.5);
        assert_eq!(round_to_one_decimal(48.55), 48.6);
        assert_eq!(round_to_one_decimal(48.56), 48.6);
    }
}
