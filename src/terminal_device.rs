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
            icmp_id_base: 10000,
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
        let mac_address = format!("00:00:00:00:00:{:02x}", id);
        
        // デフォルトルートを設定
        let mut routing_table = Vec::new();
        routing_table.push(RouteEntry {
            destination: "0.0.0.0".to_string(),
            netmask: "0.0.0.0".to_string(),
            gateway: default_gateway.clone(),
            metric: 1,
            timestamp: 0.0,
            is_default: true,
        });
        
        console_log!(
            "Created terminal device {} with IP {} (GW: {})",
            name, ip_address, default_gateway
        );
        
        TerminalDevice {
            id,
            name,
            ip_address,
            netmask,
            default_gateway,
            mac_address,
            connected_router_id: None,
            connected_interface_id: None,
            arp_table: HashMap::new(),
            routing_table,
            packet_queue: VecDeque::new(),
            is_failed: false,
            statistics: TerminalStatistics::default(),
            config: TerminalConfig::default(),
        }
    }
    
    /// JSONとしてシリアライズ
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
    
    /// ルーターに接続
    pub fn connect_to_router(&mut self, router_id: u32, interface_id: u32) {
        self.connected_router_id = Some(router_id);
        self.connected_interface_id = Some(interface_id);
        
        console_log!(
            "Terminal device {} connected to router {} interface {}",
            self.name, router_id, interface_id
        );
    }
    
    /// ルーターから切断
    pub fn disconnect(&mut self) {
        if let Some(router_id) = self.connected_router_id {
            console_log!(
                "Terminal device {} disconnected from router {}",
                self.name, router_id
            );
        }
        
        self.connected_router_id = None;
        self.connected_interface_id = None;
    }
    
    /// ルーターから切断（互換性のため）
    pub fn disconnect_from_router(&mut self) {
        self.disconnect();
    }
    
    /// 宛先IPアドレスが同一サブネットかチェック
    pub fn is_same_subnet(&self, target_ip: &str) -> bool {
        self.is_in_same_subnet(target_ip)
    }
    
    /// 宛先IPアドレスが同一サブネットかチェック（別名）
    pub fn is_in_same_subnet(&self, target_ip: &str) -> bool {
        let self_ip = ip_to_u32(&self.ip_address);
        let target_ip = ip_to_u32(target_ip);
        let mask = ip_to_u32(&self.netmask);
        
        (self_ip & mask) == (target_ip & mask)
    }
    
    /// ルートがマッチするかチェック
    fn route_matches(&self, route_dest: &str, route_mask: &str, target_ip: &str) -> bool {
        let route_net = ip_to_u32(route_dest);
        let mask = ip_to_u32(route_mask);
        let target = ip_to_u32(target_ip);
        
        (target & mask) == (route_net & mask)
    }
    
    /// ルートを検索
    pub fn lookup_route(&self, destination_ip: &str) -> Option<RouteEntry> {
        // 同一サブネットの場合は直接ルート
        if self.is_in_same_subnet(destination_ip) {
            // 直接配信用の仮想ルートを返す
            return Some(RouteEntry {
                destination: destination_ip.to_string(),
                netmask: self.netmask.clone(),
                gateway: "direct".to_string(),
                metric: 0,
                timestamp: 0.0,
                is_default: false,
            });
        }
        
        // 最も特定的なルートを検索
        let mut best_route: Option<&RouteEntry> = None;
        let mut best_mask_bits = 0u32;
        
        for route in &self.routing_table {
            if self.route_matches(&route.destination, &route.netmask, destination_ip) {
                let mask_bits = ip_to_u32(&route.netmask).count_ones();
                if mask_bits > best_mask_bits || best_route.is_none() {
                    best_route = Some(route);
                    best_mask_bits = mask_bits;
                }
            }
        }
        
        best_route.cloned()
    }
    
    /// 次ホップを決定
    pub fn resolve_next_hop(&self, destination_ip: &str) -> Option<String> {
        match self.lookup_route(destination_ip) {
            Some(route) => {
                if route.gateway == "direct" {
                    Some(destination_ip.to_string())
                } else {
                    Some(route.gateway.clone())
                }
            }
            None => None,
        }
    }
    
    /// 到達可能かチェック
    pub fn can_reach(&self, destination_ip: &str) -> bool {
        // 障害中は到達不可
        if self.is_failed {
            return false;
        }
        
        // 同一サブネットは常に到達可能（接続状態に関わらず）
        if self.is_in_same_subnet(destination_ip) {
            return true;
        }
        
        // 他のネットワークへは接続されていることが必要
        self.connected_router_id.is_some() && self.lookup_route(destination_ip).is_some()
    }
    
    /// Echo Request パケットを作成
    pub fn create_echo_request(&self, destination_ip: &str, ttl: u8, identifier: u16, sequence_number: u16) -> ICMPPacket {
        ICMPPacket {
            packet_type: ICMPType::EchoRequest,
            code: 0,
            checksum: 0,
            identifier,
            sequence_number,
            data: vec![0; 56], // 標準的なpingデータサイズ
            source_ip: self.ip_address.clone(),
            destination_ip: destination_ip.to_string(),
            ttl,
            original_packet: None,
        }
    }
    
    /// Echo Requestを処理してReplyを生成
    pub fn process_echo_request(&mut self, request: &ICMPPacket) -> Option<ICMPPacket> {
        // 自分宛でなければ無視
        if request.destination_ip != self.ip_address {
            return None;
        }
        
        // 障害中は応答しない
        if self.is_failed {
            return None;
        }
        
        self.statistics.packets_received += 1;
        self.statistics.icmp_echo_replies_sent += 1;
        
        // Echo Replyを生成
        Some(ICMPPacket {
            packet_type: ICMPType::EchoReply,
            code: 0,
            checksum: 0,
            identifier: request.identifier,
            sequence_number: request.sequence_number,
            data: request.data.clone(),
            source_ip: self.ip_address.clone(),
            destination_ip: request.source_ip.clone(),
            ttl: 64,
            original_packet: None,
        })
    }
    
    /// Echo Replyを受信
    pub fn receive_echo_reply(&mut self, reply: &ICMPPacket) {
        if reply.destination_ip == self.ip_address && reply.packet_type == ICMPType::EchoReply {
            self.statistics.packets_received += 1;
            self.statistics.icmp_echo_replies_received += 1;
        }
    }
    
    /// パケットをキューに追加
    pub fn queue_packet(&mut self, packet: ICMPPacket, destination_ip: &str) -> bool {
        if self.packet_queue.len() >= self.config.max_queue_size {
            self.statistics.packets_dropped += 1;
            return false;
        }
        
        let queued_packet = QueuedPacket {
            packet,
            destination_ip: destination_ip.to_string(),
            retry_count: 0,
            next_retry_time: 0.0,
            creation_time: 0.0,
        };
        
        self.packet_queue.push_back(queued_packet);
        true
    }
    
    /// パケットキューを処理
    pub fn process_packet_queue(&mut self, current_time: f64) -> Vec<(ICMPPacket, u32)> {
        let mut sent_packets = Vec::new();
        let mut i = 0;
        
        while i < self.packet_queue.len() {
            let mut should_remove = false;
            let mut should_send = false;
            
            {
                let queued = &self.packet_queue[i];
                
                // 再送時刻をチェック
                if current_time >= queued.next_retry_time {
                    should_send = true;
                }
            }
            
            if should_send {
                let mut queued = self.packet_queue[i].clone();
                
                // 送信可能かチェック
                if self.can_reach(&queued.destination_ip) && self.connected_router_id.is_some() {
                    // パケットを送信
                    sent_packets.push((queued.packet.clone(), self.connected_router_id.unwrap()));
                    self.statistics.packets_sent += 1;
                    should_remove = true;
                } else {
                    // 送信失敗、再試行
                    queued.retry_count += 1;
                    
                    if queued.retry_count > self.config.max_retries {
                        // 最大再試行回数を超えた
                        self.statistics.packets_dropped += 1;
                        should_remove = true;
                    } else {
                        // 次の再試行時刻を設定
                        queued.next_retry_time = current_time + self.config.retry_interval;
                        self.packet_queue[i] = queued;
                    }
                }
            }
            
            if should_remove {
                self.packet_queue.remove(i);
            } else {
                i += 1;
            }
        }
        
        sent_packets
    }
    
    /// 期限切れパケットをクリア
    pub fn clear_expired_packets(&mut self, current_time: f64) {
        let timeout = self.config.packet_timeout;
        let mut i = 0;
        
        while i < self.packet_queue.len() {
            if current_time - self.packet_queue[i].creation_time > timeout {
                self.packet_queue.remove(i);
                self.statistics.packets_dropped += 1;
            } else {
                i += 1;
            }
        }
    }
    
    /// Pingを開始
    pub fn start_ping(
        &mut self,
        destination_ip: String,
        ttl: u8,
        identifier: u16,
        _current_time: f64,
    ) -> Result<u16, String> {
        // 宛先に到達可能かチェック
        if !self.can_reach(&destination_ip) {
            self.statistics.route_lookup_failures += 1;
            return Err("Destination unreachable".to_string());
        }
        
        // Echo Requestパケットを作成してキューに追加
        let packet = self.create_echo_request(&destination_ip, ttl, identifier, 1);
        
        if self.queue_packet(packet, &destination_ip) {
            self.statistics.icmp_echo_requests_sent += 1;
            Ok(identifier)
        } else {
            Err("Packet queue is full".to_string())
        }
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
        
        // Echo Replyの場合
        if packet.packet_type == ICMPType::EchoReply {
            self.statistics.packets_received += 1;
            self.statistics.icmp_echo_replies_received += 1;
            
            console_log!(
                "Terminal {} received echo reply from {} (ID: {}, Seq: {})",
                self.name, packet.source_ip, packet.identifier, packet.sequence_number
            );
            
            return true;
        }
        
        false
    }
    
    /// ARPエントリを追加
    pub fn add_arp_entry(&mut self, ip: &str, mac: &str) {
        self.arp_table.insert(ip.to_string(), mac.to_string());
        self.statistics.arp_replies_received += 1;
        
        console_log!(
            "Terminal {} learned ARP: {} -> {}",
            self.name, ip, mac
        );
    }
    
    /// IPアドレスからMACアドレスを取得
    pub fn get_mac_for_ip(&self, ip: &str) -> Option<String> {
        self.arp_table.get(ip).cloned()
    }
    
    /// ARPエントリを検索
    pub fn lookup_arp(&self, ip: &str) -> Option<&String> {
        self.arp_table.get(ip)
    }
    
    /// ルートエントリを追加
    pub fn add_route(&mut self, destination: &str, netmask: &str, gateway: &str, metric: u32) {
        let route = RouteEntry {
            destination: destination.to_string(),
            netmask: netmask.to_string(),
            gateway: gateway.to_string(),
            metric,
            timestamp: 0.0,
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

/// IPアドレスをu32に変換
pub fn ip_to_u32(ip: &str) -> u32 {
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() != 4 {
        return 0;
    }
    
    let mut result: u32 = 0;
    for (i, part) in parts.iter().enumerate() {
        match part.parse::<u8>() {
            Ok(byte) => {
                result |= (byte as u32) << (24 - i * 8);
            }
            Err(_) => return 0,
        }
    }
    
    result
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
        assert_eq!(device.netmask, "255.255.255.0");
        assert_eq!(device.default_gateway, "192.168.1.1");
        assert_eq!(device.mac_address, "00:00:00:00:00:01");
        assert!(!device.is_failed);
        assert_eq!(device.routing_table.len(), 1);
        assert!(device.routing_table[0].is_default);
    }

    #[test]
    fn test_is_same_subnet() {
        let device = TerminalDevice::new(
            1,
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );
        
        assert!(device.is_same_subnet("192.168.1.20"));
        assert!(device.is_same_subnet("192.168.1.1"));
        assert!(device.is_same_subnet("192.168.1.254"));
        assert!(!device.is_same_subnet("192.168.2.10"));
        assert!(!device.is_same_subnet("10.0.0.1"));
    }

    #[test]
    fn test_resolve_next_hop() {
        let mut device = TerminalDevice::new(
            1,
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );
        
        // 同一サブネット宛は直接配信
        assert_eq!(device.resolve_next_hop("192.168.1.20"), Some("192.168.1.20".to_string()));
        
        // 異なるサブネット宛はデフォルトゲートウェイ経由
        assert_eq!(device.resolve_next_hop("10.0.0.1"), Some("192.168.1.1".to_string()));
        
        // 特定ルートを追加
        device.add_route("10.0.0.0", "255.255.255.0", "192.168.1.254", 10);
        
        // 特定ルートが優先される
        assert_eq!(device.resolve_next_hop("10.0.0.1"), Some("192.168.1.254".to_string()));
    }

    #[test]
    fn test_process_echo_request() {
        let mut device = TerminalDevice::new(
            1,
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );
        
        // 自分宛のEcho Request
        let request = ICMPPacket {
            packet_type: ICMPType::EchoRequest,
            code: 0,
            checksum: 0,
            identifier: 1234,
            sequence_number: 1,
            data: vec![0; 56],
            source_ip: "192.168.1.20".to_string(),
            destination_ip: "192.168.1.10".to_string(),
            ttl: 64,
            original_packet: None,
        };
        
        let reply = device.process_echo_request(&request);
        assert!(reply.is_some());
        
        let reply_packet = reply.unwrap();
        assert_eq!(reply_packet.source_ip, "192.168.1.10");
        assert_eq!(reply_packet.destination_ip, "192.168.1.20");
        assert_eq!(reply_packet.packet_type, ICMPType::EchoReply);
        assert_eq!(reply_packet.identifier, 1234);
        assert_eq!(reply_packet.sequence_number, 1);
        
        // 他の宛先へのEcho Request（無視される）
        let other_request = ICMPPacket {
            packet_type: ICMPType::EchoRequest,
            code: 0,
            checksum: 0,
            identifier: 1235,
            sequence_number: 1,
            data: vec![0; 56],
            source_ip: "192.168.1.20".to_string(),
            destination_ip: "192.168.1.11".to_string(),
            ttl: 64,
            original_packet: None,
        };
        
        let no_reply = device.process_echo_request(&other_request);
        assert!(no_reply.is_none());
    }

    #[test]
    fn test_ip_to_u32() {
        assert_eq!(ip_to_u32("192.168.1.1"), 0xC0A80101);
        assert_eq!(ip_to_u32("10.0.0.1"), 0x0A000001);
        assert_eq!(ip_to_u32("255.255.255.255"), 0xFFFFFFFF);
        assert_eq!(ip_to_u32("0.0.0.0"), 0x00000000);
        assert_eq!(ip_to_u32("invalid"), 0);
        assert_eq!(ip_to_u32("256.0.0.1"), 0);
    }
}