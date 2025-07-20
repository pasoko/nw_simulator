use std::collections::{HashMap, VecDeque};
use serde::{Serialize, Deserialize};
use crate::terminal_device::{TerminalDevice, TerminalDeviceInfo, TerminalConfig};
use crate::device::{ICMPPacket, ICMPType};
use crate::event_manager::EventManager;
use crate::console_log;

/// 端末デバイス管理システム
/// 
/// 複数の独立端末デバイスを管理し、シミュレーションと統合します。
/// ルーターとは独立した端末デバイスのライフサイクル管理、パケット配信、
/// 統計収集などを行います。

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalManager {
    /// 管理している端末デバイス
    terminals: HashMap<u32, TerminalDevice>,
    /// 次に割り当てる端末ID
    next_terminal_id: u32,
    /// パケット配信キュー
    packet_delivery_queue: VecDeque<PacketDelivery>,
    /// 管理者設定
    config: ManagerConfig,
    /// 統計情報
    statistics: ManagerStatistics,
}

/// パケット配信情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketDelivery {
    /// 送信元端末ID
    pub source_terminal_id: u32,
    /// 宛先ルーターID
    pub destination_router_id: u32,
    /// 配信するパケット
    pub packet: ICMPPacket,
    /// 配信予定時刻
    pub delivery_time: f64,
    /// 再試行回数
    pub retry_count: u32,
}

/// マネージャー設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerConfig {
    /// 最大端末数
    pub max_terminals: u32,
    /// パケット配信遅延（秒）
    pub packet_delivery_delay: f64,
    /// 統計更新間隔（秒）
    pub statistics_update_interval: f64,
    /// 自動クリーンアップ間隔（秒）
    pub cleanup_interval: f64,
}

/// マネージャー統計情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerStatistics {
    /// 管理している端末数
    pub total_terminals: u32,
    /// アクティブな端末数
    pub active_terminals: u32,
    /// 障害状態の端末数
    pub failed_terminals: u32,
    /// 総送信パケット数
    pub total_packets_sent: u64,
    /// 総受信パケット数
    pub total_packets_received: u64,
    /// 総ドロップパケット数
    pub total_packets_dropped: u64,
    /// 配信待ちパケット数
    pub queued_packets: u32,
    /// 最後の統計更新時刻
    pub last_update_time: f64,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        ManagerConfig {
            max_terminals: 1000,
            packet_delivery_delay: 0.001, // 1ms
            statistics_update_interval: 10.0,
            cleanup_interval: 60.0,
        }
    }
}

impl Default for ManagerStatistics {
    fn default() -> Self {
        ManagerStatistics {
            total_terminals: 0,
            active_terminals: 0,
            failed_terminals: 0,
            total_packets_sent: 0,
            total_packets_received: 0,
            total_packets_dropped: 0,
            queued_packets: 0,
            last_update_time: 0.0,
        }
    }
}

impl TerminalManager {
    /// 新しい端末マネージャーを作成
    pub fn new() -> Self {
        TerminalManager {
            terminals: HashMap::new(),
            next_terminal_id: 1000, // ルーターIDと区別するため1000から開始
            packet_delivery_queue: VecDeque::new(),
            config: ManagerConfig::default(),
            statistics: ManagerStatistics::default(),
        }
    }
    
    /// カスタム設定で端末マネージャーを作成
    pub fn with_config(config: ManagerConfig) -> Self {
        TerminalManager {
            terminals: HashMap::new(),
            next_terminal_id: 1000,
            packet_delivery_queue: VecDeque::new(),
            config,
            statistics: ManagerStatistics::default(),
        }
    }
    
