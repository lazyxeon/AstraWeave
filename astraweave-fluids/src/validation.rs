//! # Validation Suite
//!
//! Research-grade validation for SPH simulations with standard benchmarks,
//! metrics export, and comparison framework.
//!
//! ## Features
//!
//! - **Standard Tests**: Dam break, hydrostatic, Couette flow, Poiseuille flow
//! - **Conservation Metrics**: Energy, momentum, mass tracking
//! - **Error Analysis**: Density, divergence, pressure error statistics
//! - **Export Formats**: CSV, JSON, VTK (ParaView)
//!
//! ## References
//!
//! - Martin & Moyce 1952: Dam break experimental data
//! - Koschier et al. 2019: SPH validation best practices

use std::path::Path;

// =============================================================================
// VALIDATION METRICS
// =============================================================================

/// Comprehensive validation metrics for SPH simulation
#[derive(Clone, Copy, Debug, Default)]
pub struct ValidationMetrics {
    /// Maximum density error (ρ - ρ₀) / ρ₀
    pub density_error_max: f32,
    /// Average density error
    pub density_error_avg: f32,
    /// Maximum divergence error ∇·v.
    /// `ValidationMetrics::compute` sets this to NaN — divergence is not
    /// computed by the CPU metrics path (no neighbor lists). Check
    /// `is_nan()` before trusting. (F.1: was silently 0.0, i.e. "perfect".)
    pub divergence_error_max: f32,
    /// Average divergence error (NaN from `compute`; see `divergence_error_max`).
    pub divergence_error_avg: f32,
    /// Energy conservation ratio E/E₀
    pub energy_conservation: f32,
    /// Momentum conservation (normalized)
    pub momentum_conservation: [f32; 3],
    /// Mass conservation ratio M/M₀
    pub mass_conservation: f32,
    /// Maximum pressure error
    pub pressure_error_max: f32,
    /// Average pressure error
    pub pressure_error_avg: f32,
    /// Particle count (for tracking)
    pub particle_count: u32,
    /// Simulation time
    pub time: f32,
}

impl ValidationMetrics {
    /// Compute metrics from simulation state
    #[allow(clippy::too_many_arguments)]
    pub fn compute(
        densities: &[f32],
        velocities: &[[f32; 3]],
        pressures: &[f32],
        masses: &[f32],
        rest_density: f32,
        initial_energy: f32,
        initial_momentum: [f32; 3],
        initial_mass: f32,
        time: f32,
    ) -> Self {
        let n = densities.len();
        if n == 0 {
            return Self::default();
        }

        // Density error
        let mut density_error_max = 0.0f32;
        let mut density_error_sum = 0.0f32;

        for &rho in densities {
            let error = ((rho - rest_density) / rest_density).abs();
            density_error_max = density_error_max.max(error);
            density_error_sum += error;
        }
        let density_error_avg = density_error_sum / n as f32;

        // Divergence is NOT computed by this function (it would require
        // neighbor lists, which this CPU metrics path does not have). NaN —
        // not 0.0 — is used so a "perfect divergence" can never be reported
        // by omission; consumers must check `is_nan()` before trusting these
        // two fields. (Pre-F.1 this silently reported 0.0, i.e. perfect.)
        let divergence_error_max = f32::NAN;
        let divergence_error_avg = f32::NAN;

        // Energy conservation
        let mut kinetic_energy = 0.0f32;
        for (i, vel) in velocities.iter().enumerate() {
            let speed_sq = vel[0] * vel[0] + vel[1] * vel[1] + vel[2] * vel[2];
            kinetic_energy += 0.5 * masses[i] * speed_sq;
        }
        let energy_conservation = if initial_energy > 0.0 {
            kinetic_energy / initial_energy
        } else {
            1.0
        };

        // Momentum conservation
        let mut momentum = [0.0f32; 3];
        for (i, vel) in velocities.iter().enumerate() {
            momentum[0] += masses[i] * vel[0];
            momentum[1] += masses[i] * vel[1];
            momentum[2] += masses[i] * vel[2];
        }
        let initial_mom_mag = (initial_momentum[0].powi(2)
            + initial_momentum[1].powi(2)
            + initial_momentum[2].powi(2))
        .sqrt()
        .max(1e-6);
        let momentum_conservation = [
            momentum[0] / initial_mom_mag.max(momentum[0].abs().max(1e-6)),
            momentum[1] / initial_mom_mag.max(momentum[1].abs().max(1e-6)),
            momentum[2] / initial_mom_mag.max(momentum[2].abs().max(1e-6)),
        ];

        // Mass conservation
        let total_mass: f32 = masses.iter().sum();
        let mass_conservation = if initial_mass > 0.0 {
            total_mass / initial_mass
        } else {
            1.0
        };

        // Pressure error
        let mut pressure_error_max = 0.0f32;
        let mut pressure_error_sum = 0.0f32;
        for &p in pressures {
            let error = p.abs(); // Error from zero (ideal incompressible)
            pressure_error_max = pressure_error_max.max(error);
            pressure_error_sum += error;
        }
        let pressure_error_avg = pressure_error_sum / n as f32;

        Self {
            density_error_max,
            density_error_avg,
            divergence_error_max,
            divergence_error_avg,
            energy_conservation,
            momentum_conservation,
            mass_conservation,
            pressure_error_max,
            pressure_error_avg,
            particle_count: n as u32,
            time,
        }
    }

