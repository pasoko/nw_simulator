use std::collections::HashMap;
use crate::device::ICMPPacket;
use crate::console_log;

/// Ping要求の状態
#[derive(Debug, Clone)]
pub struct PingRequest {
    pub source_id: u32,
    pub destination_ip: String,
    pub identifier: u16,
    pub sequence_number: u16,
    pub sent_time: f64,
    pub ttl: u8,
}

/// Pingマネージャー
pub struct PingManager {
    next_identifier: u16,
    active_pings: HashMap<u16, PingRequest>,  // identifier -> PingRequest
    ping_results: Vec<PingResult>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PingResult {
    pub source_id: u32,
    pub destination_ip: String,
    pub success: bool,
    pub rtt_ms: Option<f64>,
    pub ttl: Option<u8>,
    pub error_message: Option<String>,
}

impl PingManager {
    pub fn new() -> Self {
        PingManager {
            next_identifier: 1,
            active_pings: HashMap::new(),
            ping_results: Vec::new(),
        }
    }

    /// 新しいping要求を作成
    pub fn create_ping_request(
        &mut self,
        source_id: u32,
        destination_ip: String,
        current_time: f64,
    ) -> (u16, ICMPPacket) {
        let identifier = self.next_identifier;
        self.next_identifier = self.next_identifier.wrapping_add(1);
        
        let sequence_number = 1;  // 簡単のため、常に1から開始
        let ttl = 64;  // デフォルトTTL

        let ping_request = PingRequest {
            source_id,
            destination_ip: destination_ip.clone(),
            identifier,
            sequence_number,
            sent_time: current_time,
            ttl,
        };

        self.active_pings.insert(identifier, ping_request);
        
        let icmp_packet = ICMPPacket::new_echo_request(identifier, sequence_number)
            .with_addresses(String::new(), destination_ip.clone());
        
        console_log!("Created ping request from {} to {} (id: {})", 
            source_id, destination_ip, identifier);
        
        (identifier, icmp_packet)
    }

    /// Echo Replyを処理
    pub fn process_echo_reply(
        &mut self,
        identifier: u16,
        current_time: f64,
    ) -> Option<PingResult> {
        if let Some(request) = self.active_pings.remove(&identifier) {
            let rtt_ms = (current_time - request.sent_time) * 1000.0;
            
            let result = PingResult {
                source_id: request.source_id,
                destination_ip: request.destination_ip.clone(),
                success: true,
                rtt_ms: Some(rtt_ms),
                ttl: Some(request.ttl),
                error_message: None,
            };
            
            console_log!("Ping reply received: {} -> {} RTT: {:.2}ms", 
                request.source_id, request.destination_ip, rtt_ms);
            
            self.ping_results.push(result.clone());
            Some(result)
        } else {
            console_log!("Received echo reply for unknown identifier: {}", identifier);
            None
        }
    }

    /// タイムアウトしたping要求をクリーンアップ
    pub fn cleanup_timeouts(&mut self, current_time: f64, timeout_seconds: f64) {
        let mut timed_out = Vec::new();
        
        for (identifier, request) in &self.active_pings {
            if current_time - request.sent_time > timeout_seconds {
                timed_out.push(*identifier);
            }
        }
        
        for identifier in timed_out {
            if let Some(request) = self.active_pings.remove(&identifier) {
                let result = PingResult {
                    source_id: request.source_id,
                    destination_ip: request.destination_ip.clone(),
                    success: false,
                    rtt_ms: None,
                    ttl: None,
                    error_message: Some("Request timed out".to_string()),
                };
                
                console_log!("Ping timeout: {} -> {}", 
                    request.source_id, request.destination_ip);
                
                self.ping_results.push(result);
            }
        }
    }

    /// 最近のping結果を取得
    pub fn get_recent_results(&self, count: usize) -> Vec<PingResult> {
        let start = self.ping_results.len().saturating_sub(count);
        self.ping_results[start..].to_vec()
    }

    /// すべてのping結果をクリア
    pub fn clear_results(&mut self) {
        self.ping_results.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_request_creation() {
        let mut manager = PingManager::new();
        let (id, packet) = manager.create_ping_request(1, "192.168.1.1".to_string(), 0.0);
        
        assert_eq!(id, 1);
        assert_eq!(packet.identifier, 1);
        assert_eq!(packet.packet_type, crate::device::ICMPType::EchoRequest);
        assert!(manager.active_pings.contains_key(&id));
    }

    #[test]
    fn test_echo_reply_processing() {
        let mut manager = PingManager::new();
        let (id, _) = manager.create_ping_request(1, "192.168.1.1".to_string(), 0.0);
        
        // 100ms後にreplyを受信
        let result = manager.process_echo_reply(id, 0.1).unwrap();
        
        assert!(result.success);
        assert_eq!(result.rtt_ms, Some(100.0));
        assert!(!manager.active_pings.contains_key(&id));
    }

    #[test]
    fn test_timeout_cleanup() {
        let mut manager = PingManager::new();
        let (id, _) = manager.create_ping_request(1, "192.168.1.1".to_string(), 0.0);
        
        // 5秒後にタイムアウトチェック（タイムアウト値: 3秒）
        manager.cleanup_timeouts(5.0, 3.0);
        
        assert!(!manager.active_pings.contains_key(&id));
        assert_eq!(manager.ping_results.len(), 1);
        assert!(!manager.ping_results[0].success);
    }
}