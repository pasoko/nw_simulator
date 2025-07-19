use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};
use crate::network_type::OSPFNetworkType;
use crate::router::InterfaceConfig;
use crate::console_log;

/// NBMA (Non-Broadcast Multi-Access) ネットワークサポート
/// 
/// RFC 2328 Section 7.5: NBMA networks
/// NBMAネットワークでは、ブロードキャストがサポートされないため、
/// 隣接関係は手動で設定し、Hello/LSAパケットはユニキャストで送信する必要がある。

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NBMANeighborConfig {
    /// 隣接ルーターのIPアドレス
    pub neighbor_ip: String,
    /// 隣接ルーターの優先度（DR選出用）
    pub priority: u8,
    /// Poll間隔（秒）- Dead隣接への定期的なHello送信
    pub poll_interval: u32,
    /// 隣接関係が有効かどうか
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NBMAInterfaceConfig {
    /// ネットワークタイプ（NBMAまたはPoint-to-Multipoint）
    pub network_type: OSPFNetworkType,
    /// 手動設定された隣接ルーターリスト
    pub static_neighbors: Vec<NBMANeighborConfig>,
    /// Hello間隔（秒）- NBMAではデフォルト30秒
    pub hello_interval: u32,
    /// Dead間隔（秒）- NBMAではデフォルト120秒
    pub dead_interval: u32,
    /// インターフェース優先度
    pub priority: u8,
}

impl Default for NBMAInterfaceConfig {
    fn default() -> Self {
        NBMAInterfaceConfig {
            network_type: OSPFNetworkType::NBMA,
            static_neighbors: Vec::new(),
            hello_interval: 30,  // RFC 2328: NBMAのデフォルト
            dead_interval: 120,  // RFC 2328: NBMAのデフォルト
            priority: 1,
        }
    }
}

/// NBMAネットワークマネージャー
pub struct NBMAManager {
    /// インターフェースごとのNBMA設定
    interface_configs: HashMap<u32, NBMAInterfaceConfig>,
    /// アクティブな隣接関係（interface_id -> neighbor_ips）
    active_neighbors: HashMap<u32, HashSet<String>>,
    /// Poll Timer状態（interface_id -> (neighbor_ip -> last_poll_time)）
    poll_timers: HashMap<u32, HashMap<String, f64>>,
}

impl NBMAManager {
    pub fn new() -> Self {
        NBMAManager {
            interface_configs: HashMap::new(),
            active_neighbors: HashMap::new(),
            poll_timers: HashMap::new(),
        }
    }
    
    /// インターフェースをNBMAとして設定
    pub fn configure_nbma_interface(
        &mut self,
        interface_id: u32,
        config: NBMAInterfaceConfig,
    ) -> Result<(), String> {
        // ネットワークタイプの検証
        match config.network_type {
            OSPFNetworkType::NBMA | OSPFNetworkType::PointToMultipoint => {},
            _ => return Err("Invalid network type for NBMA configuration".to_string()),
        }
        
        // Hello/Dead間隔の検証
        if config.hello_interval == 0 || config.dead_interval == 0 {
            return Err("Hello and Dead intervals must be greater than 0".to_string());
        }
        
        if config.dead_interval < config.hello_interval * 4 {
            return Err("Dead interval should be at least 4 times the Hello interval".to_string());
        }
        
        self.interface_configs.insert(interface_id, config.clone());
        self.active_neighbors.insert(interface_id, HashSet::new());
        self.poll_timers.insert(interface_id, HashMap::new());
        
        console_log!(
            "NBMA interface {} configured with {} static neighbors",
            interface_id,
            config.static_neighbors.len()
        );
        
        Ok(())
    }
    
    /// 静的隣接ルーターを追加
    pub fn add_static_neighbor(
        &mut self,
        interface_id: u32,
        neighbor: NBMANeighborConfig,
    ) -> Result<(), String> {
        let config = self.interface_configs.get_mut(&interface_id)
            .ok_or_else(|| "Interface not configured for NBMA".to_string())?;
        
        // 重複チェック
        if config.static_neighbors.iter().any(|n| n.neighbor_ip == neighbor.neighbor_ip) {
            return Err(format!("Neighbor {} already exists", neighbor.neighbor_ip));
        }
        
        config.static_neighbors.push(neighbor.clone());
        
        // Poll timerを初期化
        if let Some(timers) = self.poll_timers.get_mut(&interface_id) {
            timers.insert(neighbor.neighbor_ip.clone(), 0.0);
        }
        
        console_log!(
            "Added static neighbor {} to NBMA interface {}",
            neighbor.neighbor_ip,
            interface_id
        );
        
        Ok(())
    }
    
    /// 静的隣接ルーターを削除
    pub fn remove_static_neighbor(
        &mut self,
        interface_id: u32,
        neighbor_ip: &str,
    ) -> Result<(), String> {
        let config = self.interface_configs.get_mut(&interface_id)
            .ok_or_else(|| "Interface not configured for NBMA".to_string())?;
        
        config.static_neighbors.retain(|n| n.neighbor_ip != neighbor_ip);
        
        // アクティブリストからも削除
        if let Some(neighbors) = self.active_neighbors.get_mut(&interface_id) {
            neighbors.remove(neighbor_ip);
        }
        
        // Poll timerからも削除
        if let Some(timers) = self.poll_timers.get_mut(&interface_id) {
            timers.remove(neighbor_ip);
        }
        
        console_log!(
            "Removed static neighbor {} from NBMA interface {}",
            neighbor_ip,
            interface_id
        );
        
        Ok(())
    }
    
    /// NBMAインターフェースでHelloパケットを送信すべき隣接ルーターのリストを取得
    pub fn get_hello_destinations(
        &self,
        interface_id: u32,
    ) -> Vec<String> {
        let config = match self.interface_configs.get(&interface_id) {
            Some(c) => c,
            None => return Vec::new(),
        };
        
        // NBMAでは静的に設定された隣接ルーターにのみ送信
        if config.network_type == OSPFNetworkType::NBMA {
            config.static_neighbors
                .iter()
                .filter(|n| n.enabled)
                .map(|n| n.neighbor_ip.clone())
                .collect()
        } else {
            // Point-to-Multipointの場合は全ての隣接に送信
            Vec::new() // 実装は後で追加
        }
    }
    
    /// Poll Intervalに基づいてHelloを送信すべきかチェック
    pub fn should_send_poll_hello(
        &mut self,
        interface_id: u32,
        neighbor_ip: &str,
        current_time: f64,
    ) -> bool {
        let config = match self.interface_configs.get(&interface_id) {
            Some(c) => c,
            None => return false,
        };
        
        // アクティブな隣接にはPoll不要
        if let Some(neighbors) = self.active_neighbors.get(&interface_id) {
            if neighbors.contains(neighbor_ip) {
                return false;
            }
        }
        
        // Poll間隔をチェック
        if let Some(neighbor_config) = config.static_neighbors.iter()
            .find(|n| n.neighbor_ip == neighbor_ip) {
            
            if let Some(timers) = self.poll_timers.get_mut(&interface_id) {
                if let Some(last_poll) = timers.get_mut(neighbor_ip) {
                    if current_time - *last_poll >= neighbor_config.poll_interval as f64 {
                        *last_poll = current_time;
                        return true;
                    }
                }
            }
        }
        
        false
    }
    
    /// 隣接関係がアクティブになったことを記録
    pub fn mark_neighbor_active(
        &mut self,
        interface_id: u32,
        neighbor_ip: &str,
    ) {
        if let Some(neighbors) = self.active_neighbors.get_mut(&interface_id) {
            neighbors.insert(neighbor_ip.to_string());
            console_log!(
                "NBMA neighbor {} on interface {} is now active",
                neighbor_ip,
                interface_id
            );
        }
    }
    
    /// 隣接関係が非アクティブになったことを記録
    pub fn mark_neighbor_inactive(
        &mut self,
        interface_id: u32,
        neighbor_ip: &str,
    ) {
        if let Some(neighbors) = self.active_neighbors.get_mut(&interface_id) {
            neighbors.remove(neighbor_ip);
            console_log!(
                "NBMA neighbor {} on interface {} is now inactive",
                neighbor_ip,
                interface_id
            );
        }
        
        // Poll timerをリセット
        if let Some(timers) = self.poll_timers.get_mut(&interface_id) {
            if let Some(timer) = timers.get_mut(neighbor_ip) {
                *timer = 0.0; // 次回すぐにPollを送信
            }
        }
    }
    
    /// インターフェースがNBMAとして設定されているかチェック
    pub fn is_nbma_interface(&self, interface_id: u32) -> bool {
        self.interface_configs.contains_key(&interface_id)
    }
    
    /// NBMAインターフェースの設定を取得
    pub fn get_interface_config(&self, interface_id: u32) -> Option<&NBMAInterfaceConfig> {
        self.interface_configs.get(&interface_id)
    }
    
    /// DR適格性をチェック（優先度0の場合はDRになれない）
    pub fn is_dr_eligible(&self, interface_id: u32) -> bool {
        self.interface_configs
            .get(&interface_id)
            .map(|c| c.priority > 0)
            .unwrap_or(true)
    }
    
    /// インターフェース設定をOSPF InterfaceConfigに変換
    pub fn to_interface_config(&self, interface_id: u32) -> Option<InterfaceConfig> {
        self.interface_configs.get(&interface_id).map(|nbma_config| {
            InterfaceConfig {
                hello_interval: Some(nbma_config.hello_interval as u16),
                dead_interval: Some(nbma_config.dead_interval as u16),
                priority: Some(nbma_config.priority),
                ..Default::default()
            }
        })
    }
    
    /// 統計情報を取得
    pub fn get_statistics(&self) -> NBMAStatistics {
        NBMAStatistics {
            total_interfaces: self.interface_configs.len(),
            total_static_neighbors: self.interface_configs
                .values()
                .map(|c| c.static_neighbors.len())
                .sum(),
            active_neighbors: self.active_neighbors
                .values()
                .map(|n| n.len())
                .sum(),
            nbma_interfaces: self.interface_configs
                .values()
                .filter(|c| c.network_type == OSPFNetworkType::NBMA)
                .count(),
            p2mp_interfaces: self.interface_configs
                .values()
                .filter(|c| c.network_type == OSPFNetworkType::PointToMultipoint)
                .count(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NBMAStatistics {
    pub total_interfaces: usize,
    pub total_static_neighbors: usize,
    pub active_neighbors: usize,
    pub nbma_interfaces: usize,
    pub p2mp_interfaces: usize,
}

/// NBMAネットワークでのパケット送信先を決定
pub fn get_nbma_packet_destinations(
    network_type: OSPFNetworkType,
    static_neighbors: &[NBMANeighborConfig],
) -> Vec<String> {
    match network_type {
        OSPFNetworkType::NBMA => {
            // NBMAではすべてユニキャスト
            // Helloは設定された全ての隣接に送信
            static_neighbors
                .iter()
                .filter(|n| n.enabled)
                .map(|n| n.neighbor_ip.clone())
                .collect()
        },
        OSPFNetworkType::PointToMultipoint => {
            // Point-to-Multipointでは個別のpoint-to-pointリンクとして扱う
            Vec::new() // 実装は後で追加
        },
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_nbma_configuration() {
        let mut manager = NBMAManager::new();
        
        let config = NBMAInterfaceConfig {
            network_type: OSPFNetworkType::NBMA,
            static_neighbors: vec![
                NBMANeighborConfig {
                    neighbor_ip: "192.168.1.2".to_string(),
                    priority: 1,
                    poll_interval: 60,
                    enabled: true,
                },
            ],
            hello_interval: 30,
            dead_interval: 120,
            priority: 1,
        };
        
        assert!(manager.configure_nbma_interface(1, config).is_ok());
        assert!(manager.is_nbma_interface(1));
        
        let destinations = manager.get_hello_destinations(1);
        assert_eq!(destinations.len(), 1);
        assert_eq!(destinations[0], "192.168.1.2");
    }
    
    #[test]
    fn test_static_neighbor_management() {
        let mut manager = NBMAManager::new();
        
        let config = NBMAInterfaceConfig::default();
        manager.configure_nbma_interface(1, config).unwrap();
        
        let neighbor = NBMANeighborConfig {
            neighbor_ip: "192.168.1.3".to_string(),
            priority: 1,
            poll_interval: 60,
            enabled: true,
        };
        
        assert!(manager.add_static_neighbor(1, neighbor).is_ok());
        assert_eq!(manager.get_hello_destinations(1).len(), 1);
        
        assert!(manager.remove_static_neighbor(1, "192.168.1.3").is_ok());
        assert_eq!(manager.get_hello_destinations(1).len(), 0);
    }
    
    #[test]
    fn test_poll_timer() {
        let mut manager = NBMAManager::new();
        
        let mut config = NBMAInterfaceConfig::default();
        config.static_neighbors.push(NBMANeighborConfig {
            neighbor_ip: "192.168.1.2".to_string(),
            priority: 1,
            poll_interval: 60,
            enabled: true,
        });
        
        manager.configure_nbma_interface(1, config).unwrap();
        
        // 最初のPollはすぐに送信
        assert!(manager.should_send_poll_hello(1, "192.168.1.2", 0.0));
        
        // Poll間隔内では送信しない
        assert!(!manager.should_send_poll_hello(1, "192.168.1.2", 30.0));
        
        // Poll間隔後は送信
        assert!(manager.should_send_poll_hello(1, "192.168.1.2", 61.0));
    }
}