    /// Check if simulation meets research-grade thresholds
    pub fn is_research_grade(&self) -> bool {
        self.density_error_max < 0.01 && // <1% density error
        self.mass_conservation > 0.999 && // >99.9% mass conservation
        self.energy_conservation > 0.95 // >95% energy conservation (allow dissipation)
    }

    /// Get a summary grade
    pub fn grade(&self) -> ValidationGrade {
        if self.density_error_max < 0.001 && self.mass_conservation > 0.9999 {
            ValidationGrade::Excellent
        } else if self.density_error_max < 0.01 && self.mass_conservation > 0.999 {
            ValidationGrade::Good
        } else if self.density_error_max < 0.05 && self.mass_conservation > 0.99 {
            ValidationGrade::Acceptable
        } else {
            ValidationGrade::Poor
        }
    }
}

/// Validation quality grade
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationGrade {
    /// Excellent: <0.1% density error, >99.99% mass conservation
    Excellent,
    /// Good: <1% density error, >99.9% mass conservation
    Good,
    /// Acceptable: <5% density error, >99% mass conservation
    Acceptable,
    /// Poor: Fails basic thresholds
    Poor,
}

impl std::fmt::Display for ValidationGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Excellent => write!(f, "Excellent (A+)"),
            Self::Good => write!(f, "Good (A)"),
            Self::Acceptable => write!(f, "Acceptable (B)"),
            Self::Poor => write!(f, "Poor (C)"),
        }
    }
}

// =============================================================================
// STANDARD BENCHMARKS
// =============================================================================

/// Standard benchmark test configuration
#[derive(Clone, Debug)]
pub struct BenchmarkConfig {
    /// Benchmark name
    pub name: String,
    /// Domain size [x, y, z]
    pub domain: [f32; 3],
    /// Particle spacing
    pub particle_spacing: f32,
    /// Smoothing radius
    pub smoothing_radius: f32,
    /// Rest density
    pub rest_density: f32,
    /// Simulation duration
    pub duration: f32,
    /// Time step
    pub dt: f32,
    /// Gravity
    pub gravity: [f32; 3],
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            name: "Custom".to_string(),
            domain: [1.0, 1.0, 1.0],
            particle_spacing: 0.02,
            smoothing_radius: 0.04,
            rest_density: 1000.0,
            duration: 1.0,
            dt: 0.001,
            gravity: [0.0, -9.81, 0.0],
        }
    }
}