    /// 新しい端末デバイスを追加
    pub fn add_terminal(
        &mut self,
        name: String,
        ip_address: String,
        netmask: String,
        default_gateway: String,
    ) -> Result<u32, String> {
        if self.terminals.len() >= self.config.max_terminals as usize {
            return Err(format!("Maximum number of terminals ({}) reached", self.config.max_terminals));
        }
        
        // IPアドレスの重複チェック
        for terminal in self.terminals.values() {
            if terminal.ip_address == ip_address {
                return Err(format!("IP address {} is already in use", ip_address));
            }
        }
        
        let terminal_id = self.next_terminal_id;
        self.next_terminal_id += 1;
        
        let terminal = TerminalDevice::new(
            terminal_id,
            name.clone(),
            ip_address.clone(),
            netmask,
            default_gateway,
        );
        
        self.terminals.insert(terminal_id, terminal);
        
        console_log!(
            "Terminal {} added with ID {} (IP: {})",
            name, terminal_id, ip_address
        );
        
        Ok(terminal_id)
    }
    
    /// 端末デバイスを削除
    pub fn remove_terminal(&mut self, terminal_id: u32) -> Result<(), String> {
        if let Some(terminal) = self.terminals.remove(&terminal_id) {
            // 該当端末のパケットを配信キューから削除
            self.packet_delivery_queue.retain(|delivery| {
                delivery.source_terminal_id != terminal_id
            });
            
            console_log!("Terminal {} removed", terminal.name);
            Ok(())
        } else {
            Err(format!("Terminal {} not found", terminal_id))
        }
    }
    
    /// 端末をルーターに接続
    pub fn connect_terminal_to_router(
        &mut self,
        terminal_id: u32,
        router_id: u32,
        interface_id: u32,
    ) -> Result<(), String> {
        let terminal = self.terminals.get_mut(&terminal_id)
            .ok_or_else(|| format!("Terminal {} not found", terminal_id))?;
        
        terminal.connect_to_router(router_id, interface_id);
        Ok(())
    }
    
    /// 端末をルーターから切断
    pub fn disconnect_terminal(&mut self, terminal_id: u32) -> Result<(), String> {
        let terminal = self.terminals.get_mut(&terminal_id)
            .ok_or_else(|| format!("Terminal {} not found", terminal_id))?;
        
        terminal.disconnect_from_router();
        Ok(())
    }
    
    /// 端末からpingを送信
    pub fn send_ping_from_terminal(
        &mut self,
        terminal_id: u32,
        destination_ip: String,
        current_time: f64,
    ) -> Result<u16, String> {
        let terminal = self.terminals.get_mut(&terminal_id)
            .ok_or_else(|| format!("Terminal {} not found", terminal_id))?;
        
        // 端末デバイスに対して ping を開始
        let identifier = 10000 + terminal_id as u16; // 固定のベース値を使用
        terminal.start_ping(destination_ip, 64, identifier, current_time)
    }
    
    /// 端末でICMPパケットを処理
    pub fn process_icmp_packet(
        &mut self,
        terminal_id: u32,
        packet: ICMPPacket,
        current_time: f64,
    ) -> Result<Option<ICMPPacket>, String> {
        let terminal = self.terminals.get_mut(&terminal_id)
            .ok_or_else(|| format!("Terminal {} not found", terminal_id))?;
        
        match packet.packet_type {
            ICMPType::EchoRequest => {
                // Echo Requestを処理してReplyを生成
                Ok(terminal.process_echo_request(&packet))
            }
            ICMPType::EchoReply => {
                // Echo Replyを処理
                terminal.process_echo_reply(&packet, current_time);
                Ok(None)
            }
            _ => {
                // その他のICMPパケットは単に統計のみ更新
                terminal.statistics.packets_received += 1;
                console_log!(
                    "Terminal {} received ICMP packet type {:?}",
                    terminal.name, packet.packet_type
                );
                Ok(None)
            }
        }
    }
    
    /// すべての端末の送信待ちパケットを処理
    pub fn process_all_packet_queues(&mut self, current_time: f64) -> Vec<(u32, ICMPPacket, u32)> {
        let mut all_packets = Vec::new();
        
        for (terminal_id, terminal) in &mut self.terminals {
            let packets = terminal.process_packet_queue(current_time);
            for (packet, router_id) in packets {
                all_packets.push((*terminal_id, packet, router_id));
            }
        }
        
        all_packets
    }
    
    /// 端末の障害状態を設定
    pub fn set_terminal_failed(&mut self, terminal_id: u32, failed: bool) -> Result<(), String> {
        let terminal = self.terminals.get_mut(&terminal_id)
            .ok_or_else(|| format!("Terminal {} not found", terminal_id))?;
        
        terminal.set_failed(failed);
        Ok(())
    }
    
