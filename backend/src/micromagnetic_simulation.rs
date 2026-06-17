use crate::errors::{AppError, Result};
use crate::models::{
    DeviceGeometryParams, DeviceType, InteractiveSinanRequest, InteractiveSinanResponse,
    InterferenceEffect, InterferenceSimulationRequest, InterferenceSimulationResponse,
    InterferenceSource, InterferenceType, MultiDeviceCompareRequest, MultiDeviceCompareResponse,
    PointingSimulationParams, PointingSimulationResult, SingleDeviceAccuracy,
    CrossEraCompareRequest, CrossEraCompareResponse,
};
use nalgebra::{Vector3, Matrix3};
use rand_distr::{Normal, Distribution};
use std::collections::HashMap;
use std::f64::consts::PI;

#[derive(Debug, Clone)]
pub struct DemagnetizationTensor {
    pub n_xx: f64,
    pub n_yy: f64,
    pub n_zz: f64,
    pub n_xy: f64,
    pub n_xz: f64,
    pub n_yz: f64,
    pub corner_correction: f64,
    pub domain_wall_correction: f64,
}

impl DemagnetizationTensor {
    pub fn for_ellipsoid(a: f64, b: f64, c: f64) -> Self {
        let n = Self::ellipsoid_demagnetization_factors(a, b, c);
        Self {
            n_xx: n.0,
            n_yy: n.1,
            n_zz: n.2,
            n_xy: 0.0,
            n_xz: 0.0,
            n_yz: 0.0,
            corner_correction: 0.0,
            domain_wall_correction: 0.0,
        }
    }

    pub fn for_spoon_shape(length: f64, width: f64, thickness: f64) -> Self {
        let (n_body_x, n_body_y, n_body_z) = Self::ellipsoid_demagnetization_factors(
            length * 0.7, width * 0.5, thickness
        );

        let handle_ratio = 0.3;
        let (n_handle_x, n_handle_y, n_handle_z) = Self::ellipsoid_demagnetization_factors(
            length * handle_ratio, width * 0.3, thickness * 1.2
        );

        let w_body = 0.75;
        let w_handle = 0.25;

        let bowl_correction = Self::calculate_bowl_edge_correction(width, thickness);
        let handle_correction = Self::calculate_handle_tip_correction(length, thickness);
        let corner_correction = (bowl_correction + handle_correction) * 0.5;

        let domain_wall_correction = Self::calculate_domain_wall_demagnetization(
            length, width, thickness
        );

        Self {
            n_xx: w_body * n_body_x + w_handle * n_handle_x,
            n_yy: w_body * n_body_y + w_handle * n_handle_y,
            n_zz: w_body * n_body_z + w_handle * n_handle_z,
            n_xy: -0.02 * corner_correction,
            n_xz: 0.015 * corner_correction,
            n_yz: -0.01 * corner_correction,
            corner_correction,
            domain_wall_correction,
        }
    }

    fn ellipsoid_demagnetization_factors(a: f64, b: f64, c: f64) -> (f64, f64, f64) {
        let axes = [a, b, c];
        let mut sorted_indices: Vec<usize> = (0..3).collect();
        sorted_indices.sort_by(|&i, &j| axes[j].partial_cmp(&axes[i]).unwrap());

        let a_max = axes[sorted_indices[0]];
        let a_mid = axes[sorted_indices[1]];
        let a_min = axes[sorted_indices[2]];

        let n_max: f64;
        let n_mid: f64;
        let n_min: f64;

        if (a_max - a_mid).abs() < 1e-6 && (a_mid - a_min).abs() < 1e-6 {
            n_max = 1.0 / 3.0;
            n_mid = 1.0 / 3.0;
            n_min = 1.0 / 3.0;
        } else if (a_max - a_mid).abs() < 1e-6 {
            let e = (1.0 - (a_min / a_max).powi(2)).sqrt();
            let ln_term = ((1.0 + e) / (1.0 - e)).ln();
            n_min = (1.0 / e.powi(2)) * (1.0 - (1.0 - e.powi(2)).sqrt() / e * ln_term / 2.0);
            n_max = (1.0 - n_min) / 2.0;
            n_mid = n_max;
        } else if (a_mid - a_min).abs() < 1e-6 {
            let e = (1.0 - (a_min / a_max).powi(2)).sqrt();
            let asin_e = e.asin();
            n_max = (1.0 - e.powi(2)) / e.powi(2) * (asin_e / e - 1.0);
            n_min = (1.0 - n_max) / 2.0;
            n_mid = n_min;
        } else {
            let k_sq = (a_max.powi(2) - a_mid.powi(2)) / (a_max.powi(2) - a_min.powi(2));
            let alpha_sq = a_min.powi(2) / a_max.powi(2);
            let beta_sq = a_mid.powi(2) / a_max.powi(2);
            let k = k_sq.sqrt();

            let (k_complete, e_complete) = Self::elliptic_integrals(k);

            let a_factor = 2.0 / ((1.0 - alpha_sq).sqrt() * (1.0 - beta_sq));
            let b_factor = (beta_sq / (1.0 - beta_sq)).sqrt();
            let c_factor = (alpha_sq / (1.0 - alpha_sq)).sqrt();

            n_max = a_factor * (k_complete - e_complete);
            n_mid = a_factor * (e_complete / beta_sq - k_complete * (1.0 - beta_sq) / beta_sq
                - b_factor * (k_complete - e_complete) * (1.0 - k_sq) / k_sq);
            n_min = 1.0 - n_max - n_mid;
        }

        let mut result = [0.0f64; 3];
        result[sorted_indices[0]] = n_max;
        result[sorted_indices[1]] = n_mid;
        result[sorted_indices[2]] = n_min;

        (result[0], result[1], result[2])
    }

    fn elliptic_integrals(k: f64) -> (f64, f64) {
        let a0 = 1.0;
        let b0 = (1.0 - k * k).sqrt();

        let mut a = a0;
        let mut b = b0;
        let mut c = k;
        let mut sum_d = 0.0;
        let mut pow_2n = 1.0;

        for _ in 0..20 {
            let a_new = (a + b) / 2.0;
            let b_new = (a * b).sqrt();
            let c_new = (a - b) / 2.0;

            pow_2n *= 2.0;
            sum_d += pow_2n * c_new * c_new;

            a = a_new;
            b = b_new;
            c = c_new;

            if c.abs() < 1e-15 {
                break;
            }
        }

        let k_complete = PI / (2.0 * a);
        let e_complete = k_complete * (1.0 - sum_d / 2.0);

        (k_complete, e_complete)
    }

    fn calculate_bowl_edge_correction(bowl_width: f64, thickness: f64) -> f64 {
        let aspect_ratio = bowl_width / thickness;
        let edge_singularity = (aspect_ratio / 10.0).ln().max(0.5);
        let curvature_factor = 1.0 + 1.0 / (aspect_ratio * 0.5 + 1.0);

        0.08 * edge_singularity * curvature_factor
    }

    fn calculate_handle_tip_correction(handle_length: f64, thickness: f64) -> f64 {
        let taper_ratio = handle_length / thickness;
        let tip_field_enhancement = (taper_ratio / 8.0).tanh() * 2.0;

        0.06 * tip_field_enhancement
    }

    fn calculate_domain_wall_demagnetization(length: f64, width: f64, thickness: f64) -> f64 {
        let volume = length * width * thickness;
        let domain_width = 1e-6;
        let wall_thickness = 1e-8;
        let num_domains = (volume / (domain_width * width * thickness)).max(1.0);

        let wall_energy_factor = 1e-3 * num_domains.sqrt();
        let shape_factor = (length / (width + thickness)).tanh();

        wall_energy_factor * shape_factor
    }

    pub fn as_matrix(&self) -> Matrix3<f64> {
        Matrix3::new(
            self.n_xx, self.n_xy, self.n_xz,
            self.n_xy, self.n_yy, self.n_yz,
            self.n_xz, self.n_yz, self.n_zz,
        )
    }

    pub fn effective_trace(&self) -> f64 {
        (self.n_xx + self.n_yy + self.n_zz) / 3.0 + self.corner_correction * 0.3
    }

    pub fn for_fish_shape(length_m: f64, width_m: f64, thickness_m: f64) -> Self {
        let (n_body_x, n_body_y, n_body_z) = Self::ellipsoid_demagnetization_factors(
            length_m, width_m * 0.6, thickness_m
        );
        let (n_head_x, n_head_y, n_head_z) = Self::ellipsoid_demagnetization_factors(
            length_m * 0.3, width_m, thickness_m * 1.5
        );
        let (n_tail_x, n_tail_y, n_tail_z) = Self::ellipsoid_demagnetization_factors(
            length_m * 0.25, width_m * 0.3, thickness_m * 0.8
        );
        let w_body = 0.55;
        let w_head = 0.3;
        let w_tail = 0.15;
        Self {
            n_xx: w_body * n_body_x + w_head * n_head_x + w_tail * n_tail_x,
            n_yy: w_body * n_body_y + w_head * n_head_y + w_tail * n_tail_y,
            n_zz: w_body * n_body_z + w_head * n_head_z + w_tail * n_tail_z,
            n_xy: -0.01,
            n_xz: 0.008,
            n_yz: -0.005,
            corner_correction: 0.04,
            domain_wall_correction: 0.01,
        }
    }

    pub fn for_needle_shape(length_m: f64, width_m: f64, thickness_m: f64) -> Self {
        let (n_x, n_y, n_z) = Self::ellipsoid_demagnetization_factors(
            length_m, width_m, thickness_m
        );
        let aspect_ratio = length_m / (width_m + thickness_m).max(1e-9);
        let tip_correction = 0.02 * (aspect_ratio / 10.0).tanh();
        Self {
            n_xx: n_x,
            n_yy: n_y,
            n_zz: n_z,
            n_xy: 0.0,
            n_xz: tip_correction * 0.3,
            n_yz: tip_correction * 0.2,
            corner_correction: tip_correction,
            domain_wall_correction: tip_correction * 0.4,
        }
    }

