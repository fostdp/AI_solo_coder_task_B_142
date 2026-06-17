use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, clickhouse::Row)]
pub struct SinanSensorData {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub device_id: String,
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,
    pub magnetic_moment_x: f64,
    pub magnetic_moment_y: f64,
    pub magnetic_moment_z: f64,
    pub magnetic_moment_magnitude: f64,
    pub remanence: f64,
    pub pointing_deviation: f64,
    pub environment_temp: f64,
    pub location_lat: f64,
    pub location_lon: f64,
    #[serde(default)]
    pub is_alert: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, clickhouse::Row)]
pub struct GeomagneticFieldData {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,
    pub target_year: f64,
    pub location_lat: f64,
    pub location_lon: f64,
    pub field_intensity: f64,
    pub declination: f64,
    pub inclination: f64,
    pub bx: f64,
    pub by: f64,
    pub bz: f64,
    #[serde(default = "default_model_source")]
    pub model_source: String,
}

fn default_model_source() -> String {
    "CALS10K".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointingSimulationParams {
    pub device_id: String,
    pub simulation_id: String,
    pub target_year: f64,
    pub location_lat: f64,
    pub location_lon: f64,
    pub magnetic_moment_magnitude: f64,
    pub remanence: f64,
    pub temperature: f64,
    pub friction_coefficient: f64,
    pub demagnetization_factor: f64,
    pub anisotropy_constant: f64,
    pub expected_azimuth: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, clickhouse::Row)]
pub struct PointingSimulationResult {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,
    pub device_id: String,
    pub simulation_id: String,
    pub target_year: f64,
    pub location_lat: f64,
    pub location_lon: f64,
    pub expected_azimuth: f64,
    pub simulated_azimuth: f64,
    pub pointing_accuracy: f64,
    pub magnetic_moment_magnitude: f64,
    pub remanence: f64,
    pub temperature: f64,
    pub friction_coefficient: f64,
    pub demagnetization_factor: f64,
    pub anisotropy_constant: f64,
    pub model_parameters: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, clickhouse::Row)]
