use std::sync::Arc;

use nalgebra::Vector3;

use crate::compute_pool::{ComputeFuture, ComputePool};
use crate::errors::Result;
use crate::micromagnetic_simulation::MicromagneticSimulator;
use crate::models::{CrossEraCompareRequest, CrossEraCompareResponse};

#[derive(Clone)]
pub struct EraComparator {
    simulator: Arc<MicromagneticSimulator>,
    pool: &'static ComputePool,
}

impl EraComparator {
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
        request: &CrossEraCompareRequest,
        ancient_field: Vector3<f64>,
        modern_field: Vector3<f64>,
    ) -> Result<CrossEraCompareResponse> {
        self.simulator
            .compare_cross_era(request, ancient_field, modern_field)
    }

    pub fn compare_async(
        &self,
        request: CrossEraCompareRequest,
        ancient_field: Vector3<f64>,
        modern_field: Vector3<f64>,
    ) -> ComputeFuture<Result<CrossEraCompareResponse>> {
        let sim = self.simulator.clone();
        self.pool
            .spawn_compute("era_compare", move || {
                sim.compare_cross_era(&request, ancient_field, modern_field)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::micromagnetic_simulation::MicromagneticSimulator;
    use crate::models::{CrossEraCompareRequest, DeviceType};
    use std::sync::Arc;

    fn ancient_field() -> Vector3<f64> {
        Vector3::new(28000.0e-9, 0.0, 40000.0e-9)
    }
    fn modern_field() -> Vector3<f64> {
        Vector3::new(30000.0e-9, 0.0, 45000.0e-9)
    }

    #[test]
    fn test_era_compare_sync_sinan_normal() {
        let sim = Arc::new(MicromagneticSimulator::new());
        let ec = EraComparator::new(sim);
        let req = CrossEraCompareRequest {
            ancient_device: DeviceType::Sinan,
            ancient_year: 202.0,
            modern_year: 2024.0,
            location_lat: 39.9,
            location_lon: 116.4,
            temperature: 25.0,
            expected_azimuth: 180.0,
        };
        let res = ec.compare_sync(&req, ancient_field(), modern_field()).unwrap();
        assert!(matches!(res.ancient.device_type, DeviceType::Sinan));
        assert!(matches!(res.modern_mems.device_type, DeviceType::MemsCompass));
        assert!(res.improvement_factor.is_finite() && res.improvement_factor >= 0.0);
        assert!(!res.narrative.is_empty());
        assert!(!res.historical_context.is_empty());
        assert!(res.historical_context.contains("司南") || res.historical_context.contains("王振铎"));
    }

    #[test]
    fn test_era_compare_sync_hanluopan_normal() {
        let sim = Arc::new(MicromagneticSimulator::new());
        let ec = EraComparator::new(sim);
        let req = CrossEraCompareRequest {
            ancient_device: DeviceType::HanLuopan,
            ancient_year: 1200.0,
            modern_year: 2024.0,
            location_lat: 39.9,
            location_lon: 116.4,
            temperature: 25.0,
            expected_azimuth: 180.0,
        };
        let res = ec.compare_sync(&req, ancient_field(), modern_field()).unwrap();
        assert!(matches!(res.ancient.device_type, DeviceType::HanLuopan));
        assert!(res.historical_context.contains("郑和") || res.historical_context.contains("地理大发现") || res.historical_context.contains("远洋航海"));
    }

    #[test]
    fn test_era_compare_sync_boundary_same_field() {
        let sim = Arc::new(MicromagneticSimulator::new());
        let ec = EraComparator::new(sim);
        let req = CrossEraCompareRequest {
            ancient_device: DeviceType::HanLuopan,
            ancient_year: 1200.0,
            modern_year: 2024.0,
            location_lat: 39.9,
            location_lon: 116.4,
            temperature: 25.0,
            expected_azimuth: 180.0,
        };
        let field = ancient_field();
        let res = ec.compare_sync(&req, field, field).unwrap();
        assert!(res.improvement_factor.is_finite());
        assert!(res.ancient.mean_deviation_deg.is_finite() && res.ancient.mean_deviation_deg >= 0.0);
        assert!(res.modern_mems.mean_deviation_deg.is_finite() && res.modern_mems.mean_deviation_deg >= 0.0);
    }
}
