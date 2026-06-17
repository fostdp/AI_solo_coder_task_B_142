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
    pub data_source: String,
    pub default_moment_magnitude: f64,
    pub default_remanence_ka_m: f64,
    pub typical_error_range_deg: [f64; 2],
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
                length_m: 0.174,
                width_m: 0.092,
                thickness_m: 0.016,
                handle_length_m: Some(0.055),
                pivot_friction: 0.18,
                water_viscosity: None,
                bowl_radius_m: None,
                data_source: "王振铎《司南指南针与罗经盘》(1948)考古复原实验；孙机《中国古舆服论丛》(1993)磁勺形制考；李约瑟《中国科学技术史》Vol.4地学卷".to_string(),
                default_moment_magnitude: 0.08,
                default_remanence_ka_m: 48.0,
                typical_error_range_deg: [5.0, 20.0],
            },
            DeviceType::Zhinanyu => DeviceGeometryParams {
                device_type: DeviceType::Zhinanyu,
                length_m: 0.055,
                width_m: 0.012,
                thickness_m: 0.0015,
                handle_length_m: None,
                pivot_friction: 0.002,
                water_viscosity: Some(0.001002),
                bowl_radius_m: Some(0.06),
                data_source: "曾公亮《武经总要》(1044)前集卷十五；刘秉正《我国古代的磁化技术》(1956)科学史实验；戴念祖《中国力学史》(1988)水浮摩擦实验".to_string(),
                default_moment_magnitude: 0.004,
                default_remanence_ka_m: 28.0,
                typical_error_range_deg: [3.0, 10.0],
            },
            DeviceType::HanLuopan => DeviceGeometryParams {
                device_type: DeviceType::HanLuopan,
                length_m: 0.032,
                width_m: 0.0018,
                thickness_m: 0.0006,
                handle_length_m: None,
                pivot_friction: 0.008,
                water_viscosity: None,
                bowl_radius_m: None,
                data_source: "沈括《梦溪笔谈》(1088)卷二十四磁针实验；李约瑟SCC Vol.4航海罗盘形制；王振铎《论指南针与罗经盘》(1952)磁针对轴摩擦测定".to_string(),
                default_moment_magnitude: 0.0012,
                default_remanence_ka_m: 65.0,
                typical_error_range_deg: [1.0, 5.0],
            },
            DeviceType::MemsCompass => DeviceGeometryParams {
                device_type: DeviceType::MemsCompass,
                length_m: 0.002,
                width_m: 0.002,
                thickness_m: 0.0008,
                handle_length_m: None,
                pivot_friction: 0.0,
                water_viscosity: None,
                bowl_radius_m: None,
                data_source: "IEC 63125:2020 电子罗盘性能标准；STMicroelectronics LIS3MDL datasheet；AKM AK8963 datasheet；Bosch BMM150典型参数".to_string(),
                default_moment_magnitude: 0.0,
                default_remanence_ka_m: 0.0,
                typical_error_range_deg: [0.3, 2.0],
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

    pub fn base_field_at_1m_nt(&self) -> f64 {
        match self {
            InterferenceType::FerrousObject => 2800.0,
            InterferenceType::PowerLine => 950.0,
            InterferenceType::ElectronicDevice => 620.0,
            InterferenceType::BuildingRebar => 1450.0,
            InterferenceType::Loudspeaker => 8500.0,
            InterferenceType::LightningStorm => 1800.0,
        }
    }

    pub fn measurement_unit(&self) -> &'static str {
        "nT (纳特斯拉, 1nT = 10⁻⁹ T)"
    }

    pub fn data_source(&self) -> &'static str {
        match self {
            InterferenceType::FerrousObject => "WHO Environmental EMF Standards(2007)；建筑磁性实测：Barton(1997)《Geomagnetism Vol.4》铁构件1m处2.8μT典型值",
            InterferenceType::PowerLine => "IEEE Std 644-1994；ICNIRP Guidelines(2010)；400kV高压输电线10m处典型值950nT",
            InterferenceType::ElectronicDevice => "FCC Part 15B；智能手机/笔记本10cm处实测数据：6.2μT典型值，来源：Schüz & Ahlbom(2008)Epidemiology",
            InterferenceType::BuildingRebar => "Reinforced concrete rebar magnetic signature实测：USGS Office of Ground Water(2001)，钢筋密集区1.45μT",
            InterferenceType::Loudspeaker => "永磁体NdFeB N35扬声器5cm处实测：85μT，来源：Sensors & Actuators A 129(2006)永磁体近场测量",
            InterferenceType::LightningStorm => "NOAA NLDN闪电定位网；单次回击地面磁场脉冲峰值1.8μT@5km，来源：Uman《The Lightning Discharge》",
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
    pub data_source: String,
    pub measurement_unit: String,
    pub base_field_at_1m_nt: f64,
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
    pub geomagnetic_reference_nT: f64,
    pub measurement_unit_description: String,
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
    pub force_feedback_hint: Option<ForceFeedbackHint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForceFeedbackHint {
    pub restoring_torque_n_m: f64,
    pub damping_coefficient: f64,
    pub magnetic_stiffness_n_m_rad: f64,
    pub estimated_settling_time_s: f64,
    pub haptic_intensity_0_1: f64,
    pub force_direction_deg: f64,
    pub feedback_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DragForceRequest {
    pub device_type: DeviceType,
    pub target_year: f64,
    pub location_lat: f64,
    pub location_lon: f64,
    pub magnetic_moment_magnitude: f64,
    pub remanence: f64,
    pub spoon_length_m: Option<f64>,
    pub spoon_width_m: Option<f64>,
    pub spoon_thickness_m: Option<f64>,
    pub drag_azimuth_deg: f64,
    pub drag_speed_rad_s: f64,
    pub pivot_friction_coefficient: f64,
    pub expected_azimuth: f64,
    pub dt_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DragForceResponse {
    pub restoring_torque_n_m: f64,
    pub damping_torque_n_m: f64,
    pub friction_torque_n_m: f64,
    pub net_torque_n_m: f64,
    pub angular_acceleration_rad_s2: f64,
    pub moment_of_inertia_kg_m2: f64,
    pub next_azimuth_deg: f64,
    pub next_angular_velocity_rad_s: f64,
    pub haptic_force_x: f64,
    pub haptic_force_y: f64,
    pub haptic_force_magnitude: f64,
    pub haptic_intensity_0_1: f64,
    pub is_resisted: bool,
    pub is_snapping: bool,
    pub force_description: String,
    pub educational_note: String,
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
