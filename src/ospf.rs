use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OSPFPacket {
    pub version: u8,
    pub packet_type: OSPFPacketType,
    pub router_id: String,
    pub area_id: String,
    pub checksum: u16,
    pub auth_type: u16,
    pub authentication: u64,
    pub data: OSPFPacketData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub options: u8,
    pub router_priority: u8,
    pub router_dead_interval: u32,
    pub designated_router: String,
    pub backup_designated_router: String,
    pub neighbors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseDescriptionPacket {
    pub interface_mtu: u16,
    pub options: u8,
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
    pub options: u8,
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
    pub data: String, // Serialized LSA data for simplicity
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LSAData {
    RouterLSA(RouterLSAData),
    NetworkLSA(NetworkLSAData),
    SummaryLSA(SummaryLSAData),
    ASExternalLSA(ASExternalLSAData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterLSAData {
    pub flags: u8,
    pub num_links: u16,
    pub links: Vec<RouterLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterLink {
    pub link_id: String,
    pub link_data: String,
    pub link_type: RouterLinkType,
    pub num_tos: u8,
    pub metric: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RouterLinkType {
    PointToPoint = 1,
    TransitNetwork = 2,
    StubNetwork = 3,
    VirtualLink = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLSAData {
    pub network_mask: String,
    pub attached_routers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryLSAData {
    pub network_mask: String,
    pub metric: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ASExternalLSAData {
    pub network_mask: String,
    pub external_route_tag: u32,
    pub metric: u32,
    pub forwarding_address: String,
}