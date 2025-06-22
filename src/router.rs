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
    Down,
    Init,
    TwoWay,
    ExStart,
    Exchange,
    Loading,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSA {
    pub lsa_type: LSAType,
    pub advertising_router: String,
    pub sequence_number: u32,
    pub age: u16,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LSAType {
    RouterLSA,
    NetworkLSA,
    SummaryLSA,
    ASExternalLSA,
}

impl RouterState {
    pub fn new(id: u32, name: String) -> Self {
        RouterState {
            id,
            name,
            interfaces: HashMap::new(),
            routing_table: Vec::new(),
            ospf_state: None,
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