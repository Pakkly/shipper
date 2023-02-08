use serde::{Deserialize, Serialize};
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum InstanceMode {
    multi_instance,
    single_instance,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ShipperfileGenerated {
    //base64 encoded string
    pub icon: String,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Shipperfile {
    pub app_id: String,
    //pub os: OS // Currently these will do nothing for the shipper.
    //pub architecture: Architecture, // Currently these will do nothing for the shipper.
    pub program_path_to_binary: String,
    pub program_arguments: Option<Vec<String>>,
    pub program_working_subdirectory: Option<String>,
    pub instance_mode: Option<InstanceMode>,
    pub _generated: ShipperfileGenerated,
}