impl BenchmarkConfig {
    /// Dam break benchmark (Martin & Moyce 1952)
    pub fn dam_break() -> Self {
        Self {
            name: "Dam Break (Martin & Moyce 1952)".to_string(),
            domain: [1.0, 0.6, 0.2],
            particle_spacing: 0.01,
            smoothing_radius: 0.02,
            rest_density: 1000.0,
            duration: 2.0,
            dt: 0.0001,
            gravity: [0.0, -9.81, 0.0],
        }
    }

    /// Hydrostatic pressure test
    pub fn hydrostatic() -> Self {
        Self {
            name: "Hydrostatic Pressure".to_string(),
            domain: [0.2, 0.5, 0.2],
            particle_spacing: 0.01,
            smoothing_radius: 0.02,
            rest_density: 1000.0,
            duration: 1.0,
            dt: 0.0001,
            gravity: [0.0, -9.81, 0.0],
        }
    }

    /// Couette flow (viscosity validation)
    pub fn couette_flow() -> Self {
        Self {
            name: "Couette Flow (Linear Velocity)".to_string(),
            domain: [0.1, 0.5, 0.1],
            particle_spacing: 0.01,
            smoothing_radius: 0.02,
            rest_density: 1000.0,
            duration: 1.0,
            dt: 0.0001,
            gravity: [0.0, 0.0, 0.0], // No gravity for Couette
        }
    }

    /// Poiseuille flow (pipe flow)
    pub fn poiseuille_flow() -> Self {
        Self {
            name: "Poiseuille Flow (Parabolic Profile)".to_string(),
            domain: [0.5, 0.1, 0.1],
            particle_spacing: 0.005,
            smoothing_radius: 0.01,
            rest_density: 1000.0,
            duration: 1.0,
            dt: 0.0001,
            gravity: [0.1, 0.0, 0.0], // Pressure gradient as body force
        }
    }

    /// Rayleigh-Taylor instability
    pub fn rayleigh_taylor() -> Self {
        Self {
            name: "Rayleigh-Taylor Instability".to_string(),
            domain: [0.5, 1.0, 0.5],
            particle_spacing: 0.01,
            smoothing_radius: 0.02,
            rest_density: 1000.0,
            duration: 2.0,
            dt: 0.0001,
            gravity: [0.0, -9.81, 0.0],
        }
    }

    /// Drop splash (surface tension)
    pub fn drop_splash() -> Self {
        Self {
            name: "Drop Splash (Surface Tension)".to_string(),
            domain: [0.3, 0.4, 0.3],
            particle_spacing: 0.005,
            smoothing_radius: 0.01,
            rest_density: 1000.0,
            duration: 0.5,
            dt: 0.00005,
            gravity: [0.0, -9.81, 0.0],
        }
    }
}

// =============================================================================
// COMPARISON FRAMEWORK
// =============================================================================

/// Reference data point for comparison
#[derive(Clone, Copy, Debug, Default)]
pub struct ReferencePoint {
    /// Time
    pub t: f32,
    /// Position
    pub position: [f32; 3],
    /// Velocity (optional)
    pub velocity: Option<[f32; 3]>,
    /// Density (optional)
    pub density: Option<f32>,
}

/// Reference data for comparison
#[derive(Clone, Debug, Default)]
pub struct ReferenceData {
    /// Data source name
    pub source: String,
    /// Reference points
    pub points: Vec<ReferencePoint>,
}

