use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryEntry {
    pub id: String,
    pub role: String,
    pub company: String,
    pub contact: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Space {
    pub id: String,
    pub tag: String,
    pub name: String,
    pub level: String,
    pub space_type: Option<String>,
    pub area_m2: Option<f64>,
    pub ceiling_ht_m: Option<f64>,
    pub scope: String,
    pub parent_id: Option<String>,
    pub boundary_id: Option<String>,
    pub x: Option<f64>,
    pub y: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductType {
    pub id: String,
    pub tag: String,
    pub domain: String,
    pub category: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub description: Option<String>,
    pub mounting: Option<String>,
    pub finish: Option<String>,
    pub size_nominal: Option<String>,
    pub voltage: Option<f64>,
    pub phase: Option<i32>,
    pub hz: Option<f64>,
    pub submittal_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Placement {
    pub id: String,
    pub instance_tag: Option<String>,
    pub product_type_id: String,
    pub space_id: Option<String>,
    pub level: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub rotation: Option<f64>,
    pub cfm: Option<f64>,
    pub cfm_balanced: Option<f64>,
    pub static_pressure_pa: Option<f64>,
    pub status: String,
    pub scope: String,
    pub phase: String,
    pub weight_kg: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementSystem {
    pub placement_id: String,
    pub system_id: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct System {
    pub id: String,
    pub tag: String,
    pub name: String,
    pub system_type: String,
    pub medium: String,
    pub source_id: Option<String>,
    pub paired_system_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub system_id: String,
    pub node_type: String,
    pub placement_id: Option<String>,
    pub fitting_type: Option<String>,
    pub size_description: Option<String>,
    pub level: Option<String>,
    pub x: Option<f64>,
    pub y: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub id: String,
    pub system_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub shape: String,
    pub width_m: Option<f64>,
    pub height_m: Option<f64>,
    pub diameter_m: Option<f64>,
    pub length_m: Option<f64>,
    pub material: String,
    pub gauge: Option<i32>,
    pub pressure_class: Option<String>,
    pub construction: Option<String>,
    pub exposure: Option<String>,
    pub flow_design: Option<f64>,
    pub flow_balanced: Option<f64>,
    pub status: String,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sheet {
    pub id: String,
    pub number: String,
    pub title: String,
    pub discipline: String,
    pub sheet_size: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct View {
    pub id: String,
    pub sheet_id: String,
    pub view_type: String,
    pub title: Option<String>,
    pub scale: Option<String>,
    pub level: Option<String>,
    pub vp_x: Option<f64>,
    pub vp_y: Option<f64>,
    pub vp_width: Option<f64>,
    pub vp_height: Option<f64>,
    pub model_x_min: Option<f64>,
    pub model_y_min: Option<f64>,
    pub model_x_max: Option<f64>,
    pub model_y_max: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insulation {
    pub id: String,
    pub segment_id: Option<String>,
    pub insulation_type: String,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub thickness_m: Option<f64>,
    pub r_value: Option<f64>,
    pub facing: Option<String>,
    pub code_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyedNote {
    pub id: String,
    pub key: String,
    pub text: String,
    pub discipline: Option<String>,
    pub spec_section: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submittal {
    pub id: String,
    pub number: Option<String>,
    pub description: String,
    pub submitted_by: Option<String>,
    pub company: Option<String>,
    pub date_submitted: Option<String>,
    pub status: String,
    pub spec_section: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Revision {
    pub id: String,
    pub number: i32,
    pub name: String,
    pub date: String,
    pub description: Option<String>,
    pub author: Option<String>,
}
