//! Decimal percentage validation at the command-line boundary.

use std::str::FromStr;

/// Finite decimal percentage between zero and one hundred, inclusive.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Percentage(f64);

impl Percentage {
    /// Return the validated percentage for threshold comparison.
    pub(crate) const fn value(self) -> f64 {
        self.0
    }
}

impl FromStr for Percentage {
    type Err = String;

    /// Accept digits with an optional fractional part and reject all other forms.
    fn from_str(source: &str) -> Result<Self, Self::Err> {
        // Validate lexical spelling before accepting floating-point input.
        // Keep the accepted grammar independent of floating-point parser extensions.
        let mut parts = source.split('.');
        let digits =
            |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
        let is_valid = parts.next().is_some_and(digits)
            && parts.next().is_none_or(digits)
            && parts.next().is_none();
        // Restrict the parsed number to a finite percentage.
        let value = source
            .parse::<f64>()
            .ok()
            .filter(|value| (0.0..=100.0).contains(value));
        if is_valid && let Some(value) = value {
            Ok(Self(value))
        } else {
            Err("must be a decimal percentage from 0 through 100".to_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Percentage;

    /// Percentage endpoints and leading zeros retain their numeric values.
    #[test_case::test_case("0", 0.0; "zero")]
    #[test_case::test_case("100", 100.0; "maximum")]
    #[test_case::test_case("60.5", 60.5; "fraction")]
    #[test_case::test_case("000.50", 0.5; "leading zeros")]
    fn accepted_percentages(source: &str, expected: f64) {
        assert!((source.parse::<Percentage>().unwrap().value() - expected).abs() < f64::EPSILON);
    }
}
