use std::collections::{HashMap, VecDeque};
use serde::{Serialize, Deserialize};
use crate::device::{ICMPPacket, ICMPType};
use crate::console_log;

/// 独立端末デバイス
/// 
/// OSPFv2準拠のネットワークシミュレーションにおける独立した端末デバイスの実装。
/// ルーターとは独立して動作し、より現実的な端末デバイスの動作をシミュレートします。
/// 
/// 主な機能:
/// - 独立したIP設定とルーティング判断
/// - ARPテーブル管理
/// - ICMP Echo Request/Reply処理
/// - パケット送信キューと再送機能
/// - ネットワーク到達性の検証

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalDevice {
    /// デバイスID
    pub id: u32,
    /// デバイス名
    pub name: String,
    /// IPアドレス
    pub ip_address: String,
    /// ネットマスク
    pub netmask: String,
    /// デフォルトゲートウェイ
    pub default_gateway: String,
    /// MACアドレス（シミュレーション用）
    pub mac_address: String,
    /// 接続されたルーターID
    pub connected_router_id: Option<u32>,
    /// 接続されたルーターのインターフェースID
    pub connected_interface_id: Option<u32>,
    /// ARPテーブル (IP -> MAC)
    pub arp_table: HashMap<String, String>,
    /// ルーティングテーブル（学習した経路）
    pub routing_table: Vec<RouteEntry>,
    /// 送信待ちパケットキュー
    pub packet_queue: VecDeque<QueuedPacket>,
    /// デバイスの障害状態
    pub is_failed: bool,
    /// 統計情報
    pub statistics: TerminalStatistics,
    /// 設定パラメータ
    pub config: TerminalConfig,
}

/// ルーティングエントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteEntry {
    pub destination: String,
    pub netmask: String,
    pub gateway: String,
    pub metric: u32,
    pub timestamp: f64,
    pub is_default: bool,
}

/// 送信待ちパケット
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedPacket {
    pub packet: ICMPPacket,
    pub destination_ip: String,
    pub retry_count: u32,
    pub next_retry_time: f64,
    pub creation_time: f64,
}

/// 端末統計情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalStatistics {
    pub packets_sent: u64,
    pub packets_received: u64,
    pub packets_dropped: u64,
    pub icmp_echo_requests_sent: u64,
    pub icmp_echo_replies_received: u64,
    pub icmp_echo_replies_sent: u64,
    pub arp_requests_sent: u64,
    pub arp_replies_received: u64,
    pub route_lookup_successes: u64,
    pub route_lookup_failures: u64,
}

/// 端末設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    /// パケット再送回数
    pub max_retries: u32,
    /// 再送間隔（秒）
    pub retry_interval: f64,
    /// パケットタイムアウト（秒）
    pub packet_timeout: f64,
    /// ARPキャッシュのタイムアウト（秒）
    pub arp_timeout: f64,
    /// 最大キューサイズ
    pub max_queue_size: usize,
    /// ICMP識別子の範囲
    pub icmp_id_base: u16,
}

impl Default for TerminalStatistics {
    fn default() -> Self {
        TerminalStatistics {
            packets_sent: 0,
            packets_received: 0,
            packets_dropped: 0,
            icmp_echo_requests_sent: 0,
            icmp_echo_replies_received: 0,
            icmp_echo_replies_sent: 0,
            arp_requests_sent: 0,
            arp_replies_received: 0,
            route_lookup_successes: 0,
            route_lookup_failures: 0,
        }
    }
}

impl Default for TerminalConfig {
    fn default() -> Self {
        TerminalConfig {
            max_retries: 3,
            retry_interval: 1.0,
            packet_timeout: 30.0,
            arp_timeout: 300.0,
            max_queue_size: 100,
            icmp_id_base: 1000,
        }
    }
}

impl TerminalDevice {
    /// 新しい端末デバイスを作成
    pub fn new(
        id: u32,
        name: String,
        ip_address: String,
        netmask: String,
        default_gateway: String,
    ) -> Self {
        let mac_address = Self::generate_mac_address(id);
        
        let mut device = TerminalDevice {
            id,
            name: name.clone(),
            ip_address: ip_address.clone(),
            netmask: netmask.clone(),
            default_gateway: default_gateway.clone(),
            mac_address,
            connected_router_id: None,
            connected_interface_id: None,
            arp_table: HashMap::new(),
            routing_table: Vec::new(),
            packet_queue: VecDeque::new(),
            is_failed: false,
            statistics: TerminalStatistics::default(),
            config: TerminalConfig::default(),
        };
        
        // デフォルトルートを追加
        device.add_default_route(default_gateway.clone());
        
        console_log!(
            "Terminal device {} created: IP={}, Gateway={}",
            name, ip_address, default_gateway
        );
        
        device
    }
    