    /// 端末の設定を更新
    pub fn update_terminal_config(
        &mut self,
        terminal_id: u32,
        config: TerminalConfig,
    ) -> Result<(), String> {
        let terminal = self.terminals.get_mut(&terminal_id)
            .ok_or_else(|| format!("Terminal {} not found", terminal_id))?;
        
        terminal.update_config(config);
        Ok(())
    }
    
    /// 端末にARPエントリを追加
    pub fn add_arp_entry_to_terminal(
        &mut self,
        terminal_id: u32,
        ip: String,
        mac: String,
    ) -> Result<(), String> {
        let terminal = self.terminals.get_mut(&terminal_id)
            .ok_or_else(|| format!("Terminal {} not found", terminal_id))?;
        
        terminal.add_arp_entry(&ip, &mac);
        Ok(())
    }
    
    /// 端末にルートエントリを追加
    pub fn add_route_to_terminal(
        &mut self,
        terminal_id: u32,
        destination: String,
        netmask: String,
        gateway: String,
        metric: u32,
        _timestamp: f64,
    ) -> Result<(), String> {
        let terminal = self.terminals.get_mut(&terminal_id)
            .ok_or_else(|| format!("Terminal {} not found", terminal_id))?;
        
        terminal.add_route(&destination, &netmask, &gateway, metric);
        Ok(())
    }
    
    /// 端末デバイス情報を取得
    pub fn get_terminal_info(&self, terminal_id: u32) -> Result<TerminalDeviceInfo, String> {
        let terminal = self.terminals.get(&terminal_id)
            .ok_or_else(|| format!("Terminal {} not found", terminal_id))?;
        
        Ok(terminal.get_device_info())
    }
    
    /// すべての端末の情報を取得
    pub fn get_all_terminals_info(&self) -> Vec<TerminalDeviceInfo> {
        self.terminals.values()
            .map(|terminal| terminal.get_device_info())
            .collect()
    }
    
    /// 指定IPアドレスを持つ端末を検索
    pub fn find_terminal_by_ip(&self, ip_address: &str) -> Option<u32> {
        for (terminal_id, terminal) in &self.terminals {
            if terminal.ip_address == ip_address {
                return Some(*terminal_id);
            }
        }
        None
    }
    
    /// 統計情報を更新
    pub fn update_statistics(&mut self, current_time: f64) {
        if current_time - self.statistics.last_update_time < self.config.statistics_update_interval {
            return;
        }
        
        let mut total_packets_sent = 0;
        let mut total_packets_received = 0;
        let mut total_packets_dropped = 0;
        let mut active_terminals = 0;
        let mut failed_terminals = 0;
        
        for terminal in self.terminals.values() {
            total_packets_sent += terminal.statistics.packets_sent;
            total_packets_received += terminal.statistics.packets_received;
            total_packets_dropped += terminal.statistics.packets_dropped;
            
            if terminal.is_failed {
                failed_terminals += 1;
            } else {
                active_terminals += 1;
            }
        }
        
        self.statistics.total_terminals = self.terminals.len() as u32;
        self.statistics.active_terminals = active_terminals;
        self.statistics.failed_terminals = failed_terminals;
        self.statistics.total_packets_sent = total_packets_sent;
        self.statistics.total_packets_received = total_packets_received;
        self.statistics.total_packets_dropped = total_packets_dropped;
        self.statistics.queued_packets = self.packet_delivery_queue.len() as u32;
        self.statistics.last_update_time = current_time;
    }
    
    /// 定期クリーンアップを実行
    pub fn perform_cleanup(&mut self, current_time: f64) {
        // パケット配信キューの古いエントリを削除
        self.packet_delivery_queue.retain(|delivery| {
            current_time - delivery.delivery_time < 300.0 // 5分以内
        });
        
        console_log!(
            "Terminal manager cleanup completed at {:.2}s",
            current_time
        );
    }
    
    /// マネージャー設定を取得
    pub fn get_config(&self) -> &ManagerConfig {
        &self.config
    }
    
