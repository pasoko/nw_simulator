use std::collections::{HashMap, VecDeque};
use serde::{Serialize, Deserialize};
use crate::device::{ICMPPacket, ICMPType};
use crate::console_log;

/// 拡張されたPing機能
/// 
/// OSPFv2ネットワークシミュレーションで使用される完全なping実装。
/// 複数のシーケンス番号、TTL管理、統計収集、連続ping機能をサポート。

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingSession {
    /// セッションID
    pub session_id: u32,
    /// 送信元デバイスID
    pub source_id: u32,
    /// 送信元IPアドレス
    pub source_ip: String,
    /// 宛先IPアドレス
    pub destination_ip: String,
    /// ICMP識別子
    pub identifier: u16,
    /// 現在のシーケンス番号
    pub current_sequence: u16,
    /// 送信されたパケット数
    pub packets_sent: u32,
    /// 受信されたパケット数
    pub packets_received: u32,
    /// 失われたパケット数
    pub packets_lost: u32,
    /// 最小RTT（ミリ秒）
    pub min_rtt_ms: Option<f64>,
    /// 最大RTT（ミリ秒）
    pub max_rtt_ms: Option<f64>,
    /// 平均RTT（ミリ秒）
    pub avg_rtt_ms: Option<f64>,
    /// RTTの合計（平均計算用）
    pub total_rtt_ms: f64,
    /// 開始時刻
    pub start_time: f64,
    /// 最後のパケット送信時刻
    pub last_sent_time: f64,
    /// セッション設定
    pub config: PingSessionConfig,
    /// アクティブな要求
    pub active_requests: HashMap<u16, PingRequest>,
    /// 個別のping結果
    pub results: VecDeque<IndividualPingResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingSessionConfig {
    /// パケットサイズ（バイト）
    pub packet_size: usize,
    /// 初期TTL
    pub initial_ttl: u8,
    /// タイムアウト（秒）
    pub timeout_seconds: f64,
    /// ping間隔（秒）
    pub interval_seconds: f64,
    /// 最大ping回数（0 = 無制限）
    pub count: u32,
    /// Don't Fragment (DF) ビット
    pub dont_fragment: bool,
    /// Type of Service
    pub tos: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingRequest {
    /// シーケンス番号
    pub sequence_number: u16,
    /// 送信時刻
    pub sent_time: f64,
    /// TTL
    pub ttl: u8,
    /// パケットサイズ
    pub packet_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndividualPingResult {
    /// シーケンス番号
    pub sequence_number: u16,
    /// 成功/失敗
    pub success: bool,
    /// RTT（ミリ秒）
    pub rtt_ms: Option<f64>,
    /// 受信時のTTL
    pub reply_ttl: Option<u8>,
    /// ホップ数（推定）
    pub hop_count: Option<u8>,
    /// エラーメッセージ
    pub error_message: Option<String>,
    /// タイムスタンプ
    pub timestamp: f64,
}

/// 拡張Pingマネージャー
pub struct EnhancedPingManager {
    /// 次のセッションID
    next_session_id: u32,
    /// 次のICMP識別子
    next_identifier: u16,
    /// アクティブなpingセッション
    active_sessions: HashMap<u32, PingSession>,
    /// 識別子からセッションIDへのマッピング
    identifier_to_session: HashMap<u16, u32>,
    /// グローバル統計
    global_stats: GlobalPingStatistics,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalPingStatistics {
    /// 総送信パケット数
    pub total_packets_sent: u64,
    /// 総受信パケット数
    pub total_packets_received: u64,
    /// 総失われたパケット数
    pub total_packets_lost: u64,
    /// 総セッション数
    pub total_sessions: u32,
    /// アクティブセッション数
    pub active_sessions: u32,
    /// 完了セッション数
    pub completed_sessions: u32,
}

impl Default for PingSessionConfig {
    fn default() -> Self {
        PingSessionConfig {
            packet_size: 56,  // 標準的なpingサイズ
            initial_ttl: 64,
            timeout_seconds: 3.0,
            interval_seconds: 1.0,
            count: 0,  // 無制限
            dont_fragment: false,
            tos: 0,
        }
    }
}

impl EnhancedPingManager {
    /// 新しい拡張Pingマネージャーを作成
    pub fn new() -> Self {
        EnhancedPingManager {
            next_session_id: 1,
            next_identifier: 1000,
            active_sessions: HashMap::new(),
            identifier_to_session: HashMap::new(),
            global_stats: GlobalPingStatistics::default(),
        }
    }
    
    /// 新しいpingセッションを開始
    pub fn start_ping_session(
        &mut self,
        source_id: u32,
        source_ip: String,
        destination_ip: String,
        config: PingSessionConfig,
        current_time: f64,
    ) -> Result<u32, String> {
        let session_id = self.next_session_id;
        self.next_session_id += 1;
        
        let identifier = self.next_identifier;
        self.next_identifier = self.next_identifier.wrapping_add(1);
        
        let session = PingSession {
            session_id,
            source_id,
            source_ip: source_ip.clone(),
            destination_ip: destination_ip.clone(),
            identifier,
            current_sequence: 0,
            packets_sent: 0,
            packets_received: 0,
            packets_lost: 0,
            min_rtt_ms: None,
            max_rtt_ms: None,
            avg_rtt_ms: None,
            total_rtt_ms: 0.0,
            start_time: current_time,
            last_sent_time: 0.0,
            config,
            active_requests: HashMap::new(),
            results: VecDeque::new(),
        };
        
        self.active_sessions.insert(session_id, session);
        self.identifier_to_session.insert(identifier, session_id);
        
        self.global_stats.total_sessions += 1;
        self.global_stats.active_sessions += 1;
        
        console_log!(
            "Started ping session {} from {} to {} (identifier: {})",
            session_id, source_ip, destination_ip, identifier
        );
        
        Ok(session_id)
    }
    
    /// 次のpingパケットを生成
    pub fn generate_next_ping(
        &mut self,
        session_id: u32,
        current_time: f64,
    ) -> Result<Option<ICMPPacket>, String> {
        let session = self.active_sessions.get_mut(&session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;
        
        // 最大回数チェック
        if session.config.count > 0 && session.packets_sent >= session.config.count {
            return Ok(None);
        }
        
        // 間隔チェック
        if session.packets_sent > 0 && 
           current_time - session.last_sent_time < session.config.interval_seconds {
            return Ok(None);
        }
        
        // シーケンス番号をインクリメント
        session.current_sequence = session.current_sequence.wrapping_add(1);
        let sequence_number = session.current_sequence;
        
        // パケットを作成
        let mut packet = ICMPPacket::new_echo_request(session.identifier, sequence_number)
            .with_addresses(session.source_ip.clone(), session.destination_ip.clone())
            .with_ttl(session.config.initial_ttl);
        
        // データサイズを調整
        if session.config.packet_size > 8 {  // ICMPヘッダーサイズ
            packet.data = vec![0u8; session.config.packet_size - 8];
        }
        
        // アクティブリクエストに追加
        let request = PingRequest {
            sequence_number,
            sent_time: current_time,
            ttl: session.config.initial_ttl,
            packet_size: session.config.packet_size,
        };
        session.active_requests.insert(sequence_number, request);
        
        // 統計更新
        session.packets_sent += 1;
        session.last_sent_time = current_time;
        self.global_stats.total_packets_sent += 1;
        
        console_log!(
            "Ping session {}: sending seq {} to {}",
            session_id, sequence_number, session.destination_ip
        );
        
        Ok(Some(packet))
    }
    
    /// Echo Replyを処理
    pub fn process_echo_reply(
        &mut self,
        identifier: u16,
        sequence_number: u16,
        reply_ttl: u8,
        current_time: f64,
    ) -> Result<IndividualPingResult, String> {
        let session_id = *self.identifier_to_session.get(&identifier)
            .ok_or_else(|| format!("Unknown identifier: {}", identifier))?;
        
        let session = self.active_sessions.get_mut(&session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;
        
        let request = session.active_requests.remove(&sequence_number)
            .ok_or_else(|| format!("Unknown sequence number: {}", sequence_number))?;
        
        // RTT計算
        let rtt_ms = (current_time - request.sent_time) * 1000.0;
        
        // ホップ数推定（初期TTL - 受信TTL）
        let hop_count = if reply_ttl <= request.ttl {
            Some(request.ttl - reply_ttl)
        } else {
            None
        };
        
        // 統計更新
        session.packets_received += 1;
        session.total_rtt_ms += rtt_ms;
        
        // 最小/最大RTT更新
        match session.min_rtt_ms {
            None => session.min_rtt_ms = Some(rtt_ms),
            Some(min) if rtt_ms < min => session.min_rtt_ms = Some(rtt_ms),
            _ => {}
        }
        
        match session.max_rtt_ms {
            None => session.max_rtt_ms = Some(rtt_ms),
            Some(max) if rtt_ms > max => session.max_rtt_ms = Some(rtt_ms),
            _ => {}
        }
        
        // 平均RTT更新
        if session.packets_received > 0 {
            session.avg_rtt_ms = Some(session.total_rtt_ms / session.packets_received as f64);
        }
        
        self.global_stats.total_packets_received += 1;
        
        let result = IndividualPingResult {
            sequence_number,
            success: true,
            rtt_ms: Some(rtt_ms),
            reply_ttl: Some(reply_ttl),
            hop_count,
            error_message: None,
            timestamp: current_time,
        };
        
        session.results.push_back(result.clone());
        
        // 結果数制限（最新100件のみ保持）
        while session.results.len() > 100 {
            session.results.pop_front();
        }
        
        console_log!(
            "Ping reply: session {} seq {} RTT {:.2}ms TTL {} hops {}",
            session_id, sequence_number, rtt_ms, reply_ttl,
            hop_count.unwrap_or(0)
        );
        
        Ok(result)
    }
    
    /// ICMPエラーメッセージを処理
    pub fn process_icmp_error(
        &mut self,
        error_type: ICMPType,
        identifier: u16,
        sequence_number: u16,
        current_time: f64,
    ) -> Result<IndividualPingResult, String> {
        let session_id = *self.identifier_to_session.get(&identifier)
            .ok_or_else(|| format!("Unknown identifier: {}", identifier))?;
        
        let session = self.active_sessions.get_mut(&session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;
        
        session.active_requests.remove(&sequence_number);
        session.packets_lost += 1;
        self.global_stats.total_packets_lost += 1;
        
        let error_message = match error_type {
            ICMPType::DestinationUnreachable => "Destination unreachable",
            ICMPType::TimeExceeded => "Time exceeded (TTL expired)",
            _ => "Unknown error",
        };
        
        let result = IndividualPingResult {
            sequence_number,
            success: false,
            rtt_ms: None,
            reply_ttl: None,
            hop_count: None,
            error_message: Some(error_message.to_string()),
            timestamp: current_time,
        };
        
        session.results.push_back(result.clone());
        
        console_log!(
            "Ping error: session {} seq {} - {}",
            session_id, sequence_number, error_message
        );
        
        Ok(result)
    }
    
    /// タイムアウトチェックと処理
    pub fn check_timeouts(&mut self, current_time: f64) {
        for session in self.active_sessions.values_mut() {
            let mut timed_out = Vec::new();
            
            for (seq, request) in &session.active_requests {
                if current_time - request.sent_time > session.config.timeout_seconds {
                    timed_out.push(*seq);
                }
            }
            
            for seq in timed_out {
                if let Some(_request) = session.active_requests.remove(&seq) {
                    session.packets_lost += 1;
                    self.global_stats.total_packets_lost += 1;
                    
                    let result = IndividualPingResult {
                        sequence_number: seq,
                        success: false,
                        rtt_ms: None,
                        reply_ttl: None,
                        hop_count: None,
                        error_message: Some("Request timed out".to_string()),
                        timestamp: current_time,
                    };
                    
                    session.results.push_back(result);
                    
                    console_log!(
                        "Ping timeout: session {} seq {}",
                        session.session_id, seq
                    );
                }
            }
        }
    }
    
    /// pingセッションを停止
    pub fn stop_session(&mut self, session_id: u32) -> Result<PingSessionSummary, String> {
        let session = self.active_sessions.remove(&session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;
        
        self.identifier_to_session.remove(&session.identifier);
        
        self.global_stats.active_sessions -= 1;
        self.global_stats.completed_sessions += 1;
        
        let summary = PingSessionSummary {
            session_id: session.session_id,
            source_ip: session.source_ip,
            destination_ip: session.destination_ip,
            packets_sent: session.packets_sent,
            packets_received: session.packets_received,
            packets_lost: session.packets_lost,
            loss_percentage: if session.packets_sent > 0 {
                (session.packets_lost as f64 / session.packets_sent as f64) * 100.0
            } else {
                0.0
            },
            min_rtt_ms: session.min_rtt_ms,
            max_rtt_ms: session.max_rtt_ms,
            avg_rtt_ms: session.avg_rtt_ms,
            duration_seconds: session.last_sent_time - session.start_time,
        };
        
        console_log!(
            "Ping session {} stopped: {} packets sent, {} received, {:.1}% loss",
            session_id, summary.packets_sent, summary.packets_received, summary.loss_percentage
        );
        
        Ok(summary)
    }
    
    /// セッション情報を取得
    pub fn get_session_info(&self, session_id: u32) -> Option<&PingSession> {
        self.active_sessions.get(&session_id)
    }
    
    /// すべてのアクティブセッションを取得
    pub fn get_active_sessions(&self) -> Vec<&PingSession> {
        self.active_sessions.values().collect()
    }
    
    /// グローバル統計を取得
    pub fn get_global_statistics(&self) -> &GlobalPingStatistics {
        &self.global_stats
    }
    
    /// セッションが完了したかチェック
    pub fn is_session_complete(&self, session_id: u32) -> bool {
        if let Some(session) = self.active_sessions.get(&session_id) {
            session.config.count > 0 && session.packets_sent >= session.config.count
        } else {
            true  // セッションが存在しない場合は完了とみなす
        }
    }
    
    /// 統計をリセット
    pub fn reset_global_statistics(&mut self) {
        self.global_stats = GlobalPingStatistics::default();
        console_log!("Global ping statistics reset");
    }
}

/// Pingセッションのサマリー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingSessionSummary {
    pub session_id: u32,
    pub source_ip: String,
    pub destination_ip: String,
    pub packets_sent: u32,
    pub packets_received: u32,
    pub packets_lost: u32,
    pub loss_percentage: f64,
    pub min_rtt_ms: Option<f64>,
    pub max_rtt_ms: Option<f64>,
    pub avg_rtt_ms: Option<f64>,
    pub duration_seconds: f64,
}

/// Traceroute機能
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerouteSession {
    pub session_id: u32,
    pub source_id: u32,
    pub source_ip: String,
    pub destination_ip: String,
    pub max_hops: u8,
    pub current_ttl: u8,
    pub probes_per_hop: u8,
    pub timeout_seconds: f64,
    pub hops: Vec<TracerouteHop>,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerouteHop {
    pub hop_number: u8,
    pub router_ip: Option<String>,
    pub router_id: Option<u32>,
    pub rtt_ms: Vec<Option<f64>>,
    pub reached_destination: bool,
}

impl EnhancedPingManager {
    /// Tracerouteセッションを開始
    pub fn start_traceroute(
        &mut self,
        source_id: u32,
        source_ip: String,
        destination_ip: String,
        max_hops: u8,
        probes_per_hop: u8,
        timeout_seconds: f64,
    ) -> u32 {
        let session_id = self.next_session_id;
        self.next_session_id += 1;
        
        let _session = TracerouteSession {
            session_id,
            source_id,
            source_ip: source_ip.clone(),
            destination_ip: destination_ip.clone(),
            max_hops,
            current_ttl: 1,
            probes_per_hop,
            timeout_seconds,
            hops: Vec::new(),
            completed: false,
        };
        
        console_log!(
            "Started traceroute session {} from {} to {} (max {} hops)",
            session_id, source_ip, destination_ip, max_hops
        );
        
        // TODO: Store traceroute sessions separately
        session_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_session_creation() {
        let mut manager = EnhancedPingManager::new();
        
        let config = PingSessionConfig::default();
        let session_id = manager.start_ping_session(
            1,
            "192.168.1.10".to_string(),
            "8.8.8.8".to_string(),
            config,
            0.0,
        ).unwrap();
        
        assert_eq!(session_id, 1);
        assert_eq!(manager.active_sessions.len(), 1);
        assert_eq!(manager.global_stats.active_sessions, 1);
    }
    
    #[test]
    fn test_ping_generation() {
        let mut manager = EnhancedPingManager::new();
        
        let config = PingSessionConfig {
            count: 3,
            ..Default::default()
        };
        
        let session_id = manager.start_ping_session(
            1,
            "192.168.1.10".to_string(),
            "8.8.8.8".to_string(),
            config,
            0.0,
        ).unwrap();
        
        // 最初のping
        let packet1 = manager.generate_next_ping(session_id, 0.0).unwrap().unwrap();
        assert_eq!(packet1.sequence_number, 1);
        
        // 2番目のping（間隔前）
        let packet2 = manager.generate_next_ping(session_id, 0.5).unwrap();
        assert!(packet2.is_none());  // 間隔が短すぎる
        
        // 2番目のping（間隔後）
        let packet3 = manager.generate_next_ping(session_id, 1.0).unwrap().unwrap();
        assert_eq!(packet3.sequence_number, 2);
        
        let session = manager.get_session_info(session_id).unwrap();
        assert_eq!(session.packets_sent, 2);
    }
    
    #[test]
    fn test_echo_reply_processing() {
        let mut manager = EnhancedPingManager::new();
        
        let session_id = manager.start_ping_session(
            1,
            "192.168.1.10".to_string(),
            "8.8.8.8".to_string(),
            PingSessionConfig::default(),
            0.0,
        ).unwrap();
        
        let packet = manager.generate_next_ping(session_id, 0.0).unwrap().unwrap();
        
        // 100ms後にreply受信
        let result = manager.process_echo_reply(
            packet.identifier,
            packet.sequence_number,
            56,  // Reply TTL
            0.1,
        ).unwrap();
        
        assert!(result.success);
        assert_eq!(result.rtt_ms, Some(100.0));
        assert_eq!(result.hop_count, Some(8));  // 64 - 56 = 8 hops
        
        let session = manager.get_session_info(session_id).unwrap();
        assert_eq!(session.packets_received, 1);
        assert_eq!(session.min_rtt_ms, Some(100.0));
    }
    
    #[test]
    fn test_timeout_handling() {
        let mut manager = EnhancedPingManager::new();
        
        let session_id = manager.start_ping_session(
            1,
            "192.168.1.10".to_string(),
            "8.8.8.8".to_string(),
            PingSessionConfig::default(),
            0.0,
        ).unwrap();
        
        let _ = manager.generate_next_ping(session_id, 0.0).unwrap();
        
        // タイムアウトチェック
        manager.check_timeouts(4.0);  // 3秒タイムアウト後
        
        let session = manager.get_session_info(session_id).unwrap();
        assert_eq!(session.packets_lost, 1);
        assert_eq!(session.results.len(), 1);
        assert!(!session.results[0].success);
    }
    
    #[test]
    fn test_session_summary() {
        let mut manager = EnhancedPingManager::new();
        
        let session_id = manager.start_ping_session(
            1,
            "192.168.1.10".to_string(),
            "8.8.8.8".to_string(),
            PingSessionConfig::default(),
            0.0,
        ).unwrap();
        
        // 複数のping送信
        for i in 0..3 {
            let packet = manager.generate_next_ping(session_id, i as f64).unwrap().unwrap();
            
            // 2つ成功、1つ失敗
            if i < 2 {
                let _ = manager.process_echo_reply(
                    packet.identifier,
                    packet.sequence_number,
                    56,
                    i as f64 + 0.05,  // 50ms RTT
                ).unwrap();
            }
        }
        
        manager.check_timeouts(5.0);
        
        let summary = manager.stop_session(session_id).unwrap();
        assert_eq!(summary.packets_sent, 3);
        assert_eq!(summary.packets_received, 2);
        assert_eq!(summary.packets_lost, 1);
        assert_eq!(summary.loss_percentage, 33.333333333333336);
    }
}