    /// MACアドレスを生成（シミュレーション用）
    fn generate_mac_address(id: u32) -> String {
        format!("02:00:00:00:{:02x}:{:02x}", (id >> 8) & 0xff, id & 0xff)
    }
    
    /// デフォルトルートを追加
    fn add_default_route(&mut self, gateway: String) {
        let route = RouteEntry {
            destination: "0.0.0.0".to_string(),
            netmask: "0.0.0.0".to_string(),
            gateway,
            metric: 1,
            timestamp: 0.0,
            is_default: true,
        };
        self.routing_table.push(route);
    }
    
    /// ルーターに接続
    pub fn connect_to_router(&mut self, router_id: u32, interface_id: u32) {
        self.connected_router_id = Some(router_id);
        self.connected_interface_id = Some(interface_id);
        
        console_log!(
            "Terminal device {} connected to router {} (interface {})",
            self.name, router_id, interface_id
        );
    }
    
    /// ルーターから切断
    pub fn disconnect_from_router(&mut self) {
        if let Some(router_id) = self.connected_router_id {
            console_log!(
                "Terminal device {} disconnected from router {}",
                self.name, router_id
            );
        }
        
        self.connected_router_id = None;
        self.connected_interface_id = None;
    }
    
    /// 宛先IPアドレスが同一サブネットかチェック
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
    
    /// 次ホップを決定
    pub fn resolve_next_hop(&mut self, destination_ip: &str) -> Option<String> {
        // 1. 同一サブネットの場合は直接配信
        if self.is_same_subnet(destination_ip) {
            self.statistics.route_lookup_successes = self.statistics.route_lookup_successes.wrapping_add(1);
            return Some(destination_ip.to_string());
        }
        
        // 2. 特定ルートを検索
        for route in &self.routing_table {
            if !route.is_default && self.route_matches(&route.destination, &route.netmask, destination_ip) {
                self.statistics.route_lookup_successes = self.statistics.route_lookup_successes.wrapping_add(1);
                return Some(route.gateway.clone());
            }
        }
        
        // 3. デフォルトルートを使用
        for route in &self.routing_table {
            if route.is_default {
                self.statistics.route_lookup_successes = self.statistics.route_lookup_successes.wrapping_add(1);
                return Some(route.gateway.clone());
            }
        }
        
        self.statistics.route_lookup_failures = self.statistics.route_lookup_failures.wrapping_add(1);
        None
    }
    
    /// ルートがマッチするかチェック
    fn route_matches(&self, network: &str, netmask: &str, target_ip: &str) -> bool {
        let network_parts: Vec<u8> = network.split('.')
            .filter_map(|s| s.parse().ok())
            .collect();
        let mask_parts: Vec<u8> = netmask.split('.')
            .filter_map(|s| s.parse().ok())
            .collect();
        let target_parts: Vec<u8> = target_ip.split('.')
            .filter_map(|s| s.parse().ok())
            .collect();

        if network_parts.len() != 4 || mask_parts.len() != 4 || target_parts.len() != 4 {
            return false;
        }

        for i in 0..4 {
            if (network_parts[i] & mask_parts[i]) != (target_parts[i] & mask_parts[i]) {
                return false;
            }
        }

        true
    }
    
    /// Echo Requestを送信
    pub fn send_ping(&mut self, destination_ip: String, current_time: f64) -> Result<u16, String> {
        if self.is_failed {
            return Err("Terminal device is failed".to_string());
        }
        
        if self.connected_router_id.is_none() {
            return Err("Terminal device is not connected to any router".to_string());
        }
        
        // キューが満杯かチェック
        if self.packet_queue.len() >= self.config.max_queue_size {
            self.statistics.packets_dropped += 1;
            return Err("Packet queue is full".to_string());
        }
        
        // 次ホップを解決
        let next_hop = self.resolve_next_hop(&destination_ip)
            .ok_or_else(|| "No route to destination".to_string())?;
        
        // ICMP Echo Requestパケットを作成
        let identifier = self.config.icmp_id_base + (self.statistics.icmp_echo_requests_sent as u16 % 1000);
        let sequence_number = 1;
        
        let icmp_packet = ICMPPacket::new_echo_request(identifier, sequence_number)
            .with_addresses(self.ip_address.clone(), destination_ip.clone());
        
        // パケットをキューに追加
        let queued_packet = QueuedPacket {
            packet: icmp_packet,
            destination_ip: destination_ip.clone(),
            retry_count: 0,
            next_retry_time: current_time,
            creation_time: current_time,
        };
        
        self.packet_queue.push_back(queued_packet);
        self.statistics.icmp_echo_requests_sent += 1;
        self.statistics.packets_sent += 1;
        
        console_log!(
            "Terminal {} queued ping to {} via {} (ID: {})",
            self.name, destination_ip, next_hop, identifier
        );
        
        Ok(identifier)
    }
    