pub struct AlertEvent {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,
    pub device_id: String,
    pub alert_type: String,
    pub alert_level: String,
    pub pointing_deviation: f64,
    pub threshold: f64,
    pub sensor_data_id: Uuid,
    #[serde(default)]
    pub is_acknowledged: bool,
    pub message: String,
    pub mqtt_topic: String,
    #[serde(default)]
    pub mqtt_published: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchaeologyMagneticData {
    pub site_name: String,
    pub location_lat: f64,
    pub location_lon: f64,
    pub sample_age: f64,
    pub sample_age_error: f64,
    pub declination: f64,
    pub declination_error: f64,
    pub inclination: f64,
    pub inclination_error: f64,
    pub intensity: f64,
    pub intensity_error: f64,
    pub sample_material: String,
    pub reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorFieldPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub bx: f64,
    pub by: f64,
    pub bz: f64,
    pub magnitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorFieldRequest {
    pub target_year: f64,
    pub center_lat: f64,
    pub center_lon: f64,
    pub radius_km: f64,
    pub grid_size: usize,
    pub altitude_km: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorFieldResponse {
    pub target_year: f64,
    pub center_lat: f64,
    pub center_lon: f64,
    pub grid_size: usize,
    pub points: Vec<VectorFieldPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertAcknowledgeRequest {
    pub alert_id: Uuid,
    pub acknowledged_by: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorDataQuery {
    pub device_id: Option<String>,
    pub time_range: Option<QueryRange>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    Sinan,
    Zhinanyu,
    HanLuopan,
    MemsCompass,
}

impl DeviceType {
    pub fn display_name(&self) -> &'static str {
        match self {
            DeviceType::Sinan => "司南（磁勺）",
            DeviceType::Zhinanyu => "指南鱼",
            DeviceType::HanLuopan => "旱罗盘",
            DeviceType::MemsCompass => "现代MEMS电子罗盘",
        }
    }

    pub fn era(&self) -> &'static str {
        match self {
            DeviceType::Sinan => "战国-汉代（公元前3世纪-公元3世纪）",
            DeviceType::Zhinanyu => "宋代（公元10-13世纪）",
            DeviceType::HanLuopan => "宋代-明清（公元11世纪-19世纪）",
            DeviceType::MemsCompass => "现代（公元21世纪）",
        }
    }

    pub fn all() -> Vec<DeviceType> {
        vec![
            DeviceType::Sinan,
            DeviceType::Zhinanyu,
            DeviceType::HanLuopan,
            DeviceType::MemsCompass,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceGeometryParams {
    pub device_type: DeviceType,
    pub length_m: f64,
    pub width_m: f64,
    pub thickness_m: f64,
    pub handle_length_m: Option<f64>,
    pub pivot_friction: f64,
    pub water_viscosity: Option<f64>,
    pub bowl_radius_m: Option<f64>,
}

impl Default for DeviceGeometryParams {
    fn default() -> Self {
        Self::for_type(DeviceType::Sinan)
    }
}

impl DeviceGeometryParams {
    pub fn for_type(dt: DeviceType) -> Self {
        match dt {
            DeviceType::Sinan => DeviceGeometryParams {
                device_type: DeviceType::Sinan,
                length_m: 0.17,
                width_m: 0.08,
                thickness_m: 0.015,
                handle_length_m: Some(0.06),
                pivot_friction: 0.15,
                water_viscosity: None,
                bowl_radius_m: None,
            },
            DeviceType::Zhinanyu => DeviceGeometryParams {
                device_type: DeviceType::Zhinanyu,
                length_m: 0.06,
                width_m: 0.015,
                thickness_m: 0.002,
                handle_length_m: None,
                pivot_friction: 0.005,
                water_viscosity: Some(0.001),
                bowl_radius_m: Some(0.05),
            },
            DeviceType::HanLuopan => DeviceGeometryParams {
                device_type: DeviceType::HanLuopan,
                length_m: 0.03,
                width_m: 0.002,
                thickness_m: 0.001,
                handle_length_m: None,
                pivot_friction: 0.02,
                water_viscosity: None,
                bowl_radius_m: None,
            },
            DeviceType::MemsCompass => DeviceGeometryParams {
                device_type: DeviceType::MemsCompass,
                length_m: 0.002,
                width_m: 0.002,
                thickness_m: 0.0005,
                handle_length_m: None,
                pivot_friction: 0.0,
                water_viscosity: None,
                bowl_radius_m: None,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiDeviceCompareRequest {
    pub target_year: f64,
    pub location_lat: f64,
    pub location_lon: f64,
    pub devices: Vec<DeviceType>,
    pub magnetic_moment_magnitude: Option<f64>,
    pub remanence: Option<f64>,
    pub temperature: f64,
    pub expected_azimuth: f64,
    pub monte_carlo_samples: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleDeviceAccuracy {
    pub device_type: DeviceType,
    pub display_name: String,
    pub era: String,
    pub simulated_azimuth: f64,
    pub pointing_accuracy_deg: f64,
    pub mean_deviation_deg: f64,
    pub std_deviation_deg: f64,
    pub min_deviation_deg: f64,
    pub max_deviation_deg: f64,
    pub p95_deviation_deg: f64,
    pub geometry: DeviceGeometryParams,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiDeviceCompareResponse {
    pub target_year: f64,
    pub location_lat: f64,
    pub location_lon: f64,
    pub expected_azimuth: f64,
    pub geomagnetic_intensity_nT: f64,
    pub geomagnetic_declination_deg: f64,
    pub geomagnetic_inclination_deg: f64,
    pub devices: Vec<SingleDeviceAccuracy>,
    pub ranking: Vec<DeviceType>,
    pub summary: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum InterferenceType {
    FerrousObject,
    PowerLine,
    ElectronicDevice,
    BuildingRebar,
    Loudspeaker,
    LightningStorm,
}

impl InterferenceType {
    pub fn display_name(&self) -> &'static str {
        match self {
            InterferenceType::FerrousObject => "铁磁性物体（铁器）",
            InterferenceType::PowerLine => "高压输电线",
            InterferenceType::ElectronicDevice => "电子设备（手机/电脑）",
            InterferenceType::BuildingRebar => "建筑钢筋",
            InterferenceType::Loudspeaker => "扬声器/磁铁",
            InterferenceType::LightningStorm => "雷暴天气",
        }
    }

    pub fn all() -> Vec<InterferenceType> {
        vec![
            InterferenceType::FerrousObject,
            InterferenceType::PowerLine,
            InterferenceType::ElectronicDevice,
            InterferenceType::BuildingRebar,
            InterferenceType::Loudspeaker,
            InterferenceType::LightningStorm,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterferenceSource {
    pub interference_type: InterferenceType,
    pub distance_m: f64,
    pub intensity_factor: f64,
    pub azimuth_deg: f64,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterferenceSimulationRequest {
    pub device_type: DeviceType,
    pub target_year: f64,
    pub location_lat: f64,
    pub location_lon: f64,
    pub temperature: f64,
    pub expected_azimuth: f64,
    pub magnetic_moment_magnitude: Option<f64>,
    pub remanence: Option<f64>,
    pub interference_sources: Vec<InterferenceSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterferenceEffect {
    pub interference_type: InterferenceType,
    pub display_name: String,
    pub distance_m: f64,
    pub induced_field_nT: f64,
    pub induced_field_azimuth_deg: f64,
    pub deviation_contribution_deg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterferenceSimulationResponse {
    pub device_type: DeviceType,
    pub target_year: f64,
    pub location_lat: f64,
    pub location_lon: f64,
    pub baseline_azimuth: f64,
    pub baseline_accuracy_deg: f64,
    pub interfered_azimuth: f64,
    pub interfered_accuracy_deg: f64,
    pub total_deviation_delta_deg: f64,
    pub total_interference_field_nT: f64,
    pub interference_ratio: f64,
    pub effects: Vec<InterferenceEffect>,
    pub warning_level: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveSinanRequest {
    pub device_type: DeviceType,
    pub target_year: f64,
    pub location_lat: f64,
    pub location_lon: f64,
    pub magnetic_moment_magnitude: f64,
    pub remanence: f64,
    pub temperature: f64,
    pub friction_coefficient: f64,
    pub anisotropy_constant: f64,
    pub demagnetization_factor_override: Option<f64>,
    pub spoon_length_m: Option<f64>,
    pub spoon_width_m: Option<f64>,
    pub spoon_thickness_m: Option<f64>,
    pub expected_azimuth: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveSinanResponse {
    pub device_type: DeviceType,
    pub simulated_azimuth: f64,
    pub pointing_accuracy_deg: f64,
    pub expected_azimuth: f64,
    pub magnetic_moment_vector: [f64; 3],
    pub effective_moment_magnitude: f64,
    pub torque_magnitude: f64,
    pub thermal_fluctuation_deg: f64,
    pub demagnetization_tensor: HashMap<String, f64>,
    pub geomagnetic_field: [f64; 3],
    pub geomagnetic_intensity_nT: f64,
    pub geomagnetic_declination_deg: f64,
    pub geomagnetic_inclination_deg: f64,
    pub spoon_dimensions_m: [f64; 3],
    pub physics_insights: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossEraCompareRequest {
    pub location_lat: f64,
    pub location_lon: f64,
    pub ancient_year: f64,
    pub modern_year: f64,
    pub ancient_device: DeviceType,
    pub temperature: f64,
    pub expected_azimuth: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossEraCompareResponse {
    pub ancient: SingleDeviceAccuracy,
    pub modern_mems: SingleDeviceAccuracy,
    pub improvement_factor: f64,
    pub accuracy_gap_deg: f64,
    pub narrative: String,
    pub historical_context: String,
}
