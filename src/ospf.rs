use serde::{Serialize, Deserialize};
use crate::router::LSAData;
use crate::ospf_auth::{AuthType, AuthData};
use crate::ospf_options::OSPFOptions;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OSPFPacket {
    pub version: u8,
    pub packet_type: OSPFPacketType,
    pub router_id: String,
    pub area_id: String,
    pub checksum: u16,
    pub auth_type: AuthType,
    pub auth_data: AuthData,
    pub data: OSPFPacketData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OSPFPacketType {
    Hello = 1,
    DatabaseDescription = 2,
    LinkStateRequest = 3,
    LinkStateUpdate = 4,
    LinkStateAcknowledgment = 5,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OSPFPacketData {
    Hello(HelloPacket),
    DatabaseDescription(DatabaseDescriptionPacket),
    LinkStateRequest(LinkStateRequestPacket),
    LinkStateUpdate(LinkStateUpdatePacket),
    LinkStateAcknowledgment(LinkStateAcknowledgmentPacket),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloPacket {
    pub network_mask: String,
    pub hello_interval: u16,
    pub options: OSPFOptions,
    pub router_priority: u8,
    pub router_dead_interval: u32,
    pub designated_router: String,
    pub backup_designated_router: String,
    pub neighbors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseDescriptionPacket {
    pub interface_mtu: u16,
    pub options: OSPFOptions,
    pub flags: u8,
    pub dd_sequence_number: u32,
    pub lsa_headers: Vec<LSAHeader>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkStateRequestPacket {
    pub requests: Vec<LSARequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkStateUpdatePacket {
    pub lsas: Vec<LSA>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkStateAcknowledgmentPacket {
    pub lsa_headers: Vec<LSAHeader>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSAHeader {
    pub age: u16,
    pub options: OSPFOptions,
    pub lsa_type: u8,
    pub link_state_id: String,
    pub advertising_router: String,
    pub sequence_number: u32,
    pub checksum: u16,
    pub length: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSARequest {
    pub lsa_type: u8,
    pub link_state_id: String,
    pub advertising_router: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSA {
    pub header: LSAHeader,
    pub data: LSAData, // Using LSAData from router module
}

// LSA-related types are now imported from router module