impl ReferenceData {
    /// Create new reference data
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            points: Vec::new(),
        }
    }

    /// Add a reference point
    pub fn add_point(&mut self, t: f32, position: [f32; 3]) {
        self.points.push(ReferencePoint {
            t,
            position,
            velocity: None,
            density: None,
        });
    }

    /// Load reference data from a CSV file.
    ///
    /// Expected columns per row: `t,x,y,z[,vx,vy,vz[,density]]`.
    /// Lines starting with `#` and a non-numeric header line are skipped.
    ///
    /// (Pre-F.1 this was a placeholder that ignored the path and returned
    /// `Ok(Self::default())` — faking a successful load.)
    pub fn load_csv(path: &Path) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        let source = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "csv".to_string());
        let mut data = Self::new(&source);

        for (line_no, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split(',').map(str::trim).collect();
            if fields.len() < 4 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "line {}: expected at least 4 columns (t,x,y,z)",
                        line_no + 1
                    ),
                ));
            }
            let parse = |s: &str| s.parse::<f32>();
            let t = match parse(fields[0]) {
                Ok(v) => v,
                // Tolerate a single non-numeric header row.
                Err(_) if line_no == 0 => continue,
                Err(e) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("line {}: bad t value: {e}", line_no + 1),
                    ))
                }
            };
            let num = |i: usize| -> Result<f32, std::io::Error> {
                parse(fields[i]).map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("line {}: bad value in column {}: {e}", line_no + 1, i + 1),
                    )
                })
            };
            let position = [num(1)?, num(2)?, num(3)?];
            let velocity = if fields.len() >= 7 {
                Some([num(4)?, num(5)?, num(6)?])
            } else {
                None
            };
            let density = if fields.len() >= 8 {
                Some(num(7)?)
            } else {
                None
            };
            data.points.push(ReferencePoint {
                t,
                position,
                velocity,
                density,
            });
        }
        Ok(data)
    }
}

/// Comparison result
#[derive(Clone, Copy, Debug, Default)]
pub struct ComparisonResult {
    /// Root mean square error (position)
    pub rmse_position: f32,
    /// Peak position error
    pub peak_error_position: f32,
    /// RMSE velocity (if available)
    pub rmse_velocity: Option<f32>,
    /// Peak velocity error
    pub peak_error_velocity: Option<f32>,
    /// Number of comparison points
    pub num_points: u32,
}

impl ComparisonResult {
    /// Compute comparison between simulation and reference
    pub fn compute(sim_positions: &[[f32; 3]], ref_positions: &[[f32; 3]]) -> Self {
        let n = sim_positions.len().min(ref_positions.len());
        if n == 0 {
            return Self::default();
        }

        let mut sum_sq = 0.0f32;
        let mut peak = 0.0f32;

        for i in 0..n {
            let dx = sim_positions[i][0] - ref_positions[i][0];
            let dy = sim_positions[i][1] - ref_positions[i][1];
            let dz = sim_positions[i][2] - ref_positions[i][2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();

            sum_sq += dist * dist;
            peak = peak.max(dist);
        }

        Self {
            rmse_position: (sum_sq / n as f32).sqrt(),
            peak_error_position: peak,
            rmse_velocity: None,
            peak_error_velocity: None,
            num_points: n as u32,
        }
    }
}

// =============================================================================
// EXPORT FORMATS
// =============================================================================

/// Export format options
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExportFormat {
    /// Comma-separated values
    Csv,
    /// JavaScript Object Notation
    Json,
    /// VTK for ParaView
    Vtk,
    /// PLY mesh
    Ply,
}

/// Metrics history for time-series export
#[derive(Clone, Debug, Default)]
pub struct MetricsHistory {
    /// Recorded metrics over time
    pub records: Vec<ValidationMetrics>,
}

impl MetricsHistory {
    /// Create new empty history
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    /// Add a metrics record
    pub fn record(&mut self, metrics: ValidationMetrics) {
        self.records.push(metrics);
    }

    /// Clear history
    pub fn clear(&mut self) {
        self.records.clear();
    }

    /// Get number of records
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Export to CSV string
    pub fn to_csv(&self) -> String {
        let mut csv = String::from("time,particle_count,density_error_max,density_error_avg,energy_conservation,mass_conservation,pressure_error_max\n");

        for m in &self.records {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                m.time,
                m.particle_count,
                m.density_error_max,
                m.density_error_avg,
                m.energy_conservation,
                m.mass_conservation,
                m.pressure_error_max,
            ));
        }