    /// マネージャー設定を更新
    pub fn update_config(&mut self, config: ManagerConfig) {
        self.config = config;
        console_log!("Terminal manager configuration updated");
    }
    
    /// 統計情報を取得
    pub fn get_statistics(&self) -> &ManagerStatistics {
        &self.statistics
    }
    
    /// 統計情報をリセット
    pub fn reset_statistics(&mut self) {
        self.statistics = ManagerStatistics::default();
        
        // 各端末の統計もリセット
        for terminal in self.terminals.values_mut() {
            terminal.reset_statistics();
        }
        
        console_log!("Terminal manager statistics reset");
    }
    
    /// イベントログに端末関連イベントを記録
    pub fn log_terminal_events(&self, _event_manager: &mut EventManager, _current_time: f64) {
        // 端末統計の重要な変化をログに記録
        if self.statistics.failed_terminals > 0 {
            console_log!(
                "Terminal status: {}/{} terminals failed",
                self.statistics.failed_terminals,
                self.statistics.total_terminals
            );
        }
        
        if self.statistics.queued_packets > 100 {
            console_log!(
                "High packet queue: {} packets pending delivery",
                self.statistics.queued_packets
            );
        }
    }
    
    /// デバッグ情報を出力
    pub fn debug_info(&self) -> String {
        format!(
            "TerminalManager: {} terminals ({} active, {} failed), {} queued packets",
            self.statistics.total_terminals,
            self.statistics.active_terminals,
            self.statistics.failed_terminals,
            self.statistics.queued_packets
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_manager_creation() {
        let manager = TerminalManager::new();
        assert_eq!(manager.terminals.len(), 0);
        assert_eq!(manager.next_terminal_id, 1000);
    }
    
    #[test]
    fn test_add_terminal() {
        let mut manager = TerminalManager::new();
        
        let result = manager.add_terminal(
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );
        
        assert!(result.is_ok());
        let terminal_id = result.unwrap();
        assert_eq!(terminal_id, 1000);
        assert_eq!(manager.terminals.len(), 1);
    }
    
    #[test]
    fn test_duplicate_ip_detection() {
        let mut manager = TerminalManager::new();
        
        // 最初の端末を追加
        let result1 = manager.add_terminal(
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );
        assert!(result1.is_ok());
        
        // 同じIPアドレスで2番目の端末を追加（失敗するはず）
        let result2 = manager.add_terminal(
            "Host2".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        );
        assert!(result2.is_err());
    }
    
    #[test]
    fn test_terminal_connection() {
        let mut manager = TerminalManager::new();
        
        let terminal_id = manager.add_terminal(
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        ).unwrap();
        
        let result = manager.connect_terminal_to_router(terminal_id, 100, 1);
        assert!(result.is_ok());
        
        let terminal = manager.terminals.get(&terminal_id).unwrap();
        assert_eq!(terminal.connected_router_id, Some(100));
    }
    
    #[test]
    fn test_ping_from_terminal() {
        let mut manager = TerminalManager::new();
        
        let terminal_id = manager.add_terminal(
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        ).unwrap();
        
        manager.connect_terminal_to_router(terminal_id, 100, 1).unwrap();
        
        let result = manager.send_ping_from_terminal(
            terminal_id,
            "8.8.8.8".to_string(),
            0.0,
        );
        
        assert!(result.is_ok());
        
        let terminal = manager.terminals.get(&terminal_id).unwrap();
        assert_eq!(terminal.statistics.icmp_echo_requests_sent, 1);
    }
    
    #[test]
    fn test_statistics_update() {
        let mut manager = TerminalManager::new();
        
        // 端末を追加
        let _terminal_id = manager.add_terminal(
            "Host1".to_string(),
            "192.168.1.10".to_string(),
            "255.255.255.0".to_string(),
            "192.168.1.1".to_string(),
        ).unwrap();
        
        // 統計を更新
        manager.update_statistics(10.0);
        
        assert_eq!(manager.statistics.total_terminals, 1);
        assert_eq!(manager.statistics.active_terminals, 1);
        assert_eq!(manager.statistics.failed_terminals, 0);
    }
}