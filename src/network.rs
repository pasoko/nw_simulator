use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::router::{RouterState, RouterInterface};
use crate::network_type::OSPFNetworkType;
use crate::device::{HostDevice, DeviceType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLink {
    pub id: u32,
    pub router1_id: u32,
    pub router1_interface_id: u32,
    pub router2_id: u32,
    pub router2_interface_id: u32,
    pub cost: u32,
    pub bandwidth: u64,
    pub delay: u32,
    pub is_failed: bool,
    pub network_type: OSPFNetworkType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTopology {
    pub routers: HashMap<u32, RouterState>,
    pub hosts: HashMap<u32, HostDevice>,
    pub links: HashMap<u32, NetworkLink>,
    next_router_id: u32,
    next_host_id: u32,
    next_link_id: u32,
    next_interface_id: u32,
}

impl NetworkTopology {
    pub fn new() -> Self {
        NetworkTopology {
            routers: HashMap::new(),
            hosts: HashMap::new(),
            links: HashMap::new(),
            next_router_id: 1,
            next_host_id: 1000,  // ホストIDは1000から開始
            next_link_id: 1,
            next_interface_id: 1,
        }
    }

    pub fn add_router(&mut self, name: String) -> u32 {
        let id = self.next_router_id;
        self.next_router_id += 1;
        let router = RouterState::new(id, name);
        self.routers.insert(id, router);
        id
    }

    pub fn connect_routers(
        &mut self,
        router1_id: u32,
        router2_id: u32,
        cost: u32,
    ) -> Result<u32, String> {
        self.connect_routers_with_type(router1_id, router2_id, cost, None)
    }
    
    pub fn connect_routers_with_type(
        &mut self,
        router1_id: u32,
        router2_id: u32,
        cost: u32,
        network_type: Option<OSPFNetworkType>,
    ) -> Result<u32, String> {
        if !self.routers.contains_key(&router1_id) {
            return Err(format!("Router {} not found", router1_id));
        }
        if !self.routers.contains_key(&router2_id) {
            return Err(format!("Router {} not found", router2_id));
        }

        let link_id = self.next_link_id;
        self.next_link_id += 1;

        let interface1_id = self.next_interface_id;
        self.next_interface_id += 1;
        let interface2_id = self.next_interface_id;
        self.next_interface_id += 1;

        // Determine network type
        let net_type = network_type.unwrap_or_else(|| {
            // Auto-detect: if exactly 2 routers with no other connections, use Point-to-Point
            let router1_links = self.get_neighbors(router1_id).len();
            let router2_links = self.get_neighbors(router2_id).len();
            
            if router1_links == 0 && router2_links == 0 {
                // Only these two routers connected to each other
                OSPFNetworkType::PointToPoint
            } else {
                // Multiple routers, use Broadcast (default)
                OSPFNetworkType::default()
            }
        });
        
        let netmask = net_type.default_network_mask();
        
        let interface1 = RouterInterface {
            id: interface1_id,
            name: String::new(),  // ルーターのadd_interfaceメソッドで自動設定される
            ip_address: format!("10.0.{}.1", link_id),
            netmask: netmask.to_string(),
            connected_router_id: Some(router2_id),
            cost,
            enabled: true,
            hello_interval: 10,    // OSPFv2デフォルト値
            dead_interval: 40,     // OSPFv2デフォルト値 (hello * 4)
            priority: 1,           // DR選出優先度デフォルト
            mtu: 1500,            // Ethernetデフォルト
            manual_config: false,  // 自動設定
            auth_config: crate::ospf_auth::AuthConfig::default(),  // 認証なし（デフォルト）
        };

        let interface2 = RouterInterface {
            id: interface2_id,
            name: String::new(),  // ルーターのadd_interfaceメソッドで自動設定される
            ip_address: format!("10.0.{}.2", link_id),
            netmask: netmask.to_string(),
            connected_router_id: Some(router1_id),
            cost,
            enabled: true,
            hello_interval: 10,    // OSPFv2デフォルト値
            dead_interval: 40,     // OSPFv2デフォルト値 (hello * 4)
            priority: 1,           // DR選出優先度デフォルト
            mtu: 1500,            // Ethernetデフォルト
            manual_config: false,  // 自動設定
            auth_config: crate::ospf_auth::AuthConfig::default(),  // 認証なし（デフォルト）
        };

        if let Some(router1) = self.routers.get_mut(&router1_id) {
            router1.add_interface(interface1);
        }

        if let Some(router2) = self.routers.get_mut(&router2_id) {
            router2.add_interface(interface2);
        }

        let link = NetworkLink {
            id: link_id,
            router1_id,
            router1_interface_id: interface1_id,
            router2_id,
            router2_interface_id: interface2_id,
            cost,
            bandwidth: 100_000_000,
            delay: 10,
            is_failed: false,
            network_type: net_type,
        };

        self.links.insert(link_id, link);
        Ok(link_id)
    }

    pub fn enable_ospf_on_router(&mut self, router_id: u32) -> Result<(), String> {
        if let Some(router) = self.routers.get_mut(&router_id) {
            let router_ip = format!("{}.{}.{}.{}", 
                1, 1, 1, router_id);
            router.enable_ospf(router_ip, "0.0.0.0".to_string());
            Ok(())
        } else {
            Err(format!("Router {} not found", router_id))
        }
    }

    pub fn get_neighbors(&self, router_id: u32) -> Vec<u32> {
        let mut neighbors = Vec::new();
        for link in self.links.values() {
            if link.router1_id == router_id {
                neighbors.push(link.router2_id);
            } else if link.router2_id == router_id {
                neighbors.push(link.router1_id);
            }
        }
        neighbors
    }

    pub fn add_host(&mut self, name: String, ip_address: String, netmask: String, default_gateway: String) -> u32 {
        let id = self.next_host_id;
        self.next_host_id += 1;
        let host = HostDevice::new(id, name, ip_address, netmask, default_gateway);
        self.hosts.insert(id, host);
        id
    }

    pub fn connect_host_to_router(&mut self, host_id: u32, router_id: u32) -> Result<u32, String> {
        // ルーターが存在するか確認
        if !self.routers.contains_key(&router_id) {
            return Err(format!("Router {} not found", router_id));
        }

        // ホストが存在するか確認
        if !self.hosts.contains_key(&host_id) {
            return Err(format!("Host {} not found", host_id));
        }

        // ルーターに新しいインターフェースを作成
        let interface_id = self.next_interface_id;
        self.next_interface_id += 1;

        let host = self.hosts.get(&host_id).unwrap();
        let _host_network = self.get_network_address(&host.ip_address, &host.netmask);
        
        // ルーター側のインターフェース（通常はホストのデフォルトゲートウェイと同じIP）
        let router_interface = RouterInterface {
            id: interface_id,
            name: format!("IF-Host{}", host_id),
            ip_address: host.default_gateway.clone(),
            netmask: host.netmask.clone(),
            connected_router_id: None,  // ホスト接続なのでNone
            cost: 1,
            enabled: true,
            hello_interval: 10,
            dead_interval: 40,
            priority: 1,
            mtu: 1500,
            manual_config: true,
            auth_config: crate::ospf_auth::AuthConfig::default(),  // 認証なし（デフォルト）
        };

        // ルーターにインターフェースを追加
        if let Some(router) = self.routers.get_mut(&router_id) {
            router.add_interface(router_interface);
        }

        // ホストを接続
        if let Some(host) = self.hosts.get_mut(&host_id) {
            host.connect_to_router(router_id, interface_id);
        }

        // 仮想リンクIDを返す（ホスト接続用）
        let link_id = self.next_link_id;
        self.next_link_id += 1;
        
        Ok(link_id)
    }

    fn get_network_address(&self, ip: &str, netmask: &str) -> String {
        let ip_parts: Vec<u8> = ip.split('.')
            .filter_map(|s| s.parse().ok())
            .collect();
        let mask_parts: Vec<u8> = netmask.split('.')
            .filter_map(|s| s.parse().ok())
            .collect();

        if ip_parts.len() != 4 || mask_parts.len() != 4 {
            return String::new();
        }

        format!("{}.{}.{}.{}",
            ip_parts[0] & mask_parts[0],
            ip_parts[1] & mask_parts[1],
            ip_parts[2] & mask_parts[2],
            ip_parts[3] & mask_parts[3]
        )
    }

    pub fn get_device_type(&self, device_id: u32) -> Option<DeviceType> {
        if self.routers.contains_key(&device_id) {
            Some(DeviceType::Router)
        } else if self.hosts.contains_key(&device_id) {
            Some(DeviceType::Host)
        } else {
            None
        }
    }
}