        csv
    }

    /// Export to JSON string
    pub fn to_json(&self) -> String {
        let mut json = String::from("[\n");

        for (i, m) in self.records.iter().enumerate() {
            if i > 0 {
                json.push_str(",\n");
            }
            json.push_str(&format!(
                r#"  {{"time": {}, "particle_count": {}, "density_error_max": {}, "density_error_avg": {}, "energy_conservation": {}, "mass_conservation": {}}}"#,
                m.time,
                m.particle_count,
                m.density_error_max,
                m.density_error_avg,
                m.energy_conservation,
                m.mass_conservation,
            ));
        }

        json.push_str("\n]");
        json
    }

    /// Get summary statistics
    pub fn summary(&self) -> MetricsSummary {
        if self.records.is_empty() {
            return MetricsSummary::default();
        }

        let mut max_density_error = 0.0f32;
        let mut avg_density_error = 0.0f32;
        let mut min_energy = 1.0f32;
        let mut min_mass = 1.0f32;

        for m in &self.records {
            max_density_error = max_density_error.max(m.density_error_max);
            avg_density_error += m.density_error_avg;
            min_energy = min_energy.min(m.energy_conservation);
            min_mass = min_mass.min(m.mass_conservation);
        }

        MetricsSummary {
            max_density_error,
            avg_density_error: avg_density_error / self.records.len() as f32,
            min_energy_conservation: min_energy,
            min_mass_conservation: min_mass,
            final_grade: self
                .records
                .last()
                .map(|m| m.grade())
                .unwrap_or(ValidationGrade::Poor),
        }
    }
}

/// Summary statistics from metrics history
#[derive(Clone, Copy, Debug, Default)]
pub struct MetricsSummary {
    /// Maximum density error across all frames
    pub max_density_error: f32,
    /// Average density error
    pub avg_density_error: f32,
    /// Minimum energy conservation
    pub min_energy_conservation: f32,
    /// Minimum mass conservation
    pub min_mass_conservation: f32,
    /// Final grade
    pub final_grade: ValidationGrade,
}

impl Default for ValidationGrade {
    fn default() -> Self {
        Self::Poor
    }
}

// =============================================================================
// VTK EXPORT
// =============================================================================

/// VTK file writer for ParaView
pub struct VtkExporter {
    /// Output directory
    pub output_dir: String,
    /// Frame counter
    pub frame: u32,
}

impl VtkExporter {
    /// Create new VTK exporter
    pub fn new(output_dir: &str) -> Self {
        Self {
            output_dir: output_dir.to_string(),
            frame: 0,
        }
    }

    /// Export particle positions to VTK
    pub fn export_particles(
        &mut self,
        positions: &[[f32; 3]],
        velocities: &[[f32; 3]],
        densities: &[f32],
    ) -> String {
        let n = positions.len();

        let mut vtk = String::new();
        vtk.push_str("# vtk DataFile Version 3.0\n");
        vtk.push_str("SPH Particles\n");
        vtk.push_str("ASCII\n");
        vtk.push_str("DATASET POLYDATA\n");

        // Points
        vtk.push_str(&format!("POINTS {} float\n", n));
        for pos in positions {
            vtk.push_str(&format!("{} {} {}\n", pos[0], pos[1], pos[2]));
        }

        // Vertices
        vtk.push_str(&format!("VERTICES {} {}\n", n, n * 2));
        for i in 0..n {
            vtk.push_str(&format!("1 {}\n", i));
        }

        // Point data
        vtk.push_str(&format!("POINT_DATA {}\n", n));

        // Velocity vectors
        vtk.push_str("VECTORS velocity float\n");
        for vel in velocities {
            vtk.push_str(&format!("{} {} {}\n", vel[0], vel[1], vel[2]));
        }

        // Density scalars
        vtk.push_str("SCALARS density float 1\n");
        vtk.push_str("LOOKUP_TABLE default\n");
        for &rho in densities {
            vtk.push_str(&format!("{}\n", rho));
        }

        self.frame += 1;
        vtk
    }

    /// Get filename for current frame
    pub fn filename(&self) -> String {
        format!("{}/particles_{:05}.vtk", self.output_dir, self.frame)
    }
}

