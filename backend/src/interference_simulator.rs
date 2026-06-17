use std::sync::Arc;

use nalgebra::Vector3;

use crate::compute_pool::{ComputeFuture, ComputePool};
use crate::errors::Result;
use crate::micromagnetic_simulation::MicromagneticSimulator;
use crate::models::{
    InterferenceSimulationRequest, InterferenceSimulationResponse, InterferenceType,
};

#[derive(Clone)]
pub struct InterferenceSimulator {
    simulator: Arc<MicromagneticSimulator>,
    pool: &'static ComputePool,
}

impl InterferenceSimulator {
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

    pub fn simulate_sync(
        &self,
        request: &InterferenceSimulationRequest,
        geomagnetic_field: Vector3<f64>,
    ) -> Result<InterferenceSimulationResponse> {
        self.simulator
            .simulate_interference(request, geomagnetic_field)
    }

    pub fn simulate_async(
        &self,
        request: InterferenceSimulationRequest,
        geomagnetic_field: Vector3<f64>,
    ) -> ComputeFuture<Result<InterferenceSimulationResponse>> {
        let sim = self.simulator.clone();
        self.pool.spawn_compute("interference_sim", move || {
            sim.simulate_interference(&request, geomagnetic_field)
        })
    }

    pub fn list_interference_types() -> Vec<InterferenceType> {
        InterferenceType::all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::micromagnetic_simulation::MicromagneticSimulator;
    use crate::models::{DeviceType, InterferenceSource, InterferenceType};
    use std::sync::Arc;

    fn make_field() -> Vector3<f64> {
        Vector3::new(30000.0e-9, 0.0, 45000.0e-9)
    }

    #[test]
    fn test_interference_sync_empty_sources_is_safe() {
        let sim = Arc::new(MicromagneticSimulator::new());
        let si = InterferenceSimulator::new(sim);
        let req = InterferenceSimulationRequest {
            device_type: DeviceType::Sinan,
            target_year: 2024.0,
            location_lat: 39.9,
            location_lon: 116.4,
            interference_sources: vec![],
            magnetic_moment_magnitude: None,
            remanence: None,
            temperature: 25.0,
            expected_azimuth: 180.0,
        };
        let res = si.simulate_sync(&req, make_field()).unwrap();
        assert_eq!(res.warning_level, "安全");
        assert_eq!(res.effects.len(), 0);
        assert!(res.baseline_accuracy_deg.is_finite());
        assert!(res.total_interference_field_nT < 0.001 || res.total_interference_field_nT == 0.0);
    }

    #[test]
    fn test_interference_sync_single_ferrous() {
        let sim = Arc::new(MicromagneticSimulator::new());
        let si = InterferenceSimulator::new(sim);
        let req = InterferenceSimulationRequest {
            device_type: DeviceType::Sinan,
            target_year: 2024.0,
            location_lat: 39.9,
            location_lon: 116.4,
            interference_sources: vec![InterferenceSource {
                interference_type: InterferenceType::FerrousObject,
                distance_m: 2.0,
                intensity_factor: 1.0,
                azimuth_deg: 90.0,
                description: None,
            }],
            magnetic_moment_magnitude: Some(0.3),
            remanence: Some(90000.0),
            temperature: 25.0,
            expected_azimuth: 180.0,
        };
        let res = si.simulate_sync(&req, make_field()).unwrap();
        assert_eq!(res.effects.len(), 1);
        assert!(res.effects[0].induced_field_nT > 0.0);
        assert!(res.effects[0].base_field_at_1m_nt > 0.0);
        assert!(res.effects[0].data_source.contains("WHO") && res.effects[0].measurement_unit.contains("nT"));
        assert!(res.interference_ratio.is_finite() && res.interference_ratio >= 0.0);
    }

    #[test]
    fn test_interference_sync_loudspeaker_close_strong_is_severe() {
        let sim = Arc::new(MicromagneticSimulator::new());
        let si = InterferenceSimulator::new(sim);
        let req = InterferenceSimulationRequest {
            device_type: DeviceType::HanLuopan,
            target_year: 2024.0,
            location_lat: 39.9,
            location_lon: 116.4,
            interference_sources: vec![InterferenceSource {
                interference_type: InterferenceType::Loudspeaker,
                distance_m: 0.05,
                intensity_factor: 1.5,
                azimuth_deg: 45.0,
                description: None,
            }],
            magnetic_moment_magnitude: Some(0.3),
            remanence: Some(90000.0),
            temperature: 25.0,
            expected_azimuth: 180.0,
        };
        let res = si.simulate_sync(&req, make_field()).unwrap();
        assert_eq!(res.warning_level, "严重干扰");
        assert!(res.effects[0].induced_field_nT > 1000.0);
        assert!(res.interference_ratio > 0.01);
    }

    #[test]
    fn test_list_interference_types_has_six() {
        let types = InterferenceSimulator::list_interference_types();
        assert_eq!(types.len(), 6);
        for t in &types {
            assert!(!t.display_name().is_empty());
            assert!(t.base_field_at_1m_nt() > 0.0);
            assert!(!t.data_source().is_empty());
        }
    }
}
