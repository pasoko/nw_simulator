use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::ospf_auth::{AuthConfig, AuthType};
use crate::console_log;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterInterface {
    pub id: u32,
    pub name: String,
    pub ip_address: String,
    pub netmask: String,
    pub connected_router_id: Option<u32>,
    pub cost: u32,
    pub enabled: bool,
    // OSPFv2 インターフェース設定パラメータ
    pub hello_interval: u16,      // Hello送信間隔（秒）
    pub dead_interval: u16,       // Dead判定時間（秒）
    pub priority: u8,             // DR/BDR選出優先度
    pub mtu: u16,                 // 最大転送単位
    pub manual_config: bool,      // 手動設定フラグ
    // OSPFv2 認証設定
    pub auth_config: AuthConfig,  // 認証設定
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingTableEntry {
    pub destination: String,
    pub netmask: String,
    pub next_hop: String,
    pub interface_id: u32,
    pub interface_name: String,
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
    pub next_interface_number: u32,
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
    pub dead_interval: u16,
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
            next_interface_number: 1,
        }
    }

    pub fn add_interface(&mut self, mut interface: RouterInterface) {
        // インターフェース名が設定されていない場合は自動生成
        if interface.name.is_empty() {
            interface.name = format!("IF{}-{}", self.name, self.next_interface_number);
            console_log!("Router {} - インターフェース名を自動生成: {} (interface_id: {})", 
                self.name, interface.name, interface.id);
            self.next_interface_number += 1;
        } else {
            console_log!("Router {} - カスタムインターフェース名を使用: {} (interface_id: {})", 
                self.name, interface.name, interface.id);
        }
        
        // デバッグ: インターフェース追加前後の状態を確認
        console_log!("Router {} - Adding interface: id={}, name={}, ip={}", 
            self.name, interface.id, interface.name, interface.ip_address);
        
        self.interfaces.insert(interface.id, interface.clone());
        
        // デバッグ: 追加後の確認
        if let Some(added_interface) = self.interfaces.get(&interface.id) {
            console_log!("Router {} - Interface added successfully: id={}, name={}", 
                self.name, added_interface.id, added_interface.name);
        }
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

    pub fn update_interface_config(&mut self, interface_id: u32, config: InterfaceConfig) -> Result<(), String> {
        if let Some(interface) = self.interfaces.get_mut(&interface_id) {
            if let Some(ip) = config.ip_address {
                interface.ip_address = ip;
                interface.manual_config = true;
            }
            if let Some(mask) = config.netmask {
                interface.netmask = mask;
            }
            if let Some(cost) = config.cost {
                interface.cost = cost;
            }
            if let Some(hello) = config.hello_interval {
                interface.hello_interval = hello;
            }
            if let Some(dead) = config.dead_interval {
                interface.dead_interval = dead;
            }
            if let Some(priority) = config.priority {
                interface.priority = priority;
            }
            if let Some(mtu) = config.mtu {
                interface.mtu = mtu;
            }
            if let Some(enabled) = config.enabled {
                interface.enabled = enabled;
            }
            // 認証設定の更新
            if let Some(auth_type) = config.auth_type {
                interface.auth_config.auth_type = auth_type;
            }
            if let Some(auth_key) = config.auth_key {
                interface.auth_config.auth_key = Some(auth_key);
            }
            if let Some(key_id) = config.auth_key_id {
                interface.auth_config.key_id = Some(key_id);
            }
            Ok(())
        } else {
            Err(format!("Interface {} not found", interface_id))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceConfig {
    pub ip_address: Option<String>,
    pub netmask: Option<String>,
    pub cost: Option<u32>,
    pub hello_interval: Option<u16>,
    pub dead_interval: Option<u16>,
    pub priority: Option<u8>,
    pub mtu: Option<u16>,
    pub enabled: Option<bool>,
    pub auth_type: Option<AuthType>,
    pub auth_key: Option<String>,
    pub auth_key_id: Option<u8>,
}