// =============================================================================
// PARAMETER STUDY
// =============================================================================

/// Configuration for parameter sweep study
#[derive(Clone, Debug)]
pub struct ParameterStudy {
    /// Parameter name being varied
    pub parameter_name: String,
    /// Values to test
    pub values: Vec<f32>,
    /// Baseline configuration
    pub baseline: BenchmarkConfig,
}

impl ParameterStudy {
    /// Create new parameter study
    pub fn new(parameter_name: &str, values: Vec<f32>, baseline: BenchmarkConfig) -> Self {
        Self {
            parameter_name: parameter_name.to_string(),
            values,
            baseline,
        }
    }

    /// Get number of test cases
    pub fn num_cases(&self) -> usize {
        self.values.len()
    }

    /// Get configuration for a specific value
    pub fn config_for_value(&self, value: f32) -> BenchmarkConfig {
        let mut config = self.baseline.clone();

        // Apply parameter value based on name
        match self.parameter_name.as_str() {
            "particle_spacing" => config.particle_spacing = value,
            "smoothing_radius" => config.smoothing_radius = value,
            "dt" => config.dt = value,
            "rest_density" => config.rest_density = value,
            _ => {} // Unknown parameter
        }

        config
    }
}

/// Result of a single study case
#[derive(Clone, Debug)]
pub struct StudyResult {
    /// Parameter value
    pub parameter_value: f32,
    /// Final metrics
    pub metrics: ValidationMetrics,
    /// Metrics history
    pub history: MetricsHistory,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_metrics_default() {
        let metrics = ValidationMetrics::default();
        assert_eq!(metrics.density_error_max, 0.0);
        assert_eq!(metrics.particle_count, 0);
    }

