use std::sync::Arc;

use nalgebra::Vector3;

use crate::compute_pool::{ComputeFuture, ComputePool};
use crate::errors::Result;
use crate::micromagnetic_simulation::MicromagneticSimulator;
use crate::models::{MultiDeviceCompareRequest, MultiDeviceCompareResponse};

#[derive(Clone)]
pub struct DeviceComparator {
    simulator: Arc<MicromagneticSimulator>,
    pool: &'static ComputePool,
}

impl DeviceComparator {
    pub fn new(simulator: Arc<MicromagneticSimulator>) -> Self {
        Self::with_pool(simulator, ComputePool::global())
    }

    pub fn with_pool(simulator: Arc<MicromagneticSimulator>, pool: &'static ComputePool) -> Self {
        Self { simulator, pool }
    }

    pub fn simulator(&self) -> &MicromagneticSimulator {
        &self.simulator
    }

    pub fn compare_sync(
        &self,
        request: &MultiDeviceCompareRequest,
        geomagnetic_field: Vector3<f64>,
    ) -> Result<MultiDeviceCompareResponse> {
        self.simulator
            .compare_multiple_devices(request, geomagnetic_field)
    }

    pub fn compare_async(
        &self,
        request: MultiDeviceCompareRequest,
        geomagnetic_field: Vector3<f64>,
    ) -> ComputeFuture<Result<MultiDeviceCompareResponse>> {
        let sim = self.simulator.clone();
        self.pool.spawn_compute("device_compare", move || {
            sim.compare_multiple_devices(&request, geomagnetic_field)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DeviceType;
    use crate::micromagnetic_simulation::MicromagneticSimulator;
    use std::sync::Arc;

    fn make_field() -> Vector3<f64> {
        Vector3::new(30000.0e-9, 0.0, 45000.0e-9)
    }

    #[test]
    fn test_comparator_compare_sync_two_devices() {
        let sim = Arc::new(MicromagneticSimulator::new());
        let comp = DeviceComparator::new(sim);
        let req = MultiDeviceCompareRequest {
            target_year: 2024.0,
            location_lat: 39.9,
            location_lon: 116.4,
            devices: vec![DeviceType::Sinan, DeviceType::HanLuopan],
            magnetic_moment_magnitude: Some(0.3),
            remanence: Some(90000.0),
            temperature: 25.0,
            expected_azimuth: 180.0,
            monte_carlo_samples: None,
        };
        let res = comp.compare_sync(&req, make_field()).unwrap();
        assert_eq!(res.devices.len(), 2);
        assert_eq!(res.ranking.len(), 2);
        assert!(res.summary.contains("精度排名"));
        for d in &res.devices {
            assert!(d.mean_deviation_deg.is_finite() && d.mean_deviation_deg >= 0.0);
        }
        assert!(res.geomagnetic_intensity_nT > 50000.0);
    }

    #[test]
    fn test_comparator_compare_sync_empty_devices_defaults_to_all() {
        let sim = Arc::new(MicromagneticSimulator::new());
        let comp = DeviceComparator::new(sim);
        let req = MultiDeviceCompareRequest {
            target_year: 2024.0,
            location_lat: 39.9,
            location_lon: 116.4,
            devices: vec![],
            magnetic_moment_magnitude: Some(0.3),
            remanence: Some(90000.0),
            temperature: 25.0,
            expected_azimuth: 180.0,
            monte_carlo_samples: None,
        };
        let res = comp.compare_sync(&req, make_field()).unwrap();
        assert_eq!(res.devices.len(), 4);
        assert_eq!(res.ranking.len(), 4);
    }

    #[test]
    fn test_comparator_has_valid_simulator() {
        let sim = Arc::new(MicromagneticSimulator::with_spoon_dimensions(0.2, 0.1, 0.02));
        let comp = DeviceComparator::new(sim);
        let tensor = comp.simulator().get_demagnetization_tensor();
        assert!(tensor.n_xx.is_finite());
    }
}
