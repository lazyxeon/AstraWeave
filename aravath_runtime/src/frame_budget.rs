//! CPU frame times against the GTX 1660 Ti-class budget.
//!
//! The number is the whole frame the demo is allowed on that class of GPU.
//! `--frame-times` measures the runtime's own sim-and-skin work and prints
//! this budget beside it. A software adapter is not that GPU.

/// Whole-frame budget for a GTX 1660 Ti-class baseline.
pub const FRAME_BUDGET_MS: f64 = 11.0;

pub const BASELINE: &str = "GTX 1660 Ti-class";

#[derive(Clone, Debug, PartialEq)]
pub struct FrameReport {
    pub count: usize,
    pub min_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub max_ms: f64,
}

impl FrameReport {
    pub fn from_nanos(samples: &[u128]) -> Self {
        if samples.is_empty() {
            return Self {
                count: 0,
                min_ms: 0.0,
                p50_ms: 0.0,
                p95_ms: 0.0,
                max_ms: 0.0,
            };
        }
        let mut ordered: Vec<u128> = samples.to_vec();
        ordered.sort_unstable();
        let at = |fraction: f64| {
            let index = ((ordered.len() - 1) as f64 * fraction).round() as usize;
            nanos_to_ms(ordered[index])
        };
        Self {
            count: ordered.len(),
            min_ms: nanos_to_ms(ordered[0]),
            p50_ms: at(0.50),
            p95_ms: at(0.95),
            max_ms: nanos_to_ms(*ordered.last().unwrap_or(&0)),
        }
    }

    pub fn inside_budget(&self) -> bool {
        self.count > 0 && self.p95_ms < FRAME_BUDGET_MS
    }
}

pub fn nanos_to_ms(nanos: u128) -> f64 {
    nanos as f64 / 1_000_000.0
}

pub fn budget_line(report: &FrameReport) -> String {
    let word = if report.inside_budget() {
        "inside budget"
    } else {
        "over budget"
    };
    format!(
        "cpu frame n={} min {:.3} ms p50 {:.3} ms p95 {:.3} ms max {:.3} ms against {:.3} ms {BASELINE}: {word}",
        report.count, report.min_ms, report.p50_ms, report.p95_ms, report.max_ms, FRAME_BUDGET_MS
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_milliseconds_is_inside_the_1660_ti_budget() {
        let samples = vec![10_000_000_u128; 20];
        let report = FrameReport::from_nanos(&samples);
        assert!(report.inside_budget());
        assert!((report.p95_ms - 10.0).abs() < 1e-6);
        assert!(budget_line(&report).contains("inside budget"));
        assert!(budget_line(&report).contains("11.000"));
    }

    #[test]
    fn twelve_milliseconds_misses_the_budget() {
        let report = FrameReport::from_nanos(&[12_000_000]);
        assert!(!report.inside_budget());
        assert!(budget_line(&report).contains("over budget"));
    }
}