    #[test]
    fn test_validation_metrics_compute() {
        let densities = vec![1000.0, 1005.0, 995.0];
        let velocities = vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let pressures = vec![100.0, 200.0, 50.0];
        let masses = vec![1.0, 1.0, 1.0];

        let metrics = ValidationMetrics::compute(
            &densities,
            &velocities,
            &pressures,
            &masses,
            1000.0,
            1.5,             // initial energy
            [1.0, 1.0, 1.0], // initial momentum
            3.0,             // initial mass
            0.5,
        );

        assert_eq!(metrics.particle_count, 3);
        assert!(metrics.density_error_max > 0.0);
        assert!((metrics.mass_conservation - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_validation_grade_excellent() {
        let metrics = ValidationMetrics {
            density_error_max: 0.0005,
            mass_conservation: 0.99999,
            ..Default::default()
        };

        assert_eq!(metrics.grade(), ValidationGrade::Excellent);
    }

    #[test]
    fn test_validation_grade_good() {
        let metrics = ValidationMetrics {
            density_error_max: 0.005,
            mass_conservation: 0.9995,
            ..Default::default()
        };

        assert_eq!(metrics.grade(), ValidationGrade::Good);
    }

    #[test]
    fn test_validation_grade_acceptable() {
        let metrics = ValidationMetrics {
            density_error_max: 0.03,
            mass_conservation: 0.995,
            ..Default::default()
        };

        assert_eq!(metrics.grade(), ValidationGrade::Acceptable);
    }

    #[test]
    fn test_validation_grade_poor() {
        let metrics = ValidationMetrics {
            density_error_max: 0.1,
            mass_conservation: 0.9,
            ..Default::default()
        };

        assert_eq!(metrics.grade(), ValidationGrade::Poor);
    }

    #[test]
    fn test_is_research_grade() {
        let good = ValidationMetrics {
            density_error_max: 0.005,
            mass_conservation: 0.9999,
            energy_conservation: 0.98,
            ..Default::default()
        };
        assert!(good.is_research_grade());

        let bad = ValidationMetrics {
            density_error_max: 0.05,
            mass_conservation: 0.99,
            energy_conservation: 0.8,
            ..Default::default()
        };
        assert!(!bad.is_research_grade());
    }

    #[test]
    fn test_reference_data_load_csv_roundtrip() {
        let path = std::env::temp_dir().join("aw_fluids_f1_load_csv_test.csv");
        std::fs::write(
            &path,
            "t,x,y,z,vx,vy,vz,density\n# comment\n0.0,1.0,2.0,3.0,0.1,0.2,0.3,1000.0\n0.5,1.5,2.5,3.5\n",
        )
        .expect("write temp csv");

        let data = ReferenceData::load_csv(&path).expect("load_csv should parse valid file");
        std::fs::remove_file(&path).ok();

        assert_eq!(data.points.len(), 2);
        assert_eq!(data.points[0].t, 0.0);
        assert_eq!(data.points[0].position, [1.0, 2.0, 3.0]);
        assert_eq!(data.points[0].velocity, Some([0.1, 0.2, 0.3]));
        assert_eq!(data.points[0].density, Some(1000.0));
        assert_eq!(data.points[1].t, 0.5);
        assert_eq!(data.points[1].velocity, None);
    }

    #[test]
    fn test_reference_data_load_csv_missing_file_errors() {
        // Pre-F.1 the placeholder returned Ok(default) for ANY path.
        let missing = std::path::Path::new("definitely/not/a/real/file.csv");
        assert!(ReferenceData::load_csv(missing).is_err());
    }

    #[test]
    fn test_compute_divergence_is_nan_not_fake_zero() {
        // F.1: compute() must not silently report perfect (0.0) divergence.
        let m = ValidationMetrics::compute(
            &[1000.0],          // densities
            &[[0.0, 0.0, 0.0]], // velocities
            &[0.0],             // pressures
            &[1.0],             // masses
            1000.0,             // rest_density
            1.0,                // initial_energy
            [0.0, 0.0, 0.0],    // initial_momentum
            1.0,                // initial_mass
            0.0,                // time
        );
        assert!(m.divergence_error_max.is_nan());
        assert!(m.divergence_error_avg.is_nan());
    }

    #[test]
    fn test_benchmark_config_dam_break() {
        let config = BenchmarkConfig::dam_break();
        assert!(config.name.contains("Dam Break"));
        assert!(config.gravity[1] < 0.0);
    }

    #[test]
    fn test_benchmark_config_hydrostatic() {
        let config = BenchmarkConfig::hydrostatic();
        assert!(config.name.contains("Hydrostatic"));
    }

    #[test]
    fn test_benchmark_config_couette() {
        let config = BenchmarkConfig::couette_flow();
        assert!(config.name.contains("Couette"));
        // Couette has no gravity
        assert_eq!(config.gravity, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_benchmark_config_poiseuille() {
        let config = BenchmarkConfig::poiseuille_flow();
        assert!(config.name.contains("Poiseuille"));
        // Poiseuille has pressure gradient as body force
        assert!(config.gravity[0] > 0.0);
    }

    #[test]
    fn test_reference_data_creation() {
        let mut data = ReferenceData::new("Test Source");
        data.add_point(0.0, [0.0, 0.0, 0.0]);
        data.add_point(0.1, [0.1, 0.0, 0.0]);

        assert_eq!(data.source, "Test Source");
        assert_eq!(data.points.len(), 2);
    }

    #[test]
    fn test_comparison_result_compute() {
        let sim_pos = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
        let ref_pos = vec![[0.0, 0.0, 0.0], [1.1, 0.0, 0.0]];

        let result = ComparisonResult::compute(&sim_pos, &ref_pos);

        assert_eq!(result.num_points, 2);
        assert!(result.rmse_position > 0.0);
        assert!((result.peak_error_position - 0.1).abs() < 1e-6);
    }

    #[test]
    fn test_metrics_history_record() {
        let mut history = MetricsHistory::new();

        history.record(ValidationMetrics {
            time: 0.0,
            ..Default::default()
        });
        history.record(ValidationMetrics {
            time: 0.1,
            ..Default::default()
        });

        assert_eq!(history.len(), 2);
        assert!(!history.is_empty());
    }

    #[test]
    fn test_metrics_history_to_csv() {
        let mut history = MetricsHistory::new();
        history.record(ValidationMetrics {
            time: 0.0,
            particle_count: 100,
            density_error_max: 0.01,
            ..Default::default()
        });

        let csv = history.to_csv();
        assert!(csv.contains("time,particle_count"));
        assert!(csv.contains("0,100,0.01"));
    }

    #[test]
    fn test_metrics_history_to_json() {
        let mut history = MetricsHistory::new();
        history.record(ValidationMetrics {
            time: 0.5,
            particle_count: 200,
            ..Default::default()
        });

        let json = history.to_json();
        assert!(json.contains("\"time\": 0.5"));
        assert!(json.contains("\"particle_count\": 200"));
    }

    #[test]
    fn test_metrics_history_summary() {
        let mut history = MetricsHistory::new();
        history.record(ValidationMetrics {
            density_error_max: 0.01,
            energy_conservation: 0.99,
            mass_conservation: 0.999,
            ..Default::default()
        });
        history.record(ValidationMetrics {
            density_error_max: 0.02,
            energy_conservation: 0.95,
            mass_conservation: 0.998,
            ..Default::default()
        });

        let summary = history.summary();

        assert!((summary.max_density_error - 0.02).abs() < 1e-6);
        assert!((summary.min_energy_conservation - 0.95).abs() < 1e-6);
        assert!((summary.min_mass_conservation - 0.998).abs() < 1e-6);
    }

    #[test]
    fn test_metrics_history_clear() {
        let mut history = MetricsHistory::new();
        history.record(ValidationMetrics::default());
        history.clear();

        assert!(history.is_empty());
    }

    #[test]
    fn test_vtk_exporter_creation() {
        let exporter = VtkExporter::new("output");
        assert_eq!(exporter.output_dir, "output");
        assert_eq!(exporter.frame, 0);
    }

    #[test]
    fn test_vtk_exporter_export() {
        let mut exporter = VtkExporter::new("output");
        let positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
        let velocities = vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let densities = vec![1000.0, 1005.0];

        let vtk = exporter.export_particles(&positions, &velocities, &densities);

        assert!(vtk.contains("vtk DataFile"));
        assert!(vtk.contains("POINTS 2"));
        assert!(vtk.contains("VECTORS velocity"));
        assert!(vtk.contains("SCALARS density"));
        assert_eq!(exporter.frame, 1);
    }

    #[test]
    fn test_vtk_exporter_filename() {
        let mut exporter = VtkExporter::new("results");
        assert_eq!(exporter.filename(), "results/particles_00000.vtk");

        exporter.frame = 42;
        assert_eq!(exporter.filename(), "results/particles_00042.vtk");
    }

    #[test]
    fn test_parameter_study_creation() {
        let study = ParameterStudy::new(
            "particle_spacing",
            vec![0.01, 0.02, 0.04],
            BenchmarkConfig::default(),
        );

        assert_eq!(study.parameter_name, "particle_spacing");
        assert_eq!(study.num_cases(), 3);
    }

    #[test]
    fn test_parameter_study_config_for_value() {
        let study = ParameterStudy::new("dt", vec![0.001, 0.0001], BenchmarkConfig::default());

        let config = study.config_for_value(0.0001);
        assert!((config.dt - 0.0001).abs() < 1e-8);
    }

    #[test]
    fn test_validation_grade_display() {
        assert_eq!(format!("{}", ValidationGrade::Excellent), "Excellent (A+)");
        assert_eq!(format!("{}", ValidationGrade::Good), "Good (A)");
        assert_eq!(format!("{}", ValidationGrade::Acceptable), "Acceptable (B)");
        assert_eq!(format!("{}", ValidationGrade::Poor), "Poor (C)");
    }

    #[test]
    fn test_export_format_values() {
        assert_ne!(ExportFormat::Csv, ExportFormat::Json);
        assert_ne!(ExportFormat::Vtk, ExportFormat::Ply);
    }
}