    pub fn for_mems_chip(size_m: f64) -> Self {
        let (n_x, n_y, n_z) = Self::ellipsoid_demagnetization_factors(
            size_m, size_m, size_m * 0.25
        );
        Self {
            n_xx: n_x,
            n_yy: n_y,
            n_zz: n_z,
            n_xy: 0.0,
            n_xz: 0.0,
            n_yz: 0.0,
            corner_correction: 0.0,
            domain_wall_correction: 0.0,
        }
    }
}

#[derive(Clone)]
pub struct MicromagneticSimulator {
    pub boltzmann_constant: f64,
    pub vacuum_permeability: f64,
    pub gyromagnetic_ratio: f64,
    pub spoon_length_m: f64,
    pub spoon_width_m: f64,
    pub spoon_thickness_m: f64,
}

impl Default for MicromagneticSimulator {
    fn default() -> Self {
        Self {
            boltzmann_constant: 1.380649e-23,
            vacuum_permeability: 4.0 * PI * 1e-7,
            gyromagnetic_ratio: 1.760859644e11,
            spoon_length_m: 0.17,
            spoon_width_m: 0.08,
            spoon_thickness_m: 0.015,
        }
    }
}

impl MicromagneticSimulator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_spoon_dimensions(length_m: f64, width_m: f64, thickness_m: f64) -> Self {
        let mut s = Self::default();
        s.spoon_length_m = length_m;
        s.spoon_width_m = width_m;
        s.spoon_thickness_m = thickness_m;
        s
    }

    pub fn get_demagnetization_tensor(&self) -> DemagnetizationTensor {
        DemagnetizationTensor::for_spoon_shape(
            self.spoon_length_m,
            self.spoon_width_m,
            self.spoon_thickness_m,
        )
    }

    pub fn simulate_pointing(
        &self,
        params: &PointingSimulationParams,
        geomagnetic_field: Vector3<f64>,
    ) -> Result<PointingSimulationResult> {
        let demag_tensor = self.get_demagnetization_tensor();

        let magnetic_moment_vec = self.calculate_equilibrium_magnetization(
            params.magnetic_moment_magnitude,
            params.remanence,
            geomagnetic_field,
            params.anisotropy_constant,
            params.temperature,
        )?;

        let effective_moment = self.apply_demagnetization_tensor(
            magnetic_moment_vec,
            &demag_tensor,
            params.remanence,
            geomagnetic_field,
        )?;

        let torque = self.calculate_magnetic_torque(effective_moment, geomagnetic_field);

        let (simulated_azimuth, pointing_accuracy) = self.calculate_equilibrium_orientation(
            effective_moment,
            geomagnetic_field,
            torque,
            params.friction_coefficient,
            params.temperature,
            params.expected_azimuth,
            &demag_tensor,
        )?;

        let model_params = serde_json::json!({
            "boltzmann_constant": self.boltzmann_constant,
            "vacuum_permeability": self.vacuum_permeability,
            "gyromagnetic_ratio": self.gyromagnetic_ratio,
            "effective_moment_magnitude": effective_moment.magnitude(),
            "torque_magnitude": torque.magnitude(),
            "thermal_energy": self.boltzmann_constant * (params.temperature + 273.15),
            "magnetic_potential_energy": -effective_moment.dot(&geomagnetic_field),
            "demagnetization_tensor": {
                "n_xx": demag_tensor.n_xx,
                "n_yy": demag_tensor.n_yy,
                "n_zz": demag_tensor.n_zz,
                "n_xy": demag_tensor.n_xy,
                "n_xz": demag_tensor.n_xz,
                "n_yz": demag_tensor.n_yz,
                "corner_correction": demag_tensor.corner_correction,
                "domain_wall_correction": demag_tensor.domain_wall_correction,
            },
            "spoon_dimensions": {
                "length_m": self.spoon_length_m,
                "width_m": self.spoon_width_m,
                "thickness_m": self.spoon_thickness_m,
            },
        });

        Ok(PointingSimulationResult {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            device_id: params.device_id.clone(),
            simulation_id: params.simulation_id.clone(),
            target_year: params.target_year,
            location_lat: params.location_lat,
            location_lon: params.location_lon,
            expected_azimuth: params.expected_azimuth,
            simulated_azimuth,
            pointing_accuracy,
            magnetic_moment_magnitude: params.magnetic_moment_magnitude,
            remanence: params.remanence,
            temperature: params.temperature,
            friction_coefficient: params.friction_coefficient,
            demagnetization_factor: params.demagnetization_factor,
            anisotropy_constant: params.anisotropy_constant,
            model_parameters: model_params.to_string(),
        })
    }

    fn calculate_equilibrium_magnetization(
        &self,
        moment_magnitude: f64,
        remanence: f64,
        external_field: Vector3<f64>,
        anisotropy_constant: f64,
        temperature: f64,
    ) -> Result<Vector3<f64>> {
        let field_magnitude = external_field.magnitude();
        if field_magnitude < 1e-12 {
            return Err(AppError::SimulationError(
                "地磁场强度过小，无法进行有效仿真".to_string(),
            ));
        }

        let temp_kelvin = temperature + 273.15;
        let saturation_magnetization = remanence / self.vacuum_permeability;

        let field_unit = external_field / field_magnitude;

        let thermal_energy = self.boltzmann_constant * temp_kelvin;
        let anisotropy_energy = anisotropy_constant;
        let zeeman_energy = moment_magnitude * field_magnitude;

        let effective_field_ratio = zeeman_energy / thermal_energy;

        let langevin_argument = if effective_field_ratio > 100.0 {
            1.0 - 1.0 / effective_field_ratio
        } else if effective_field_ratio < 0.01 {
            effective_field_ratio / 3.0
        } else {
            (effective_field_ratio).cosh() / (effective_field_ratio).sinh()
                - 1.0 / effective_field_ratio
        };

        let anisotropy_factor = (anisotropy_energy / thermal_energy).tanh();
        let total_magnetization_ratio = langevin_argument * (1.0 + 0.3 * anisotropy_factor);

        let effective_magnitude = saturation_magnetization * total_magnetization_ratio.max(0.1);

        Ok(field_unit * effective_magnitude)
    }

    #[allow(dead_code)]
    fn apply_demagnetization(
        &self,
        magnetization: Vector3<f64>,
        demagnetization_factor: f64,
        remanence: f64,
    ) -> Result<Vector3<f64>> {
        let n = demagnetization_factor.clamp(0.0, 1.0 / 3.0);
        let demagnetizing_field = -magnetization * n;
        let internal_field = demagnetizing_field;

        let saturation_magnetization = remanence / self.vacuum_permeability;
        let susceptibility = 100.0;

        let effective_magnetization = magnetization + internal_field * susceptibility;

        let scale = if effective_magnetization.magnitude() > saturation_magnetization {
            saturation_magnetization / effective_magnetization.magnitude()
        } else {
            1.0
        };

        Ok(effective_magnetization * scale)
    }

    fn apply_demagnetization_tensor(
        &self,
        magnetization: Vector3<f64>,
        demag_tensor: &DemagnetizationTensor,
        remanence: f64,
        external_field: Vector3<f64>,
    ) -> Result<Vector3<f64>> {
        let n_matrix = demag_tensor.as_matrix();

        let demagnetizing_field_vec = -n_matrix * magnetization;

        let corner_field = self.calculate_corner_demagnetizing_field(
            &magnetization,
            demag_tensor.corner_correction,
        );

        let domain_wall_field = self.calculate_domain_wall_field(
            &magnetization,
            demag_tensor.domain_wall_correction,
        );

        let total_demag_field = demagnetizing_field_vec + corner_field + domain_wall_field;

        let saturation_magnetization = remanence / self.vacuum_permeability;
        let susceptibility = self.calculate_field_dependent_susceptibility(
            &external_field,
            &total_demag_field,
        );

        let internal_field = external_field * self.vacuum_permeability + total_demag_field;
        let mut effective_magnetization = magnetization + internal_field * susceptibility;

        let mag_magnitude = effective_magnetization.magnitude();
        if mag_magnitude > saturation_magnetization && mag_magnitude > 1e-20 {
            effective_magnetization = effective_magnetization * (saturation_magnetization / mag_magnitude);
        }

        if mag_magnitude < saturation_magnetization * 0.1 && saturation_magnetization > 1e-20 {
            let boost_factor = 0.15 * demag_tensor.corner_correction;
            effective_magnetization = effective_magnetization * (1.0 + boost_factor);
        }

        Ok(effective_magnetization)
    }

    fn calculate_corner_demagnetizing_field(
        &self,
        magnetization: &Vector3<f64>,
        corner_correction: f64,
    ) -> Vector3<f64> {
        let length_ratio = self.spoon_length_m / self.spoon_width_m.max(1e-6);
        let thickness_ratio = self.spoon_width_m / self.spoon_thickness_m.max(1e-6);

        let bowl_edge_factor = 0.6 * corner_correction;
        let handle_tip_factor = 0.4 * corner_correction;

        let mag_normalized = if magnetization.magnitude() > 1e-20 {
            magnetization / magnetization.magnitude()
        } else {
            Vector3::new(1.0, 0.0, 0.0)
        };

        let bowl_field = -mag_normalized * bowl_edge_factor * (thickness_ratio / 5.0).tanh();

        let handle_dir = Vector3::new(1.0, 0.0, 0.0);
        let handle_component = mag_normalized.dot(&handle_dir);
        let handle_field = -handle_dir * handle_component * handle_tip_factor
            * (length_ratio / 3.0).tanh();

        let edge_shear = Vector3::new(
            0.0,
            corner_correction * 0.15 * mag_normalized.x,
            -corner_correction * 0.1 * mag_normalized.y,
        );

        bowl_field + handle_field + edge_shear
    }

    fn calculate_domain_wall_field(
        &self,
        magnetization: &Vector3<f64>,
        domain_wall_correction: f64,
    ) -> Vector3<f64> {
        let mag_xy = Vector3::new(magnetization.x, magnetization.y, 0.0);
        let xy_magnitude = mag_xy.magnitude();

        if xy_magnitude < 1e-20 {
            return Vector3::zeros();
        }

        let wall_count_factor = (self.spoon_length_m / 1e-6).sqrt();
        let pinning_strength = domain_wall_correction * wall_count_factor * 0.01;

        let domain_avg = -magnetization * pinning_strength * 0.5;

        let wall_roughness = Vector3::new(
            (magnetization.y * pinning_strength * 0.3).tanh(),
            (-magnetization.x * pinning_strength * 0.3).tanh(),
            0.0,
        );

        domain_avg + wall_roughness
    }

    fn calculate_field_dependent_susceptibility(
        &self,
        external_field: &Vector3<f64>,
        demag_field: &Vector3<f64>,
    ) -> f64 {
        let total_field_mag = (external_field * self.vacuum_permeability + demag_field).magnitude();

        let base_susceptibility = 100.0;

        if total_field_mag < 1e-12 {
            return base_susceptibility;
        }

        let saturation_field = 50e-3;
        let field_ratio = total_field_mag / saturation_field;

        base_susceptibility / (1.0 + field_ratio * 0.5)
    }

    fn calculate_magnetic_torque(
        &self,
        magnetic_moment: Vector3<f64>,
        magnetic_field: Vector3<f64>,
    ) -> Vector3<f64> {
        magnetic_moment.cross(&magnetic_field) * self.vacuum_permeability
    }

    fn calculate_equilibrium_orientation(
        &self,
        magnetic_moment: Vector3<f64>,
        magnetic_field: Vector3<f64>,
        torque: Vector3<f64>,
        friction_coefficient: f64,
        temperature: f64,
        expected_azimuth: f64,
        demag_tensor: &DemagnetizationTensor,
    ) -> Result<(f64, f64)> {
        let field_xy = Vector3::new(magnetic_field.x, magnetic_field.y, 0.0);
        let theoretical_azimuth = if field_xy.magnitude() > 1e-12 {
            let angle = field_xy.y.atan2(field_xy.x);
            (angle * 180.0 / PI + 360.0) % 360.0
        } else {
            expected_azimuth
        };

        let torque_magnitude = torque.magnitude();
        let moment_magnitude = magnetic_moment.magnitude();
        let field_magnitude = magnetic_field.magnitude();

        let max_torque = self.vacuum_permeability * moment_magnitude * field_magnitude;

        let alignment_angle = if max_torque > 1e-20 {
            (torque_magnitude / max_torque).asin() * 180.0 / PI
        } else {
            0.0
        };

        let temp_kelvin = temperature + 273.15;
        let thermal_energy = self.boltzmann_constant * temp_kelvin;
        let magnetic_energy = self.vacuum_permeability * moment_magnitude * field_magnitude;

        let stability_ratio = magnetic_energy / thermal_energy;

        let shape_anisotropy_bias = self.calculate_shape_anisotropy_bias(
            demag_tensor,
            &magnetic_moment,
        );

        let corner_noise_amplitude = demag_tensor.corner_correction * 2.5;
        let domain_wall_drift = demag_tensor.domain_wall_correction * 1.5;

        let thermal_fluctuation = if stability_ratio > 1.0 {
            (1.0 / stability_ratio).sqrt() * (5.0 + corner_noise_amplitude + domain_wall_drift)
        } else {
            15.0 + corner_noise_amplitude * 2.0 + domain_wall_drift
        };

        let friction_damping = 1.0 - friction_coefficient.clamp(0.0, 0.9) * 0.5;

        let mut rng = rand::thread_rng();
        let normal = Normal::new(0.0, 1.0).unwrap();

        let random_variation = normal.sample(&mut rng) * thermal_fluctuation * friction_damping;

        let simulated_azimuth = theoretical_azimuth
            + alignment_angle * 0.3
            + shape_anisotropy_bias
            + random_variation;

        let mut deviation = (simulated_azimuth - expected_azimuth).abs();
        if deviation > 180.0 {
            deviation = 360.0 - deviation;
        }

        let finite_element_correction = 1.0 + demag_tensor.corner_correction * 0.4
            + demag_tensor.domain_wall_correction * 0.3;
        let corrected_deviation = deviation * finite_element_correction;

        let accuracy = if corrected_deviation < 0.5 {
            0.1
        } else if corrected_deviation < 2.0 {
            0.5
        } else if corrected_deviation < 5.0 {
            1.0
        } else {
            2.0
        };

        Ok((simulated_azimuth, accuracy))
    }

    fn calculate_shape_anisotropy_bias(
        &self,
        demag_tensor: &DemagnetizationTensor,
        magnetic_moment: &Vector3<f64>,
    ) -> f64 {
        let n_diff = demag_tensor.n_yy - demag_tensor.n_xx;

        let moment_xy_mag = (magnetic_moment.x.powi(2) + magnetic_moment.y.powi(2)).sqrt();
        if moment_xy_mag < 1e-20 {
            return 0.0;
        }

        let moment_angle = magnetic_moment.y.atan2(magnetic_moment.x);
        let sin_2theta = (2.0 * moment_angle).sin();

        let anisotropy_field = n_diff * moment_xy_mag;
        let bias_angle_deg = (anisotropy_field * sin_2theta * 1e3).clamp(-3.0, 3.0);

        bias_angle_deg + demag_tensor.n_xy * 5.0
    }

    pub fn calculate_pointing_deviation(
        &self,
        measured_moment: Vector3<f64>,
        geomagnetic_field: Vector3<f64>,
    ) -> Result<f64> {
        let moment_xy = Vector3::new(measured_moment.x, measured_moment.y, 0.0);
        let field_xy = Vector3::new(geomagnetic_field.x, geomagnetic_field.y, 0.0);

        if moment_xy.magnitude() < 1e-12 || field_xy.magnitude() < 1e-12 {
            return Err(AppError::SimulationError(
                "磁矩或地磁场在水平面分量过小，无法计算指向偏差".to_string(),
            ));
        }

        let moment_azimuth = (moment_xy.y.atan2(moment_xy.x) * 180.0 / PI + 360.0) % 360.0;
        let field_azimuth = (field_xy.y.atan2(field_xy.x) * 180.0 / PI + 360.0) % 360.0;

        let mut deviation = (moment_azimuth - field_azimuth).abs();
        if deviation > 180.0 {
            deviation = 360.0 - deviation;
        }

        Ok(deviation)
    }

    pub fn calculate_magnetic_moment_from_sensor(
        &self,
        mx: f64,
        my: f64,
        mz: f64,
    ) -> Vector3<f64> {
        Vector3::new(mx, my, mz)
    }

    pub fn calculate_field_components(
        &self,
        intensity: f64,
        declination: f64,
        inclination: f64,
    ) -> Vector3<f64> {
        let dec_rad = declination * PI / 180.0;
        let inc_rad = inclination * PI / 180.0;

        let h = intensity * inc_rad.cos();
        let bx = h * dec_rad.cos();
        let by = h * dec_rad.sin();
        let bz = intensity * inc_rad.sin();

        Vector3::new(bx, by, bz)
    }

    pub fn stoner_wohlfarth_switching(
        &self,
        anisotropy_constant: f64,
        saturation_magnetization: f64,
        applied_field: f64,
        field_angle: f64,
    ) -> Result<f64> {
        let anisotropy_field = 2.0 * anisotropy_constant / (self.vacuum_permeability * saturation_magnetization);

        if anisotropy_field < 1e-12 {
            return Err(AppError::SimulationError(
                "各向异性场过小，无法计算开关场".to_string(),
            ));
        }

        let theta = field_angle * PI / 180.0;
        let reduced_field = applied_field / anisotropy_field;

        let switching_field = if theta.abs() < 1e-6 {
            1.0
        } else {
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();
            (sin_theta.powf(2.0/3.0) + cos_theta.powf(2.0/3.0)).powf(-1.5)
        };

        Ok(switching_field * anisotropy_field)
    }

    pub fn get_demagnetization_tensor_for_device(&self, geom: &DeviceGeometryParams) -> DemagnetizationTensor {
        match geom.device_type {
            DeviceType::Sinan => DemagnetizationTensor::for_spoon_shape(
                geom.length_m, geom.width_m, geom.thickness_m
            ),
            DeviceType::Zhinanyu => DemagnetizationTensor::for_fish_shape(
                geom.length_m, geom.width_m, geom.thickness_m
            ),
            DeviceType::HanLuopan => DemagnetizationTensor::for_needle_shape(
                geom.length_m, geom.width_m, geom.thickness_m
            ),
            DeviceType::MemsCompass => DemagnetizationTensor::for_mems_chip(geom.length_m),
        }
    }

    fn get_device_default_moment(&self, dt: DeviceType) -> (f64, f64) {
        match dt {
            DeviceType::Sinan => (0.05, 0.4),
            DeviceType::Zhinanyu => (0.002, 0.3),
            DeviceType::HanLuopan => (0.0005, 0.6),
            DeviceType::MemsCompass => (0.0, 0.0),
        }
    }

    fn get_device_notes(&self, dt: DeviceType) -> String {
        match dt {
            DeviceType::Sinan => "战国至汉代发明，天然磁铁矿琢磨成勺形，放置于青铜地盘。因勺底摩擦较大、天然磁石磁性较弱，典型指向误差约5°-20°。文献记载：'司南之杓，投之于地，其柢指南'（《韩非子·有度》）。".to_string(),
            DeviceType::Zhinanyu => "北宋《武经总要》记载的军事指南工具，薄铁叶剪裁成鱼形，经地磁场磁化后浮于水面。水浮摩擦极小，但铁片剩磁较弱，典型误差约3°-10°。".to_string(),
            DeviceType::HanLuopan => "宋代出现的旱罗盘，将磁针贯穿灯芯草或支承于尖枢轴上，配合二十四向方位盘。磁针细长形各向异性强、摩擦极小，典型误差约1°-5°，是古代航海的主要导航工具。".to_string(),
            DeviceType::MemsCompass => "基于MEMS（微机电系统）技术的现代电子罗盘，通常采用各向异性磁阻(AMR)或隧穿磁阻(TMR)传感器，配合三轴加速度计倾角补偿、硬铁/软铁校准，典型精度0.5°-2°，消费级可达0.3°以内。".to_string(),
        }
    }

    fn simulate_mems_compass(&self, geomagnetic_field: Vector3<f64>, expected_azimuth: f64, temperature: f64) -> (f64, f64, f64) {
        let field_xy = Vector3::new(geomagnetic_field.x, geomagnetic_field.y, 0.0);
        let theoretical_azimuth = if field_xy.magnitude() > 1e-12 {
            let angle = field_xy.y.atan2(field_xy.x);
            (angle * 180.0 / PI + 360.0) % 360.0
        } else {
            expected_azimuth
        };

        let base_noise_std = 0.8;
        let temp_drift = ((temperature - 25.0) * 0.02).abs();
        let hard_iron_offset = 0.3;
        let total_std = base_noise_std + temp_drift + hard_iron_offset;

        let mut rng = rand::thread_rng();
        let normal = Normal::new(0.0, 1.0).unwrap();
        let noise = normal.sample(&mut rng) * total_std;

        let simulated = theoretical_azimuth + noise;
        let mut deviation = (simulated - expected_azimuth).abs();
        if deviation > 180.0 {
            deviation = 360.0 - deviation;
        }
        (simulated, deviation, total_std)
    }

    pub fn simulate_device_pointing(
        &self,
        device_type: DeviceType,
        geomagnetic_field: Vector3<f64>,
        target_year: f64,
        location_lat: f64,
        location_lon: f64,
        temperature: f64,
        expected_azimuth: f64,
        magnetic_moment: Option<f64>,
        remanence: Option<f64>,
    ) -> Result<SingleDeviceAccuracy> {
        let geom = DeviceGeometryParams::for_type(device_type);
        let (default_moment, default_remanence) = self.get_device_default_moment(device_type);
        let effective_moment = magnetic_moment.unwrap_or(default_moment);
        let effective_remanence = remanence.unwrap_or(default_remanence);

        if matches!(device_type, DeviceType::MemsCompass) {
            let (sim_az, dev, std_dev) = self.simulate_mems_compass(
                geomagnetic_field, expected_azimuth, temperature
            );
            return Ok(SingleDeviceAccuracy {
                device_type,
                display_name: device_type.display_name().to_string(),
                era: device_type.era().to_string(),
                simulated_azimuth: sim_az,
                pointing_accuracy_deg: if dev < 1.0 { 0.3 } else if dev < 2.0 { 0.5 } else { 1.0 },
                mean_deviation_deg: dev,
                std_deviation_deg: std_dev,
                min_deviation_deg: (dev - 2.0 * std_dev).max(0.0),
                max_deviation_deg: dev + 2.0 * std_dev,
                p95_deviation_deg: dev + 1.645 * std_dev,
                geometry: geom,
                notes: self.get_device_notes(device_type),
            });
        }

        let samples = 50usize;
        let mut deviations: Vec<f64> = Vec::with_capacity(samples);
        let mut simulated_azimuths: Vec<f64> = Vec::with_capacity(samples);

        let friction = match device_type {
            DeviceType::Sinan => geom.pivot_friction,
            DeviceType::Zhinanyu => 0.002 + geom.water_viscosity.unwrap_or(0.001),
            DeviceType::HanLuopan => geom.pivot_friction,
            DeviceType::MemsCompass => 0.0,
        };

        for _ in 0..samples {
            let params = PointingSimulationParams {
                device_id: format!("{:?}", device_type),
                simulation_id: uuid::Uuid::new_v4().to_string(),
                target_year,
                location_lat,
                location_lon,
                magnetic_moment_magnitude: effective_moment,
                remanence: effective_remanence,
                temperature,
                friction_coefficient: friction,
                demagnetization_factor: 0.1,
                anisotropy_constant: match device_type {
                    DeviceType::HanLuopan => 5000.0,
                    DeviceType::Zhinanyu => 1000.0,
                    _ => 500.0,
                },
                expected_azimuth,
            };

            let sim = self.simulate_pointing_with_device(&params, geomagnetic_field, &geom)?;
            simulated_azimuths.push(sim.simulated_azimuth);
            let mut dev = (sim.simulated_azimuth - expected_azimuth).abs();
            if dev > 180.0 { dev = 360.0 - dev; }
            deviations.push(dev);
        }

        deviations.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mean_dev = deviations.iter().sum::<f64>() / deviations.len() as f64;
        let variance = deviations.iter().map(|d| (d - mean_dev).powi(2)).sum::<f64>() / deviations.len() as f64;
        let std_dev = variance.sqrt();
        let p95_idx = ((deviations.len() as f64) * 0.95) as usize;
        let p95 = deviations[p95_idx.min(deviations.len() - 1)];

        let avg_az = simulated_azimuths.iter().sum::<f64>() / simulated_azimuths.len() as f64;

        let accuracy = if mean_dev < 1.0 { 0.5 }
            else if mean_dev < 3.0 { 1.0 }
            else if mean_dev < 7.0 { 2.0 }
            else if mean_dev < 15.0 { 5.0 }
            else { 10.0 };

        Ok(SingleDeviceAccuracy {
            device_type,
            display_name: device_type.display_name().to_string(),
            era: device_type.era().to_string(),
            simulated_azimuth: avg_az,
            pointing_accuracy_deg: accuracy,
            mean_deviation_deg: mean_dev,
            std_deviation_deg: std_dev,
            min_deviation_deg: *deviations.first().unwrap_or(&0.0),
            max_deviation_deg: *deviations.last().unwrap_or(&0.0),
            p95_deviation_deg: p95,
            geometry: geom,
            notes: self.get_device_notes(device_type),
        })
    }

    fn simulate_pointing_with_device(
        &self,
        params: &PointingSimulationParams,
        geomagnetic_field: Vector3<f64>,
        geom: &DeviceGeometryParams,
    ) -> Result<PointingSimulationResult> {
        let saved_length = self.spoon_length_m;
        let saved_width = self.spoon_width_m;
        let saved_thickness = self.spoon_thickness_m;

        let mut sim = self.clone();
        sim.spoon_length_m = geom.length_m;
        sim.spoon_width_m = geom.width_m;
        sim.spoon_thickness_m = geom.thickness_m;

        let result = sim.simulate_pointing(params, geomagnetic_field);

        let _ = (saved_length, saved_width, saved_thickness);
        result
    }

    pub fn compare_multiple_devices(
        &self,
        request: &MultiDeviceCompareRequest,
        geomagnetic_field: Vector3<f64>,
    ) -> Result<MultiDeviceCompareResponse> {
        let mut results = Vec::new();
        let devices = if request.devices.is_empty() {
            DeviceType::all()
        } else {
            request.devices.clone()
        };

        for dt in &devices {
            let acc = self.simulate_device_pointing(
                *dt,
                geomagnetic_field,
                request.target_year,
                request.location_lat,
                request.location_lon,
                request.temperature,
                request.expected_azimuth,
                request.magnetic_moment_magnitude,
                request.remanence,
            )?;
            results.push(acc);
        }

        let mut ranking = results.clone();
        ranking.sort_by(|a, b| a.mean_deviation_deg.partial_cmp(&b.mean_deviation_deg).unwrap());
        let ranking: Vec<DeviceType> = ranking.iter().map(|r| r.device_type).collect();

        let best = results.iter().min_by(|a, b| a.mean_deviation_deg.partial_cmp(&b.mean_deviation_deg).unwrap());
        let worst = results.iter().max_by(|a, b| a.mean_deviation_deg.partial_cmp(&b.mean_deviation_deg).unwrap());
        let summary = match (best, worst) {
            (Some(b), Some(w)) => format!(
                "精度排名：{}（最优，平均偏差{:.2}°）> {}（最差，平均偏差{:.2}°）；精度差异约{:.1}倍，反映了从战国到现代约2300年的导航技术进步。",
                b.display_name, b.mean_deviation_deg,
                w.display_name, w.mean_deviation_deg,
                w.mean_deviation_deg / b.mean_deviation_deg.max(0.01)
            ),
            _ => "对比完成".to_string(),
        };

        let field_intensity = geomagnetic_field.magnitude() * 1e9;
        let (declination, inclination) = self.field_to_di(geomagnetic_field);

        Ok(MultiDeviceCompareResponse {
            target_year: request.target_year,
            location_lat: request.location_lat,
            location_lon: request.location_lon,
            expected_azimuth: request.expected_azimuth,
            geomagnetic_intensity_nT: field_intensity,
            geomagnetic_declination_deg: declination,
            geomagnetic_inclination_deg: inclination,
            devices: results,
            ranking,
            summary,
        })
    }

    pub fn simulate_interference(
        &self,
        request: &InterferenceSimulationRequest,
        geomagnetic_field: Vector3<f64>,
    ) -> Result<InterferenceSimulationResponse> {
        let geom = DeviceGeometryParams::for_type(request.device_type);

        let (default_moment, default_remanence) = self.get_device_default_moment(request.device_type);
        let effective_moment = request.magnetic_moment_magnitude.unwrap_or(default_moment);
        let effective_remanence = request.remanence.unwrap_or(default_remanence);

        let baseline = self.simulate_device_pointing(
            request.device_type,
            geomagnetic_field,
            request.target_year,
            request.location_lat,
            request.location_lon,
            request.temperature,
            request.expected_azimuth,
            Some(effective_moment),
            Some(effective_remanence),
        )?;

        let mut total_induced = Vector3::zeros();
        let mut effects = Vec::new();

        for src in &request.interference_sources {
            let (induced_vec, effect) = self.calculate_interference_field(
                src, geomagnetic_field
            );
            total_induced = total_induced + induced_vec;
            effects.push(effect);
        }

        let interfered_field = geomagnetic_field + total_induced;

        let interfered = self.simulate_device_pointing(
            request.device_type,
            interfered_field,
            request.target_year,
            request.location_lat,
            request.location_lon,
            request.temperature,
            request.expected_azimuth,
            Some(effective_moment),
            Some(effective_remanence),
        )?;

        let delta = (interfered.mean_deviation_deg - baseline.mean_deviation_deg).abs();
        let total_induced_nT = total_induced.magnitude() * 1e9;
        let geo_nT = geomagnetic_field.magnitude() * 1e9;
        let interference_ratio = total_induced_nT / geo_nT.max(1.0);

        let warning_level = if interference_ratio < 0.05 { "安全".to_string() }
            else if interference_ratio < 0.2 { "轻微干扰".to_string() }
            else if interference_ratio < 0.5 { "中度干扰".to_string() }
            else { "严重干扰".to_string() };

        let recommendation = if interference_ratio < 0.05 {
            "当前环境对指向装置影响极小，可放心使用。".to_string()
        } else if interference_ratio < 0.2 {
            "建议远离干扰源至少1-2米，以获得更准确的指向。".to_string()
        } else if interference_ratio < 0.5 {
            "干扰显著！建议将装置移至室外或距离大型金属/电气设备5米以上的区域。".to_string()
        } else {
            "严重干扰警告！当前位置不适合使用任何磁指向装置，请移动到开阔区域或更换为惯性/GPS导航。".to_string()
        };

        let _ = geom;
        Ok(InterferenceSimulationResponse {
            device_type: request.device_type,
            target_year: request.target_year,
            location_lat: request.location_lat,
            location_lon: request.location_lon,
            baseline_azimuth: baseline.simulated_azimuth,
            baseline_accuracy_deg: baseline.mean_deviation_deg,
            interfered_azimuth: interfered.simulated_azimuth,
            interfered_accuracy_deg: interfered.mean_deviation_deg,
            total_deviation_delta_deg: delta,
            total_interference_field_nT: total_induced_nT,
            interference_ratio,
            effects,
            warning_level,
            recommendation,
        })
    }

    fn calculate_interference_field(
        &self,
        src: &InterferenceSource,
        geomagnetic_field: Vector3<f64>,
    ) -> (Vector3<f64>, InterferenceEffect) {
        let base_amplitude_nT: f64 = match src.interference_type {
            InterferenceType::FerrousObject => 5000.0,
            InterferenceType::PowerLine => 2000.0,
            InterferenceType::ElectronicDevice => 800.0,
            InterferenceType::BuildingRebar => 1500.0,
            InterferenceType::Loudspeaker => 10000.0,
            InterferenceType::LightningStorm => 3000.0,
        };

        let distance = src.distance_m.max(0.01);
        let dipole_decay = 1.0 / distance.powi(3);
        let induced_nT = base_amplitude_nT * src.intensity_factor * dipole_decay;

        let az_rad = src.azimuth_deg * PI / 180.0;
        let geo_inclination = self.field_to_di(geomagnetic_field).1;
        let inc_rad = geo_inclination * PI / 180.0;

        let h = induced_nT * inc_rad.cos();
        let bx = h * az_rad.cos();
        let by = h * az_rad.sin();
        let bz = induced_nT * inc_rad.sin();

        let induced_vec = Vector3::new(bx * 1e-9, by * 1e-9, bz * 1e-9);

        let geo_xy = Vector3::new(geomagnetic_field.x, geomagnetic_field.y, 0.0);
        let ind_xy = Vector3::new(induced_vec.x, induced_vec.y, 0.0);
        let mut dev_contr = 0.0;
        if geo_xy.magnitude() > 1e-20 && ind_xy.magnitude() > 1e-20 {
            let geo_az = geo_xy.y.atan2(geo_xy.x);
            let ind_az = ind_xy.y.atan2(ind_xy.x);
            let angle_diff = ((ind_az - geo_az).sin()).abs();
            let ratio = ind_xy.magnitude() / geo_xy.magnitude().max(1e-20);
            dev_contr = (ratio * angle_diff * 180.0 / PI).min(45.0);
        }

        (induced_vec, InterferenceEffect {
            interference_type: src.interference_type,
            display_name: src.interference_type.display_name().to_string(),
            distance_m: src.distance_m,
            induced_field_nT: induced_nT,
            induced_field_azimuth_deg: src.azimuth_deg,
            deviation_contribution_deg: dev_contr,
        })
    }

    fn field_to_di(&self, field: Vector3<f64>) -> (f64, f64) {
        let field_xy = Vector3::new(field.x, field.y, 0.0);
        let declination = if field_xy.magnitude() > 1e-12 {
            field_xy.y.atan2(field_xy.x) * 180.0 / PI
        } else { 0.0 };
        let inclination = if field.magnitude() > 1e-12 {
            field.z.atan2(field_xy.magnitude()) * 180.0 / PI
        } else { 0.0 };
        (declination, inclination)
    }

    pub fn simulate_interactive(
        &self,
        request: &InteractiveSinanRequest,
        geomagnetic_field: Vector3<f64>,
    ) -> Result<InteractiveSinanResponse> {
        let geom = DeviceGeometryParams::for_type(request.device_type);

        let length = request.spoon_length_m.unwrap_or(geom.length_m);
        let width = request.spoon_width_m.unwrap_or(geom.width_m);
        let thickness = request.spoon_thickness_m.unwrap_or(geom.thickness_m);

        let mut sim = self.clone();
        sim.spoon_length_m = length;
        sim.spoon_width_m = width;
        sim.spoon_thickness_m = thickness;

        let demag_tensor = match request.device_type {
            DeviceType::Sinan => DemagnetizationTensor::for_spoon_shape(length, width, thickness),
            DeviceType::Zhinanyu => DemagnetizationTensor::for_fish_shape(length, width, thickness),
            DeviceType::HanLuopan => DemagnetizationTensor::for_needle_shape(length, width, thickness),
            DeviceType::MemsCompass => DemagnetizationTensor::for_mems_chip(length),
        };

        let params = PointingSimulationParams {
            device_id: format!("interactive-{:?}", request.device_type),
            simulation_id: uuid::Uuid::new_v4().to_string(),
            target_year: request.target_year,
            location_lat: request.location_lat,
            location_lon: request.location_lon,
            magnetic_moment_magnitude: request.magnetic_moment_magnitude,
            remanence: request.remanence,
            temperature: request.temperature,
            friction_coefficient: request.friction_coefficient,
            demagnetization_factor: request.demagnetization_factor_override.unwrap_or(0.1),
            anisotropy_constant: request.anisotropy_constant,
            expected_azimuth: request.expected_azimuth,
        };

        if matches!(request.device_type, DeviceType::MemsCompass) {
            let (sim_az, dev, std_dev) = self.simulate_mems_compass(
                geomagnetic_field, request.expected_azimuth, request.temperature
            );
            let mut demag_map = HashMap::new();
            demag_map.insert("n_xx".to_string(), demag_tensor.n_xx);
            demag_map.insert("n_yy".to_string(), demag_tensor.n_yy);
            demag_map.insert("n_zz".to_string(), demag_tensor.n_zz);

            return Ok(InteractiveSinanResponse {
                device_type: request.device_type,
                simulated_azimuth: sim_az,
                pointing_accuracy_deg: dev,
                expected_azimuth: request.expected_azimuth,
                magnetic_moment_vector: [0.0, 0.0, 0.0],
                effective_moment_magnitude: 0.0,
                torque_magnitude: 0.0,
                thermal_fluctuation_deg: std_dev,
                demagnetization_tensor: demag_map,
                geomagnetic_field: [geomagnetic_field.x, geomagnetic_field.y, geomagnetic_field.z],
                geomagnetic_intensity_nT: geomagnetic_field.magnitude() * 1e9,
                geomagnetic_declination_deg: self.field_to_di(geomagnetic_field).0,
                geomagnetic_inclination_deg: self.field_to_di(geomagnetic_field).1,
                spoon_dimensions_m: [length, width, thickness],
                physics_insights: vec![
                    "现代MEMS电子罗盘采用磁阻传感器（AMR/TMR），不依赖永磁体磁矩。".to_string(),
                    format!("当前噪声标准差约{:.2}°，包含温度漂移和硬铁偏移。", std_dev),
                    "MEMS罗盘需配合加速度计做倾角补偿，并定期做软硬铁校准。".to_string(),
                ],
            });
        }

        let mag_vec = sim.calculate_equilibrium_magnetization(
            request.magnetic_moment_magnitude,
            request.remanence,
            geomagnetic_field,
            request.anisotropy_constant,
            request.temperature,
        )?;

        let effective_moment = sim.apply_demagnetization_tensor(
            mag_vec,
            &demag_tensor,
            request.remanence,
            geomagnetic_field,
        )?;

        let torque = sim.calculate_magnetic_torque(effective_moment, geomagnetic_field);

        let (sim_az, _accuracy) = sim.calculate_equilibrium_orientation(
            effective_moment,
            geomagnetic_field,
            torque,
            request.friction_coefficient,
            request.temperature,
            request.expected_azimuth,
            &demag_tensor,
        )?;

        let mut dev = (sim_az - request.expected_azimuth).abs();
        if dev > 180.0 { dev = 360.0 - dev; }

        let temp_kelvin = request.temperature + 273.15;
        let thermal_energy = self.boltzmann_constant * temp_kelvin;
        let magnetic_energy = self.vacuum_permeability * effective_moment.magnitude() * geomagnetic_field.magnitude();
        let stability_ratio = magnetic_energy / thermal_energy.max(1e-30);
        let thermal_fluctuation = if stability_ratio > 1.0 {
            (1.0 / stability_ratio).sqrt() * 5.0
        } else { 15.0 };

        let mut demag_map = HashMap::new();
        demag_map.insert("n_xx".to_string(), demag_tensor.n_xx);
        demag_map.insert("n_yy".to_string(), demag_tensor.n_yy);
        demag_map.insert("n_zz".to_string(), demag_tensor.n_zz);
        demag_map.insert("n_xy".to_string(), demag_tensor.n_xy);
        demag_map.insert("n_xz".to_string(), demag_tensor.n_xz);
        demag_map.insert("n_yz".to_string(), demag_tensor.n_yz);
        demag_map.insert("corner_correction".to_string(), demag_tensor.corner_correction);
        demag_map.insert("domain_wall_correction".to_string(), demag_tensor.domain_wall_correction);

        let mut insights = Vec::new();
        insights.push(format!(
            "当前装置尺寸：长{:.1}cm × 宽{:.1}cm × 厚{:.2}cm，退磁因子(Nxx,Nyy,Nzz)=({:.3}, {:.3}, {:.3})。",
            length * 100.0, width * 100.0, thickness * 100.0,
            demag_tensor.n_xx, demag_tensor.n_yy, demag_tensor.n_zz
        ));
        insights.push(format!(
            "平衡磁化强度 {:.2} kA/m，有效磁矩 {:.3} A·m²，施加到磁矩上的力矩 {:.2e} N·m。",
            mag_vec.magnitude() / 1000.0,
            effective_moment.magnitude(),
            torque.magnitude()
        ));
        insights.push(format!(
            "热涨落能量 {:.2e} J，磁能 {:.2e} J，能垒比={:.1}；若此值<10则热扰动显著。",
            thermal_energy, magnetic_energy, stability_ratio
        ));
        if request.friction_coefficient > 0.1 {
            insights.push(format!(
                "摩擦系数 {:.2} 偏高，勺体与地盘间机械阻力会掩盖微小磁力矩，导致指向模糊。建议使用更光滑的铜质地盘。",
                request.friction_coefficient
            ));
        }
        if request.magnetic_moment_magnitude < 0.01 {
            insights.push("磁矩不足！天然磁铁矿的磁矩通常在0.02-0.1 A·m²之间，磁矩过小将无法克服摩擦。".to_string());
        }
        if thermal_fluctuation > 5.0 {
            insights.push(format!("热涨落较大（σ≈{:.1}°），建议降低温度或使用剩磁更高的磁石材料。", thermal_fluctuation));
        }

        Ok(InteractiveSinanResponse {
            device_type: request.device_type,
            simulated_azimuth: sim_az,
            pointing_accuracy_deg: dev,
            expected_azimuth: request.expected_azimuth,
            magnetic_moment_vector: [effective_moment.x, effective_moment.y, effective_moment.z],
            effective_moment_magnitude: effective_moment.magnitude(),
            torque_magnitude: torque.magnitude(),
            thermal_fluctuation_deg: thermal_fluctuation,
            demagnetization_tensor: demag_map,
            geomagnetic_field: [geomagnetic_field.x, geomagnetic_field.y, geomagnetic_field.z],
            geomagnetic_intensity_nT: geomagnetic_field.magnitude() * 1e9,
            geomagnetic_declination_deg: self.field_to_di(geomagnetic_field).0,
            geomagnetic_inclination_deg: self.field_to_di(geomagnetic_field).1,
            spoon_dimensions_m: [length, width, thickness],
            physics_insights: insights,
        })
    }

    pub fn compare_cross_era(
        &self,
        request: &CrossEraCompareRequest,
        ancient_field: Vector3<f64>,
        modern_field: Vector3<f64>,
    ) -> Result<CrossEraCompareResponse> {
        let ancient = self.simulate_device_pointing(
            request.ancient_device,
            ancient_field,
            request.ancient_year,
            request.location_lat,
            request.location_lon,
            request.temperature,
            request.expected_azimuth,
            None,
            None,
        )?;

        let modern = self.simulate_device_pointing(
            DeviceType::MemsCompass,
            modern_field,
            request.modern_year,
            request.location_lat,
            request.location_lon,
            request.temperature,
            request.expected_azimuth,
            None,
            None,
        )?;

        let improvement_factor = ancient.mean_deviation_deg / modern.mean_deviation_deg.max(0.01);
        let accuracy_gap_deg = ancient.mean_deviation_deg - modern.mean_deviation_deg;

        let narrative = format!(
            "从汉代的{}（平均偏差{:.2}°）到现代MEMS电子罗盘（平均偏差{:.2}°），\
            人类磁导航精度在约2000年间提升了约{:.0}倍，指向误差从数度级别降低到亚度级别，\
            这直接支撑了从陆地短途辨向到远洋航海再到无人机精密导航的技术跃迁。",
            ancient.display_name, ancient.mean_deviation_deg,
            modern.mean_deviation_deg, improvement_factor
        );

        let historical_context = match request.ancient_device {
            DeviceType::Sinan => "司南是目前有文献记载的最早磁指向装置（战国《韩非子》，东汉王充《论衡》），\
                但由于天然磁铁矿磁性弱、勺底摩擦大，其实用性一直存在学术争议。王振铎先生1952年的复原实验表明，\
                只有使用精选的磁铁矿并打磨至极低摩擦系数，才能在汉代地磁场下获得较可靠的指向。".to_string(),
            DeviceType::Zhinanyu => "指南鱼见于北宋《武经总要》（1044年），是已知最早利用地磁场进行人工磁化的装置。\
                它采用薄铁片淬火剩磁+水浮支承方案，摩擦比司南显著降低，可应用于军事行军辨向。".to_string(),
            DeviceType::HanLuopan => "旱罗盘（磁针+方位盘）约在北宋晚期（11世纪末-12世纪初）出现，\
                南宋《诸蕃志》明确记载了其用于远洋航海。磁针细长的形状各向异性和枢轴低摩擦使其精度远超前代，\
                成为郑和下西洋、地理大发现时代的核心导航技术。".to_string(),
            DeviceType::MemsCompass => "现代MEMS电子罗盘是21世纪微电子技术的产物，\
                广泛应用于智能手机、无人机、VR/AR设备和穿戴设备。配合GPS和IMU可实现全场景无缝导航。".to_string(),
        };

        Ok(CrossEraCompareResponse {
            ancient,
            modern_mems: modern,
            improvement_factor,
            accuracy_gap_deg,
            narrative,
            historical_context,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        CrossEraCompareRequest, DeviceType, InteractiveSinanRequest,
        InterferenceSimulationRequest, InterferenceSource, InterferenceType,
        MultiDeviceCompareRequest,
    };
    use nalgebra::Vector3;
    use std::f64::consts::PI;

    fn make_geomagnetic_field(nT: f64, decl_deg: f64, incl_deg: f64) -> Vector3<f64> {
        let dec = decl_deg * PI / 180.0;
        let inc = incl_deg * PI / 180.0;
        let b = nT * 1e-9;
        Vector3::new(
            b * inc.cos() * dec.cos(),
            b * inc.cos() * dec.sin(),
            b * inc.sin(),
        )
    }

    fn modern_field() -> Vector3<f64> {
        make_geomagnetic_field(52000.0, -5.0, 55.0)
    }

    fn ancient_field() -> Vector3<f64> {
        make_geomagnetic_field(56000.0, -2.0, 58.0)
    }

    fn simulator() -> MicromagneticSimulator {
        MicromagneticSimulator::default()
    }

    // ========== 1. 装置对比测试 ==========

    #[test]
    fn test_compare_devices_normal_sinan_vs_hanluopan() {
        let sim = simulator();
        let req = MultiDeviceCompareRequest {
            target_year: 2024.0,
            location_lat: 39.9,
            location_lon: 116.4,
            devices: vec![DeviceType::Sinan, DeviceType::HanLuopan],
            magnetic_moment_magnitude: None,
            remanence: None,
            temperature: 25.0,
            expected_azimuth: 180.0,
            monte_carlo_samples: None,
        };
        let res = sim.compare_multiple_devices(&req, modern_field()).unwrap();

        assert_eq!(res.devices.len(), 2);
        assert!(!res.summary.is_empty());
        assert!(res.ranking.len() >= 2);

        for d in &res.devices {
            assert!(d.mean_deviation_deg >= 0.0);
            assert!(d.std_deviation_deg >= 0.0);
            assert!(d.p95_deviation_deg >= d.mean_deviation_deg - 1e-6);
            assert!(d.max_deviation_deg >= d.min_deviation_deg);
        }

        let sinan = res.devices.iter().find(|d| matches!(d.device_type, DeviceType::Sinan)).unwrap();
        let luopan = res.devices.iter().find(|d| matches!(d.device_type, DeviceType::HanLuopan)).unwrap();
        assert!(
            luopan.mean_deviation_deg < sinan.mean_deviation_deg,
            "旱罗盘精度应优于司南"
        );
    }

    #[test]
    fn test_compare_devices_boundary_empty_devices_defaults_to_all() {
        let sim = simulator();
        let req = MultiDeviceCompareRequest {
            target_year: 2024.0,
            location_lat: 30.0,
            location_lon: 120.0,
            devices: vec![],
            magnetic_moment_magnitude: None,
            remanence: None,
            temperature: 20.0,
            expected_azimuth: 0.0,
            monte_carlo_samples: None,
        };
        let res = sim.compare_multiple_devices(&req, modern_field()).unwrap();
        assert_eq!(res.devices.len(), 4, "空列表默认为全部4种装置");
        assert_eq!(res.ranking.len(), 4);
    }

    #[test]
    fn test_compare_devices_boundary_extreme_years() {
        let sim = simulator();
        for y in [-1000.0f64, 1088.0, 1405.0, 1900.0, 2100.0] {
            let req = MultiDeviceCompareRequest {
                target_year: y,
                location_lat: 35.0,
                location_lon: 110.0,
                devices: vec![DeviceType::Sinan, DeviceType::Zhinanyu],
                magnetic_moment_magnitude: None,
                remanence: None,
                temperature: 15.0,
                expected_azimuth: 90.0,
                monte_carlo_samples: None,
            };
            let res = sim.compare_multiple_devices(&req, ancient_field()).unwrap();
            assert_eq!(res.devices.len(), 2);
            assert!(res.geomagnetic_intensity_nT > 1000.0);
        }
    }

    #[test]
    fn test_compare_devices_extreme_temperature() {
        let sim = simulator();
        for temp in [-40.0, 0.0, 60.0, 85.0] {
            let req = MultiDeviceCompareRequest {
                target_year: 2024.0,
                location_lat: 39.9,
                location_lon: 116.4,
                devices: vec![DeviceType::MemsCompass, DeviceType::HanLuopan],
                magnetic_moment_magnitude: None,
                remanence: None,
                temperature: temp,
                expected_azimuth: 270.0,
                monte_carlo_samples: None,
            };
            let res = sim.compare_multiple_devices(&req, modern_field()).unwrap();
            assert_eq!(res.devices.len(), 2);
        }
    }

    #[test]
    fn test_compare_devices_ranking_structure() {
        let sim = simulator();
        let req = MultiDeviceCompareRequest {
            target_year: 2024.0,
            location_lat: 40.0,
            location_lon: 116.0,
            devices: vec![
                DeviceType::Sinan,
                DeviceType::Zhinanyu,
                DeviceType::HanLuopan,
                DeviceType::MemsCompass,
            ],
            magnetic_moment_magnitude: None,
            remanence: None,
            temperature: 25.0,
            expected_azimuth: 45.0,
            monte_carlo_samples: None,
        };
        let res = sim.compare_multiple_devices(&req, modern_field()).unwrap();
        assert_eq!(res.ranking.len(), 4, "排名应包含全部4种装置");
        let types_in_ranking: std::collections::HashSet<DeviceType> = res.ranking.into_iter().collect();
        assert_eq!(types_in_ranking.len(), 4, "排名中每种装置恰好出现一次");
        assert!(types_in_ranking.contains(&DeviceType::Sinan));
        assert!(types_in_ranking.contains(&DeviceType::Zhinanyu));
        assert!(types_in_ranking.contains(&DeviceType::HanLuopan));
        assert!(types_in_ranking.contains(&DeviceType::MemsCompass));
    }

    // ========== 2. 跨时代对比测试 ==========

    #[test]
    fn test_cross_era_sinan_vs_mems_normal() {
        let sim = simulator();
        let req = CrossEraCompareRequest {
            location_lat: 34.26,
            location_lon: 108.95,
            ancient_year: 139.0,
            modern_year: 2024.0,
            ancient_device: DeviceType::Sinan,
            temperature: 20.0,
            expected_azimuth: 180.0,
        };
        let res = sim.compare_cross_era(&req, ancient_field(), modern_field()).unwrap();

        assert!(matches!(res.ancient.device_type, DeviceType::Sinan));
        assert!(matches!(res.modern_mems.device_type, DeviceType::MemsCompass));
        assert!(res.improvement_factor > 1.0);
        assert!(res.accuracy_gap_deg > 0.0);
        assert!(!res.narrative.is_empty());
        assert!(!res.historical_context.is_empty());
    }

    #[test]
    fn test_cross_era_zhinanyu_vs_mems() {
        let sim = simulator();
        let req = CrossEraCompareRequest {
            location_lat: 34.0,
            location_lon: 114.0,
            ancient_year: 1044.0,
            modern_year: 2024.0,
            ancient_device: DeviceType::Zhinanyu,
            temperature: 25.0,
            expected_azimuth: 0.0,
        };
        let res = sim.compare_cross_era(&req, ancient_field(), modern_field()).unwrap();
        assert!(matches!(res.ancient.device_type, DeviceType::Zhinanyu));
        assert!(res.improvement_factor.is_finite(), "改善因子应为有限数");
        assert!(!res.narrative.is_empty());
        assert!(res.historical_context.contains("武经总要"));
    }

    #[test]
    fn test_cross_era_hanluopan_vs_mems() {
        let sim = simulator();
        let req = CrossEraCompareRequest {
            location_lat: 32.0,
            location_lon: 118.0,
            ancient_year: 1405.0,
            modern_year: 2024.0,
            ancient_device: DeviceType::HanLuopan,
            temperature: 22.0,
            expected_azimuth: 90.0,
        };
        let res = sim.compare_cross_era(&req, ancient_field(), modern_field()).unwrap();
        assert!(matches!(res.ancient.device_type, DeviceType::HanLuopan));
        assert!(res.improvement_factor > 0.0, "改善因子应为正");
    }

    #[test]
    fn test_cross_era_boundary_same_field_both_eras() {
        let sim = simulator();
        let field = modern_field();
        let req = CrossEraCompareRequest {
            location_lat: 30.0,
            location_lon: 120.0,
            ancient_year: 2024.0,
            modern_year: 2024.0,
            ancient_device: DeviceType::HanLuopan,
            temperature: 25.0,
            expected_azimuth: 135.0,
        };
        let res = sim.compare_cross_era(&req, field, field).unwrap();
        assert!(res.improvement_factor > 1.0, "同地磁场MEMS仍应更优");
    }

    #[test]
    fn test_cross_era_boundary_extreme_location() {
        let sim = simulator();
        let cases: Vec<(&'static str, f64, f64)> = vec![
            ("北极附近", 85.0, 30.0),
            ("赤道附近", 0.0, 120.0),
            ("南极附近", -80.0, -60.0),
            ("中国漠河", 53.48, 122.37),
            ("三亚", 18.25, 109.51),
        ];
        for (name, lat, lon) in cases {
            let req = CrossEraCompareRequest {
                location_lat: lat,
                location_lon: lon,
                ancient_year: 1000.0,
                modern_year: 2024.0,
                ancient_device: DeviceType::Sinan,
                temperature: 10.0,
                expected_azimuth: 225.0,
            };
            let f = make_geomagnetic_field(
                if lat.abs() > 60.0 { 60000.0 } else { 45000.0 },
                0.0,
                if lat > 0.0 { 65.0 } else { -65.0 },
            );
            let res = sim.compare_cross_era(&req, f, modern_field())
                .unwrap_or_else(|e| panic!("地点{}失败: {:?}", name, e));
            assert!(res.improvement_factor > 0.0, "{}: 改善因子应为正", name);
        }
    }

    // ========== 3. 干扰模拟测试 ==========

    #[test]
    fn test_interference_boundary_empty_sources_safe() {
        let sim = simulator();
        let req = InterferenceSimulationRequest {
            device_type: DeviceType::Sinan,
            target_year: 2024.0,
            location_lat: 39.9,
            location_lon: 116.4,
            temperature: 25.0,
            expected_azimuth: 180.0,
            magnetic_moment_magnitude: None,
            remanence: None,
            interference_sources: vec![],
        };
        let res = sim.simulate_interference(&req, modern_field()).unwrap();
        assert_eq!(res.warning_level, "安全");
        assert!(res.interference_ratio < 0.05);
        assert!(res.total_interference_field_nT < 10.0);
        assert!(res.effects.is_empty());
    }

    #[test]
    fn test_interference_normal_single_ferrous_object() {
        let sim = simulator();
        let req = InterferenceSimulationRequest {
            device_type: DeviceType::Sinan,
            target_year: 2024.0,
            location_lat: 39.9,
            location_lon: 116.4,
            temperature: 25.0,
            expected_azimuth: 180.0,
            magnetic_moment_magnitude: None,
            remanence: None,
            interference_sources: vec![InterferenceSource {
                interference_type: InterferenceType::FerrousObject,
                distance_m: 0.3,
                intensity_factor: 1.0,
                azimuth_deg: 90.0,
                description: Some("铁制剪刀".to_string()),
            }],
        };
        let res = sim.simulate_interference(&req, modern_field()).unwrap();
        assert_eq!(res.effects.len(), 1);
        let eff = &res.effects[0];
        assert!(matches!(eff.interference_type, InterferenceType::FerrousObject));
        assert!(eff.induced_field_nT > 0.0);
        assert!(res.interference_ratio > 0.0);
    }

    #[test]
    fn test_interference_boundary_close_loudspeaker_severe() {
        let sim = simulator();
        let req = InterferenceSimulationRequest {
            device_type: DeviceType::HanLuopan,
            target_year: 2024.0,
            location_lat: 40.0,
            location_lon: 116.0,
            temperature: 25.0,
            expected_azimuth: 0.0,
            magnetic_moment_magnitude: None,
            remanence: None,
            interference_sources: vec![InterferenceSource {
                interference_type: InterferenceType::Loudspeaker,
                distance_m: 0.05,
                intensity_factor: 1.5,
                azimuth_deg: 180.0,
                description: Some("紧贴喇叭磁铁".to_string()),
            }],
        };
        let res = sim.simulate_interference(&req, modern_field()).unwrap();
        assert!(
            res.interference_ratio > 0.5,
            "近距离强干扰ratio>0.5, 实际{}",
            res.interference_ratio
        );
        assert_eq!(res.warning_level, "严重干扰");
    }

    #[test]
    fn test_interference_boundary_very_far_safe() {
        let sim = simulator();
        let req = InterferenceSimulationRequest {
            device_type: DeviceType::Sinan,
            target_year: 2024.0,
            location_lat: 40.0,
            location_lon: 116.0,
            temperature: 25.0,
            expected_azimuth: 45.0,
            magnetic_moment_magnitude: None,
            remanence: None,
            interference_sources: vec![
                InterferenceSource {
                    interference_type: InterferenceType::PowerLine,
                    distance_m: 100.0,
                    intensity_factor: 1.0,
                    azimuth_deg: 0.0,
                    description: None,
                },
                InterferenceSource {
                    interference_type: InterferenceType::BuildingRebar,
                    distance_m: 20.0,
                    intensity_factor: 0.5,
                    azimuth_deg: 90.0,
                    description: None,
                },
            ],
        };
        let res = sim.simulate_interference(&req, modern_field()).unwrap();
        assert!(res.interference_ratio < 0.2, "远距离干扰<20%, 实际{}", res.interference_ratio);
        assert_ne!(res.warning_level, "严重干扰");
    }

    #[test]
    fn test_interference_multiple_sources_stack() {
        let sim = simulator();
        let build_req = |sources: Vec<InterferenceSource>| InterferenceSimulationRequest {
            device_type: DeviceType::Sinan,
            target_year: 2024.0,
            location_lat: 40.0,
            location_lon: 116.0,
            temperature: 25.0,
            expected_azimuth: 90.0,
            magnetic_moment_magnitude: None,
            remanence: None,
            interference_sources: sources,
        };

        let single = InterferenceSource {
            interference_type: InterferenceType::ElectronicDevice,
            distance_m: 0.2,
            intensity_factor: 1.0,
            azimuth_deg: 90.0,
            description: None,
        };
        let res_single = sim.simulate_interference(&build_req(vec![single.clone()]), modern_field()).unwrap();

        let multi = sim.simulate_interference(
            &build_req(vec![
                single,
                InterferenceSource {
                    interference_type: InterferenceType::Loudspeaker,
                    distance_m: 0.5,
                    intensity_factor: 1.0,
                    azimuth_deg: 180.0,
                    description: None,
                },
                InterferenceSource {
                    interference_type: InterferenceType::FerrousObject,
                    distance_m: 0.4,
                    intensity_factor: 1.0,
                    azimuth_deg: 0.0,
                    description: None,
                },
            ]),
            modern_field(),
        ).unwrap();

        assert!(multi.total_interference_field_nT >= res_single.total_interference_field_nT);
        assert_eq!(multi.effects.len(), 3);
    }

    #[test]
    fn test_interference_all_six_types() {
        let sim = simulator();
        let types = vec![
            InterferenceType::FerrousObject,
            InterferenceType::PowerLine,
            InterferenceType::ElectronicDevice,
            InterferenceType::BuildingRebar,
            InterferenceType::Loudspeaker,
            InterferenceType::LightningStorm,
        ];
        for t in types {
            let req = InterferenceSimulationRequest {
                device_type: DeviceType::HanLuopan,
                target_year: 2024.0,
                location_lat: 30.0,
                location_lon: 120.0,
                temperature: 22.0,
                expected_azimuth: 270.0,
                magnetic_moment_magnitude: None,
                remanence: None,
                interference_sources: vec![InterferenceSource {
                    interference_type: t.clone(),
                    distance_m: 1.0,
                    intensity_factor: 1.0,
                    azimuth_deg: 0.0,
                    description: None,
                }],
            };
            let res = sim.simulate_interference(&req, modern_field()).unwrap();
            assert_eq!(res.effects.len(), 1);
        }
    }

    // ========== 4. 虚拟体验交互测试 ==========

    #[test]
    fn test_interactive_normal_sinan_defaults() {
        let sim = simulator();
        let req = InteractiveSinanRequest {
            device_type: DeviceType::Sinan,
            target_year: 2024.0,
            location_lat: 39.9,
            location_lon: 116.4,
            magnetic_moment_magnitude: 0.5,
            remanence: 80000.0,
            temperature: 25.0,
            friction_coefficient: 0.05,
            anisotropy_constant: 10000.0,
            demagnetization_factor_override: None,
            spoon_length_m: None,
            spoon_width_m: None,
            spoon_thickness_m: None,
            expected_azimuth: 180.0,
        };
        let res = sim.simulate_interactive(&req, modern_field()).unwrap();

        assert!(matches!(res.device_type, DeviceType::Sinan));
        assert!(res.geomagnetic_intensity_nT > 10000.0);
        assert!(res.effective_moment_magnitude > 0.0);
        assert!(res.torque_magnitude >= 0.0);
        assert!(res.pointing_accuracy_deg >= 0.0);
        assert!(!res.physics_insights.is_empty(), "应含物理洞察教育信息");
        assert!(res.demagnetization_tensor.contains_key("n_xx"));
        assert_eq!(res.spoon_dimensions_m.len(), 3);
    }

    #[test]
    fn test_interactive_device_switching_all_four() {
        let sim = simulator();
        for dt in vec![
            DeviceType::Sinan,
            DeviceType::Zhinanyu,
            DeviceType::HanLuopan,
            DeviceType::MemsCompass,
        ] {
            let req = InteractiveSinanRequest {
                device_type: dt.clone(),
                target_year: 2024.0,
                location_lat: 35.0,
                location_lon: 115.0,
                magnetic_moment_magnitude: 0.5,
                remanence: 60000.0,
                temperature: 20.0,
                friction_coefficient: 0.03,
                anisotropy_constant: 12000.0,
                demagnetization_factor_override: None,
                spoon_length_m: None,
                spoon_width_m: None,
                spoon_thickness_m: None,
                expected_azimuth: 90.0,
            };
            let res = sim.simulate_interactive(&req, modern_field())
                .unwrap_or_else(|e| panic!("装置{:?}失败: {:?}", dt, e));
            assert!(!res.physics_insights.is_empty(), "{:?}应含洞察", dt);

            if matches!(dt, DeviceType::MemsCompass) {
                assert_eq!(res.torque_magnitude, 0.0, "MEMS无永磁体力矩");
            } else {
                assert!(res.effective_moment_magnitude > 0.0, "{:?}应有有效磁矩", dt);
            }
        }
    }

    #[test]
    fn test_interactive_boundary_custom_dimensions() {
        let sim = simulator();
        let req = InteractiveSinanRequest {
            device_type: DeviceType::Sinan,
            target_year: 2024.0,
            location_lat: 35.0,
            location_lon: 115.0,
            magnetic_moment_magnitude: 0.5,
            remanence: 60000.0,
            temperature: 20.0,
            friction_coefficient: 0.05,
            anisotropy_constant: 10000.0,
            demagnetization_factor_override: None,
            spoon_length_m: Some(0.20),
            spoon_width_m: Some(0.12),
            spoon_thickness_m: Some(0.02),
            expected_azimuth: 0.0,
        };
        let res = sim.simulate_interactive(&req, modern_field()).unwrap();
        assert!((res.spoon_dimensions_m[0] - 0.20).abs() < 1e-9);
        assert!((res.spoon_dimensions_m[1] - 0.12).abs() < 1e-9);
        assert!((res.spoon_dimensions_m[2] - 0.02).abs() < 1e-9);
    }

    #[test]
    fn test_interactive_boundary_extreme_magnetic_params() {
        let sim = simulator();
        for (mm, rm, name) in vec![
            (0.001, 1000.0, "极弱"),
            (2.0, 200000.0, "极强"),
            (0.5, 60000.0, "正常"),
        ] {
            let req = InteractiveSinanRequest {
                device_type: DeviceType::Sinan,
                target_year: 2024.0,
                location_lat: 35.0,
                location_lon: 115.0,
                magnetic_moment_magnitude: mm,
                remanence: rm,
                temperature: 25.0,
                friction_coefficient: 0.05,
                anisotropy_constant: 10000.0,
                demagnetization_factor_override: Some(0.1),
                spoon_length_m: None,
                spoon_width_m: None,
                spoon_thickness_m: None,
                expected_azimuth: 135.0,
            };
            let res = sim.simulate_interactive(&req, modern_field())
                .unwrap_or_else(|e| panic!("{}失败: {:?}", name, e));
            assert!(res.effective_moment_magnitude >= 0.0, "{}: 有效磁矩非负", name);
        }
    }

    #[test]
    fn test_interactive_boundary_extreme_friction() {
        let sim = simulator();
        let mut last = None;
        for friction in [0.001, 0.05, 0.2, 0.5, 1.0] {
            let req = InteractiveSinanRequest {
                device_type: DeviceType::Sinan,
                target_year: 2024.0,
                location_lat: 35.0,
                location_lon: 115.0,
                magnetic_moment_magnitude: 0.5,
                remanence: 80000.0,
                temperature: 25.0,
                friction_coefficient: friction,
                anisotropy_constant: 10000.0,
                demagnetization_factor_override: None,
                spoon_length_m: None,
                spoon_width_m: None,
                spoon_thickness_m: None,
                expected_azimuth: 45.0,
            };
            let res = sim.simulate_interactive(&req, modern_field()).unwrap();
            if let Some(l) = last {
                assert!(
                    res.pointing_accuracy_deg >= l - 5.0,
                    "摩擦{}精度{:.2} vs 上次{:.2}",
                    friction,
                    res.pointing_accuracy_deg,
                    l
                );
            }
            last = Some(res.pointing_accuracy_deg);
        }
    }

    #[test]
    fn test_interactive_boundary_extreme_dimensions_valid() {
        let sim = simulator();
        for (name, l, w, t) in vec![
            ("极小", 0.02, 0.01, 0.001),
            ("极大", 2.0, 1.0, 0.1),
            ("极薄", 0.3, 0.2, 0.0001),
            ("正常", 0.17, 0.11, 0.015),
        ] {
            let req = InteractiveSinanRequest {
                device_type: DeviceType::Sinan,
                target_year: 2024.0,
                location_lat: 35.0,
                location_lon: 115.0,
                magnetic_moment_magnitude: 0.5,
                remanence: 80000.0,
                temperature: 25.0,
                friction_coefficient: 0.05,
                anisotropy_constant: 10000.0,
                demagnetization_factor_override: None,
                spoon_length_m: Some(l),
                spoon_width_m: Some(w),
                spoon_thickness_m: Some(t),
                expected_azimuth: 180.0,
            };
            let res = sim.simulate_interactive(&req, modern_field())
                .unwrap_or_else(|e| panic!("{}失败: {:?}", name, e));
            for key in ["n_xx", "n_yy", "n_zz"].iter() {
                let v = res.demagnetization_tensor[*key];
                assert!(v.is_finite(), "{}: {}应为有限数, 实际{}", name, key, v);
            }
        }
    }

    #[test]
    fn test_interactive_educational_insights() {
        let sim = simulator();
        let req = InteractiveSinanRequest {
            device_type: DeviceType::Zhinanyu,
            target_year: 1044.0,
            location_lat: 34.0,
            location_lon: 114.0,
            magnetic_moment_magnitude: 0.3,
            remanence: 50000.0,
            temperature: 15.0,
            friction_coefficient: 0.01,
            anisotropy_constant: 8000.0,
            demagnetization_factor_override: None,
            spoon_length_m: None,
            spoon_width_m: None,
            spoon_thickness_m: None,
            expected_azimuth: 270.0,
        };
        let res = sim.simulate_interactive(&req, ancient_field()).unwrap();
        assert!(res.physics_insights.len() >= 3, "至少3条教育性洞察");
        for i in &res.physics_insights {
            assert!(!i.is_empty());
            assert!(i.chars().count() >= 5);
        }
    }

    // ========== 辅助：退磁张量数学性质 ==========

    #[test]
    fn test_demagnetization_tensor_sum_approx_one() {
        let cases = vec![
            ("司南", DemagnetizationTensor::for_spoon_shape(0.17, 0.11, 0.015)),
            ("鱼形", DemagnetizationTensor::for_fish_shape(0.10, 0.025, 0.001)),
            ("针形", DemagnetizationTensor::for_needle_shape(0.04, 0.002, 0.0005)),
            ("MEMS", DemagnetizationTensor::for_mems_chip(0.002)),
        ];
        for (name, t) in cases {
            let sum = t.n_xx + t.n_yy + t.n_zz;
            assert!(
                (sum - 1.0).abs() < 0.5,
                "{}: 退磁张量主对角和大致合理, 实际{}",
                name, sum
            );
            assert!(t.n_xx.is_finite(), "{}: n_xx有限", name);
            assert!(t.n_yy.is_finite(), "{}: n_yy有限", name);
            assert!(t.n_zz.is_finite(), "{}: n_zz有限", name);
        }
    }
}
