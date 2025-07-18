use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// デバイスタイプ
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DeviceType {
    Router,
    Host,
}

/// ホストデバイス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostDevice {
    pub id: u32,
    pub name: String,
    pub ip_address: String,
    pub netmask: String,
    pub default_gateway: String,
    pub interface_id: u32,  // 仮想インターフェースID
    pub connected_router_id: Option<u32>,
    pub connected_interface_id: Option<u32>,
    pub arp_table: HashMap<String, String>,  // IP -> MAC
    pub is_failed: bool,
}

impl HostDevice {
    pub fn new(id: u32, name: String, ip_address: String, netmask: String, default_gateway: String) -> Self {
        HostDevice {
            id,
            name,
            ip_address,
            netmask,
            default_gateway,
            interface_id: 0,  // 単一インターフェース
            connected_router_id: None,
            connected_interface_id: None,
            arp_table: HashMap::new(),
            is_failed: false,
        }
    }

    /// ルーターへの接続
    pub fn connect_to_router(&mut self, router_id: u32, router_interface_id: u32) {
        self.connected_router_id = Some(router_id);
        self.connected_interface_id = Some(router_interface_id);
    }

    /// 切断
    pub fn disconnect(&mut self) {
        self.connected_router_id = None;
        self.connected_interface_id = None;
    }

    /// ARPエントリの追加
    pub fn add_arp_entry(&mut self, ip: String, mac: String) {
        self.arp_table.insert(ip, mac);
    }

    /// 同一サブネットかチェック
    pub fn is_same_subnet(&self, target_ip: &str) -> bool {
        let self_ip_parts: Vec<u8> = self.ip_address.split('.')
            .filter_map(|s| s.parse().ok())
            .collect();
        let target_ip_parts: Vec<u8> = target_ip.split('.')
            .filter_map(|s| s.parse().ok())
            .collect();
        let mask_parts: Vec<u8> = self.netmask.split('.')
            .filter_map(|s| s.parse().ok())
            .collect();

        if self_ip_parts.len() != 4 || target_ip_parts.len() != 4 || mask_parts.len() != 4 {
            return false;
        }

        for i in 0..4 {
            if (self_ip_parts[i] & mask_parts[i]) != (target_ip_parts[i] & mask_parts[i]) {
                return false;
            }
        }

        true
    }

    /// 次ホップの決定（同一サブネットならtarget_ip、そうでなければdefault_gateway）
    pub fn get_next_hop(&self, target_ip: &str) -> String {
        if self.is_same_subnet(target_ip) {
            target_ip.to_string()
        } else {
            self.default_gateway.clone()
        }
    }
}

/// パケットタイプ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PacketType {
    OSPF(crate::ospf::OSPFPacket),
    ICMP(ICMPPacket),
}

/// ICMPパケット（基本実装）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ICMPPacket {
    pub packet_type: ICMPType,
    pub code: u8,
    pub checksum: u16,
    pub identifier: u16,
    pub sequence_number: u16,
    pub data: Vec<u8>,
}

/// ICMPタイプ
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ICMPType {
    EchoReply = 0,
    DestinationUnreachable = 3,
    EchoRequest = 8,
    TimeExceeded = 11,
}

impl ICMPPacket {
    pub fn new_echo_request(identifier: u16, sequence_number: u16) -> Self {
        ICMPPacket {
            packet_type: ICMPType::EchoRequest,
            code: 0,
            checksum: 0,
            identifier,
            sequence_number,
            data: vec![0; 32],  // 32バイトのダミーデータ
        }
    }

    pub fn new_echo_reply(identifier: u16, sequence_number: u16) -> Self {
        ICMPPacket {
            packet_type: ICMPType::EchoReply,
            code: 0,
            checksum: 0,
            identifier,
            sequence_number,
            data: vec![0; 32],  // 32バイトのダミーデータ
        }
    }
}