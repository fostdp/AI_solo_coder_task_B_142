use std::sync::Arc;

use nalgebra::Vector3;

use crate::compute_pool::{ComputeFuture, ComputePool};
use crate::errors::Result;
use crate::micromagnetic_simulation::MicromagneticSimulator;
use crate::models::{
    DragForceRequest, DragForceResponse, ForceFeedbackHint, InteractiveSinanRequest,
    InteractiveSinanResponse,
};

#[derive(Clone)]
pub struct VRSinan {
    simulator: Arc<MicromagneticSimulator>,
    pool: &'static ComputePool,
}

impl VRSinan {
    pub fn new(simulator: Arc<MicromagneticSimulator>) -> Self {
        Self::with_pool(simulator, ComputePool::global())
    }

    pub fn with_pool(
        simulator: Arc<MicromagneticSimulator>,
        pool: &'static ComputePool,
    ) -> Self {
        Self { simulator, pool }
    }

    pub fn simulator(&self) -> &MicromagneticSimulator {
        &self.simulator
    }

    pub fn simulate_interactive_sync(
        &self,
        request: &InteractiveSinanRequest,
        geomagnetic_field: Vector3<f64>,
    ) -> Result<InteractiveSinanResponse> {
        self.simulator.simulate_interactive(request, geomagnetic_field)
    }

    pub fn simulate_interactive_async(
        &self,
        request: InteractiveSinanRequest,
        geomagnetic_field: Vector3<f64>,
    ) -> ComputeFuture<Result<InteractiveSinanResponse>> {
        let sim = self.simulator.clone();
        self.pool
            .spawn_compute("vr_sinan_interactive", move || {
                sim.simulate_interactive(&request, geomagnetic_field)
            })
    }

    pub fn simulate_drag_force_sync(
        &self,
        request: &DragForceRequest,
        geomagnetic_field: Vector3<f64>,
    ) -> Result<DragForceResponse> {
        self.simulator.simulate_drag_force(request, geomagnetic_field)
    }

    pub fn simulate_drag_force_async(
        &self,
        request: DragForceRequest,
        geomagnetic_field: Vector3<f64>,
    ) -> ComputeFuture<Result<DragForceResponse>> {
        let sim = self.simulator.clone();
        self.pool.spawn_compute("vr_sinan_drag", move || {
            sim.simulate_drag_force(&request, geomagnetic_field)
        })
    }