    /// Echo Replyを処理
    pub fn process_echo_reply(&mut self, packet: &ICMPPacket, _current_time: f64) -> bool {
        if self.is_failed {
            return false;
        }
        
        // 自分宛のパケットかチェック
        if packet.destination_ip != self.ip_address {
            return false;
        }
        
        self.statistics.packets_received += 1;
        self.statistics.icmp_echo_replies_received += 1;
        
        console_log!(
            "Terminal {} received echo reply from {} (ID: {}, Seq: {})",
            self.name, packet.source_ip, packet.identifier, packet.sequence_number
        );
        
        // 対応するEcho Requestを送信キューから削除
        self.packet_queue.retain(|queued| {
            !(queued.packet.packet_type == ICMPType::EchoRequest && 
              queued.packet.identifier == packet.identifier &&
              queued.packet.sequence_number == packet.sequence_number)
        });
        
        true
    }
    
    /// Echo Requestを処理してReplyを生成
    pub fn process_echo_request(&mut self, packet: &ICMPPacket) -> Option<ICMPPacket> {
        if self.is_failed {
            return None;
        }
        
        // 自分宛のパケットかチェック
        if packet.destination_ip != self.ip_address {
            return None;
        }
        
        self.statistics.packets_received += 1;
        
        // Echo Replyを生成
        let reply = ICMPPacket::new_echo_reply(packet.identifier, packet.sequence_number)
            .with_addresses(self.ip_address.clone(), packet.source_ip.clone());
        
        self.statistics.icmp_echo_replies_sent += 1;
        self.statistics.packets_sent += 1;
        
        console_log!(
            "Terminal {} sent echo reply to {} (ID: {}, Seq: {})",
            self.name, packet.source_ip, packet.identifier, packet.sequence_number
        );
        
        Some(reply)
    }
    
    /// 送信待ちパケットを処理
    pub fn process_packet_queue(&mut self, current_time: f64) -> Vec<(ICMPPacket, u32)> {
        let mut packets_to_send = Vec::new();
        let mut expired_packets = Vec::new();
        
        for (index, queued_packet) in self.packet_queue.iter_mut().enumerate() {
            // タイムアウトチェック
            if current_time - queued_packet.creation_time > self.config.packet_timeout {
                expired_packets.push(index);
                continue;
            }
            
            // 再送時間チェック
            if current_time >= queued_packet.next_retry_time {
                if queued_packet.retry_count < self.config.max_retries {
                    // 再送
                    if let Some(router_id) = self.connected_router_id {
                        packets_to_send.push((queued_packet.packet.clone(), router_id));
                        queued_packet.retry_count += 1;
                        queued_packet.next_retry_time = current_time + self.config.retry_interval;
                        
                        console_log!(
                            "Terminal {} retrying packet to {} (attempt {})",
                            self.name, queued_packet.destination_ip, queued_packet.retry_count
                        );
                    }
                } else {
                    // 最大再送回数に達した
                    expired_packets.push(index);
                    self.statistics.packets_dropped += 1;
                    
                    console_log!(
                        "Terminal {} dropping packet to {} after {} retries",
                        self.name, queued_packet.destination_ip, queued_packet.retry_count
                    );
                }
            }
        }
        
        // 期限切れパケットを削除（逆順で削除してインデックスを保持）
        for index in expired_packets.into_iter().rev() {
            self.packet_queue.remove(index);
        }
        
        packets_to_send
    }
    
    /// ARPエントリを追加
    pub fn add_arp_entry(&mut self, ip: String, mac: String) {
        self.arp_table.insert(ip.clone(), mac.clone());
        console_log!("Terminal {} learned ARP: {} -> {}", self.name, ip, mac);
    }
    
    /// ARPエントリを検索
    pub fn lookup_arp(&self, ip: &str) -> Option<&String> {
        self.arp_table.get(ip)
    }
    
