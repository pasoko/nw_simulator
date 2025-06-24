use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterInterface {
    pub id: u32,
    pub ip_address: String,
    pub netmask: String,
    pub connected_router_id: Option<u32>,
    pub cost: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingTableEntry {
    pub destination: String,
    pub netmask: String,
    pub next_hop: String,
    pub interface_id: u32,
    pub metric: u32,
    pub protocol: RoutingProtocol,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RoutingProtocol {
    Direct,
    Static,
    OSPF,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterState {
    pub id: u32,
    pub name: String,
    pub interfaces: HashMap<u32, RouterInterface>,
    pub routing_table: Vec<RoutingTableEntry>,
    pub ospf_state: Option<OSPFState>,
    pub is_failed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OSPFState {
    pub router_id: String,
    pub area_id: String,
    pub neighbors: HashMap<u32, OSPFNeighbor>,
    pub lsa_database: Vec<LSA>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OSPFNeighbor {
    pub router_id: String,
    pub state: OSPFNeighborState,
    pub interface_id: u32,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OSPFNeighborState {
    Down,      // Initial state, no information received
    Init,      // Hello received, but not seen self in neighbor list
    TwoWay,    // Bidirectional communication established
    ExStart,   // Master/Slave negotiation for DD exchange
    Exchange,  // Database Description packets being exchanged
    Loading,   // LSAs being requested and received
    Full,      // Databases synchronized, full adjacency
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSAHeader {
    pub ls_age: u16,                   // Time in seconds since LSA originated
    pub ls_type: LSAType,              // Type of LSA
    pub link_state_id: String,         // Depends on LSA type
    pub advertising_router: String,    // Router that originated the LSA
    pub ls_sequence_number: u32,       // For detecting old/duplicate LSAs
    pub ls_checksum: u16,              // Fletcher checksum
    pub length: u16,                   // Length of LSA including header
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSA {
    pub header: LSAHeader,
    pub data: LSAData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LSAData {
    Router(RouterLSA),
    Network(NetworkLSA),
    Summary(SummaryLSA),
    ASExternal(ASExternalLSA),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterLSA {
    pub flags: u8,                     // V, E, B bits
    pub num_links: u16,
    pub links: Vec<RouterLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterLink {
    pub link_id: String,               // Depends on link type
    pub link_data: String,             // Depends on link type
    pub link_type: LinkType,
    pub num_tos: u8,
    pub metric: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LinkType {
    PointToPoint = 1,
    TransitNetwork = 2,
    StubNetwork = 3,
    VirtualLink = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLSA {
    pub network_mask: String,
    pub attached_routers: Vec<String>, // Router IDs of attached routers
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryLSA {
    pub network_mask: String,
    pub metric: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ASExternalLSA {
    pub network_mask: String,
    pub metric: u32,
    pub forwarding_address: String,
    pub external_route_tag: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LSAType {
    RouterLSA = 1,
    NetworkLSA = 2,
    SummaryLSA = 3,
    SummaryASBR = 4,
    ASExternalLSA = 5,
}

impl RouterState {
    pub fn new(id: u32, name: String) -> Self {
        RouterState {
            id,
            name,
            interfaces: HashMap::new(),
            routing_table: Vec::new(),
            ospf_state: None,
            is_failed: false,
        }
    }

    pub fn add_interface(&mut self, interface: RouterInterface) {
        self.interfaces.insert(interface.id, interface);
    }

    pub fn enable_ospf(&mut self, router_id: String, area_id: String) {
        self.ospf_state = Some(OSPFState {
            router_id,
            area_id,
            neighbors: HashMap::new(),
            lsa_database: Vec::new(),
        });
    }

    pub fn update_routing_table(&mut self, entry: RoutingTableEntry) {
        self.routing_table.retain(|e| {
            !(e.destination == entry.destination && e.netmask == entry.netmask)
        });
        self.routing_table.push(entry);
    }
}