    pub fn force_feedback_hint_sync(
        &self,
        device_type: crate::models::DeviceType,
        geomagnetic_field: Vector3<f64>,
        expected_azimuth: f64,
    ) -> Option<ForceFeedbackHint> {
        self.simulator
            .force_feedback_hint(device_type, geomagnetic_field, expected_azimuth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::micromagnetic_simulation::MicromagneticSimulator;
    use crate::models::{DeviceGeometryParams, DeviceType, DragForceRequest, InteractiveSinanRequest};
    use std::sync::Arc;

    fn make_field() -> Vector3<f64> {
        Vector3::new(30000.0e-9, 0.0, 45000.0e-9)
    }

    #[test]
    fn test_vr_interactive_sync_sinan_defaults() {
        let sim = Arc::new(MicromagneticSimulator::new());
        let vr = VRSinan::new(sim);
        let req = InteractiveSinanRequest {
            device_type: DeviceType::Sinan,
            target_year: 2024.0,
            location_lat: 39.9,
            location_lon: 116.4,
            magnetic_moment_magnitude: 0.3,
            remanence: 90000.0,
            temperature: 25.0,
            friction_coefficient: 0.18,
            anisotropy_constant: 10000.0,
            demagnetization_factor_override: None,
            spoon_length_m: None,
            spoon_width_m: None,
            spoon_thickness_m: None,
            expected_azimuth: 180.0,
        };
        let res = vr.simulate_interactive_sync(&req, make_field()).unwrap();
        assert!(matches!(res.device_type, DeviceType::Sinan));
        assert!(res.simulated_azimuth.is_finite());
        assert!(res.physics_insights.len() >= 3);
        assert!(res.spoon_dimensions_m[0] > 0.0);
        assert!(res.force_feedback_hint.is_some());
        let hint = res.force_feedback_hint.as_ref().unwrap();
        assert!(hint.magnetic_stiffness_n_m_rad > 0.0);
        assert!(hint.estimated_settling_time_s > 0.0);
        assert!(hint.haptic_intensity_0_1 >= 0.0 && hint.haptic_intensity_0_1 <= 1.0);
    }

    #[test]
    fn test_vr_interactive_sync_mems_no_feedback() {
        let sim = Arc::new(MicromagneticSimulator::new());
        let vr = VRSinan::new(sim);
        let req = InteractiveSinanRequest {
            device_type: DeviceType::MemsCompass,
            target_year: 2024.0,
            location_lat: 39.9,
            location_lon: 116.4,
            magnetic_moment_magnitude: 0.0,
            remanence: 0.0,
            temperature: 25.0,
            friction_coefficient: 0.0,
            anisotropy_constant: 0.0,
            demagnetization_factor_override: None,
            spoon_length_m: None,
            spoon_width_m: None,
            spoon_thickness_m: None,
            expected_azimuth: 180.0,
        };
        let res = vr.simulate_interactive_sync(&req, make_field()).unwrap();
        assert!(matches!(res.device_type, DeviceType::MemsCompass));
        assert!(res.force_feedback_hint.is_none());
    }

    #[test]
    fn test_vr_drag_force_sync_off_axis_is_resisted() {
        let sim = Arc::new(MicromagneticSimulator::new());
        let vr = VRSinan::new(sim);
        let req = DragForceRequest {
            device_type: DeviceType::Sinan,
            target_year: 2024.0,
            location_lat: 39.9,
            location_lon: 116.4,
            magnetic_moment_magnitude: 0.3,
            remanence: 90000.0,
            spoon_length_m: None,
            spoon_width_m: None,
            spoon_thickness_m: None,
            drag_azimuth_deg: 90.0,
            drag_speed_rad_s: 0.5,
            pivot_friction_coefficient: 0.18,
            expected_azimuth: 180.0,
            dt_seconds: 0.01,
        };
        let res = vr.simulate_drag_force_sync(&req, make_field()).unwrap();
        assert!(res.restoring_torque_n_m.is_finite());
        assert!(res.haptic_intensity_0_1 >= 0.0 && res.haptic_intensity_0_1 <= 1.0);
        assert!(res.moment_of_inertia_kg_m2 > 0.0);
        assert!(res.next_azimuth_deg.is_finite());
        assert!(!res.force_description.is_empty());
        assert!(!res.educational_note.is_empty());
    }

    #[test]
    fn test_vr_drag_force_mems_zero_stiffness() {
        let sim = Arc::new(MicromagneticSimulator::new());
        let vr = VRSinan::new(sim);
        let req = DragForceRequest {
            device_type: DeviceType::MemsCompass,
            target_year: 2024.0,
            location_lat: 39.9,
            location_lon: 116.4,
            magnetic_moment_magnitude: 0.0,
            remanence: 0.0,
            spoon_length_m: None,
            spoon_width_m: None,
            spoon_thickness_m: None,
            drag_azimuth_deg: 90.0,
            drag_speed_rad_s: 0.5,
            pivot_friction_coefficient: 0.0,
            expected_azimuth: 180.0,
            dt_seconds: 0.01,
        };
        let res = vr.simulate_drag_force_sync(&req, make_field()).unwrap();
        assert_eq!(res.restoring_torque_n_m, 0.0);
        assert!(res.force_description.contains("MEMS"));
    }

    #[test]
    fn test_vr_feedback_hint_sinan_vs_luopan() {
        let sim = Arc::new(MicromagneticSimulator::new());
        let vr = VRSinan::new(sim);
        let hint_sinan = vr
            .force_feedback_hint_sync(DeviceType::Sinan, make_field(), 180.0)
            .unwrap();
        let geom_lp = DeviceGeometryParams::for_type(DeviceType::HanLuopan);
        let hint_lp = vr
            .force_feedback_hint_sync(DeviceType::HanLuopan, make_field(), 180.0)
            .unwrap();
        assert!(hint_sinan.estimated_settling_time_s > hint_lp.estimated_settling_time_s
            || geom_lp.pivot_friction < 0.1);
        assert!(hint_sinan.magnetic_stiffness_n_m_rad.is_finite());
        assert!(hint_lp.magnetic_stiffness_n_m_rad.is_finite());
    }
}