    /// ルートエントリを追加
    pub fn add_route(&mut self, destination: String, netmask: String, gateway: String, metric: u32, timestamp: f64) {
        let route = RouteEntry {
            destination,
            netmask,
            gateway,
            metric,
            timestamp,
            is_default: false,
        };
        
        console_log!(
            "Terminal {} learned route: {}/{} via {} (metric {})",
            self.name, route.destination, route.netmask, route.gateway, metric
        );
        self.routing_table.push(route);
    }
    
    /// 障害状態を設定
    pub fn set_failed(&mut self, failed: bool) {
        if self.is_failed != failed {
            self.is_failed = failed;
            
            if failed {
                // 障害時はキューをクリア
                self.packet_queue.clear();
                console_log!("Terminal {} failed", self.name);
            } else {
                console_log!("Terminal {} recovered", self.name);
            }
        }
    }
    
    /// 統計情報をリセット
    pub fn reset_statistics(&mut self) {
        self.statistics = TerminalStatistics::default();
        console_log!("Terminal {} statistics reset", self.name);
    }
    
    /// 設定を更新
    pub fn update_config(&mut self, config: TerminalConfig) {
        self.config = config;
        console_log!("Terminal {} configuration updated", self.name);
    }
    
    /// デバイス情報を取得
    pub fn get_device_info(&self) -> TerminalDeviceInfo {
        TerminalDeviceInfo {
            id: self.id,
            name: self.name.clone(),
            ip_address: self.ip_address.clone(),
            netmask: self.netmask.clone(),
            default_gateway: self.default_gateway.clone(),
            mac_address: self.mac_address.clone(),
            connected_router_id: self.connected_router_id,
            is_failed: self.is_failed,
            queue_size: self.packet_queue.len(),
            arp_entries: self.arp_table.len(),
            route_entries: self.routing_table.len(),
            statistics: self.statistics.clone(),
        }
    }
}

/// 端末デバイス情報（シリアライゼーション用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalDeviceInfo {
    pub id: u32,
    pub name: String,
    pub ip_address: String,
    pub netmask: String,
    pub default_gateway: String,
    pub mac_address: String,
    pub connected_router_id: Option<u32>,
    pub is_failed: bool,
    pub queue_size: usize,
    pub arp_entries: usize,
    pub route_entries: usize,
    pub statistics: TerminalStatistics,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_device_creation() {
        let device = TerminalDevice::new(
            1,
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );
        
        assert_eq!(device.id, 1);
        assert_eq!(device.name, "Host1");
        assert_eq!(device.ip_address, "192.168.1.10");
        assert_eq!(device.default_gateway, "192.168.1.1");
        assert_eq!(device.routing_table.len(), 1); // Default route
        assert!(device.routing_table[0].is_default);
    }
    
    #[test]
    fn test_same_subnet_check() {
        let device = TerminalDevice::new(
            1,
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );
        
        assert!(device.is_same_subnet("192.168.1.1"));
        assert!(device.is_same_subnet("192.168.1.255"));
        assert!(!device.is_same_subnet("192.168.2.1"));
        assert!(!device.is_same_subnet("10.0.0.1"));
    }
    
    #[test]
    fn test_next_hop_resolution() {
        let mut device = TerminalDevice::new(
            1,
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );
        
        // Same subnet
        assert_eq!(device.resolve_next_hop("192.168.1.20"), Some("192.168.1.20".to_string()));
        
        // Different subnet (use default gateway)
        assert_eq!(device.resolve_next_hop("10.0.0.1"), Some("192.168.1.1".to_string()));
    }
    
    #[test]
    fn test_ping_sending() {
        let mut device = TerminalDevice::new(
            1,
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );
        
        device.connect_to_router(100, 1);
        
        let result = device.send_ping("8.8.8.8".to_string(), 0.0);
        assert!(result.is_ok());
        assert_eq!(device.packet_queue.len(), 1);
        assert_eq!(device.statistics.icmp_echo_requests_sent, 1);
    }
    
    #[test]
    fn test_echo_reply_processing() {
        let mut device = TerminalDevice::new(
            1,
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );
        
        let packet = ICMPPacket::new_echo_reply(1000, 1)
            .with_addresses("8.8.8.8".to_string(), "192.168.1.10".to_string());
        
        let processed = device.process_echo_reply(&packet, 0.0);
        assert!(processed);
        assert_eq!(device.statistics.icmp_echo_replies_received, 1);
    }
}