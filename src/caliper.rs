use crate::units::Meters;

/// Calibration data points: (input_length_mm, caliper_reading_mm)
/// The caliper holds a cord that is almost folded in half, so the reading
/// is approximately half the input length, but not exactly.
const CALIBRATION_MM : &[(f32, f32)] = &[
    (29.0, 15.0),
    (57.0, 30.0),
    (88.5, 45.0),
    (118.3, 60.0),
    (140.0, 70.9),
];

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
        (None, Some((_, y))) => y, // Below first point, use first value
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
        // Test exact calibration points
        assert_eq!(caliper_reading(Meters(0.029)), "15.0");
        assert_eq!(caliper_reading(Meters(0.057)), "30.0");
        assert_eq!(caliper_reading(Meters(0.0885)), "45.0");
        assert_eq!(caliper_reading(Meters(0.1183)), "60.0");
        assert_eq!(caliper_reading(Meters(0.140)), "70.9");
    }

    #[test]
    fn test_caliper_reading_interpolation() {
        // Test interpolation between calibration points
        // Between 57mm and 88.5mm (midpoint ~72.75mm should give ~37.5)
        let reading = caliper_reading(Meters(0.07275));
        assert_eq!(reading, "37.5");
    }

    #[test]
    fn test_caliper_reading_at_140mm() {
        // Verified by physical measurement: 140mm cord → 70.9mm caliper
        assert_eq!(caliper_reading(Meters(0.140)), "70.9");
    }

    #[test]
    fn test_caliper_reading_extrapolation() {
        // Test extrapolation beyond last calibration point (140mm)
        // Slope from last two points: (70.9-60)/(140-118.3) = 10.9/21.7 ≈ 0.502
        // At 160mm: 70.9 + 0.502 * (160 - 140) ≈ 70.9 + 10.0 ≈ 80.9
        let reading = caliper_reading(Meters(0.160));
        assert_eq!(reading, "80.9");
    }

    #[test]
    fn test_round_to_one_decimal() {
        assert_eq!(round_to_one_decimal(48.54), 48.5);
        assert_eq!(round_to_one_decimal(48.55), 48.6);
        assert_eq!(round_to_one_decimal(48.56), 48.6);
    }
}
