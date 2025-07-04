// Error Recovery Strategies
//
// Implements various recovery strategies for different error scenarios
// in the OSPF protocol implementation.

use serde::{Serialize, Deserialize};
use crate::ospf_refactored::state::NeighborState;
use crate::ospf_refactored::events::OSPFEvent;

/// Recovery actions that can be taken
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryAction {
    /// Retry the operation
    Retry,
    /// Reset neighbor to Down state
    ResetNeighbor,
    /// Clear interface state
    ClearInterface,
    /// Resend specific packet
    ResendPacket(PacketResendInfo),
    /// Trigger SPF recalculation
    RecalculateSPF,
    /// Flush LSA from database
    FlushLSA(LSAIdentifier),
    /// Restart protocol on interface
    RestartInterface,
    /// Log and continue
    LogAndContinue,
    /// Escalate to higher level
    Escalate,
    /// No action needed
    NoAction,
}

/// Information for packet resend
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PacketResendInfo {
    pub packet_type: u8,
    pub neighbor_id: u32,
    pub interface_id: u32,
    pub sequence_number: Option<u32>,
}

/// LSA identifier for flush operations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LSAIdentifier {
    pub ls_type: u8,
    pub ls_id: u32,
    pub advertising_router: u32,
}

/// Strategy for determining recovery actions
pub trait RecoveryStrategy {
    /// Determine recovery action for a given error
    fn determine_action(&self, error_type: &str, context: &crate::ospf_refactored::error_handling::ErrorContext) -> RecoveryAction;
    
    /// Execute recovery action
    fn execute_recovery(&mut self, action: &RecoveryAction) -> Result<Vec<OSPFEvent>, String>;
}

/// Default recovery strategy implementation
pub struct DefaultRecoveryStrategy {
    /// Maximum retries before escalation
    pub max_retries: u32,
    /// Current retry counts per neighbor
    pub retry_counts: std::collections::HashMap<u32, u32>,
}

impl DefaultRecoveryStrategy {
    pub fn new() -> Self {
        Self {
            max_retries: 3,
            retry_counts: std::collections::HashMap::new(),
        }
    }
    
    /// Increment retry count for a neighbor
    fn increment_retry(&mut self, neighbor_id: u32) -> u32 {
        let count = self.retry_counts.entry(neighbor_id).or_insert(0);
        *count += 1;
        *count
    }
    
    /// Clear retry count for a neighbor
    fn clear_retry(&mut self, neighbor_id: u32) {
        self.retry_counts.remove(&neighbor_id);
    }
}

impl RecoveryStrategy for DefaultRecoveryStrategy {
    fn determine_action(&self, error_type: &str, context: &crate::ospf_refactored::error_handling::ErrorContext) -> RecoveryAction {
        match error_type {
            "PacketError::ChecksumMismatch" => RecoveryAction::LogAndContinue,
            
            "PacketError::InvalidFormat" => {
                if let Some(neighbor_id) = context.neighbor_id {
                    if self.retry_counts.get(&neighbor_id).copied().unwrap_or(0) < self.max_retries {
                        RecoveryAction::Retry
                    } else {
                        RecoveryAction::ResetNeighbor
                    }
                } else {
                    RecoveryAction::LogAndContinue
                }
            }
            
            "StateError::InvalidTransition" => {
                match context.state.as_deref() {
                    Some("ExStart") | Some("Exchange") => RecoveryAction::ResetNeighbor,
                    _ => RecoveryAction::LogAndContinue,
                }
            }
            
            "EventError::EventLoopDetected" => RecoveryAction::Escalate,
            
            "DD sequence mismatch" => {
                if let Some(neighbor_id) = context.neighbor_id {
                    RecoveryAction::ResendPacket(PacketResendInfo {
                        packet_type: 2, // DD packet
                        neighbor_id,
                        interface_id: context.interface_id.unwrap_or(0),
                        sequence_number: None,
                    })
                } else {
                    RecoveryAction::ResetNeighbor
                }
            }
            
            "LSA not found" => {
                if let Some(info) = context.additional_info.as_ref() {
                    if let Some(lsa_type) = info.get("lsa_type").and_then(|v| v.as_u64()) {
                        if let Some(lsa_id) = info.get("lsa_id").and_then(|v| v.as_u64()) {
                            if let Some(adv_router) = info.get("advertising_router").and_then(|v| v.as_u64()) {
                                return RecoveryAction::FlushLSA(LSAIdentifier {
                                    ls_type: lsa_type as u8,
                                    ls_id: lsa_id as u32,
                                    advertising_router: adv_router as u32,
                                });
                            }
                        }
                    }
                }
                RecoveryAction::LogAndContinue
            }
            
            _ => RecoveryAction::LogAndContinue,
        }
    }
    
    fn execute_recovery(&mut self, action: &RecoveryAction) -> Result<Vec<OSPFEvent>, String> {
        let mut events = Vec::new();
        
        match action {
            RecoveryAction::ResetNeighbor => {
                if let Some(neighbor_id) = self.retry_counts.keys().next().copied() {
                    self.clear_retry(neighbor_id);
                    events.push(OSPFEvent::NeighborStateChanged {
                        router_id: 0, // Should be provided by context
                        neighbor_id,
                        from_state: NeighborState::Full, // Should check actual state
                        to_state: NeighborState::Down,
                        interface_id: 0, // Should be provided by context
                    });
                }
                Ok(events)
            }
            
            RecoveryAction::ResendPacket(info) => {
                let packet_type = match info.packet_type {
                    1 => crate::ospf_refactored::events::PacketType::Hello,
                    2 => crate::ospf_refactored::events::PacketType::DatabaseDescription,
                    3 => crate::ospf_refactored::events::PacketType::LinkStateRequest,
                    4 => crate::ospf_refactored::events::PacketType::LinkStateUpdate,
                    5 => crate::ospf_refactored::events::PacketType::LinkStateAck,
                    _ => return Err("Invalid packet type".to_string()),
                };
                events.push(OSPFEvent::PacketSendRequired {
                    packet_type,
                    destination: info.neighbor_id,
                    interface_id: info.interface_id,
                    additional_data: info.sequence_number.map(|seq| format!("seq={}", seq)),
                });
                Ok(events)
            }
            
            RecoveryAction::RecalculateSPF => {
                events.push(OSPFEvent::SPFRequired {
                    area_id: 0, // Should be provided by context
                    reason: "Error recovery".to_string(),
                });
                Ok(events)
            }
            
            RecoveryAction::FlushLSA(lsa_id) => {
                events.push(OSPFEvent::LSAFloodRequired {
                    lsa_key: format!("{}:{}:{}", lsa_id.ls_type, lsa_id.ls_id, lsa_id.advertising_router),
                    exclude_interface: None,
                    exclude_neighbor: None,
                });
                Ok(events)
            }
            
            RecoveryAction::LogAndContinue | RecoveryAction::NoAction => {
                Ok(events)
            }
            
            _ => Err(format!("Recovery action {:?} not implemented", action)),
        }
    }
}

/// Recovery coordinator that manages multiple strategies
pub struct RecoveryCoordinator {
    strategies: Vec<Box<dyn RecoveryStrategy + Send>>,
    /// History of recovery actions
    history: Vec<RecoveryHistoryEntry>,
    /// Maximum history entries
    max_history: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryHistoryEntry {
    pub timestamp: f64,
    pub error_type: String,
    pub action: RecoveryAction,
    pub success: bool,
}

impl RecoveryCoordinator {
    pub fn new() -> Self {
        Self {
            strategies: vec![Box::new(DefaultRecoveryStrategy::new())],
            history: Vec::new(),
            max_history: 100,
        }
    }
    
    /// Add a recovery strategy
    pub fn add_strategy(&mut self, strategy: Box<dyn RecoveryStrategy + Send>) {
        self.strategies.push(strategy);
    }
    
    /// Determine and execute recovery
    pub fn handle_error(
        &mut self,
        error_type: &str,
        context: &crate::ospf_refactored::error_handling::ErrorContext,
    ) -> Result<Vec<OSPFEvent>, String> {
        // Try each strategy until one provides an action
        let action = self.strategies
            .iter()
            .map(|s| s.determine_action(error_type, context))
            .find(|a| !matches!(a, RecoveryAction::NoAction))
            .unwrap_or(RecoveryAction::LogAndContinue);
        
        // Execute the recovery action
        let result = self.strategies
            .iter_mut()
            .find_map(|s| s.execute_recovery(&action).ok())
            .ok_or_else(|| "No strategy could execute recovery".to_string());
        
        // Record in history
        self.record_recovery(error_type, action, result.is_ok());
        
        result
    }
    
    /// Record recovery action in history
    fn record_recovery(&mut self, error_type: &str, action: RecoveryAction, success: bool) {
        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        
        self.history.push(RecoveryHistoryEntry {
            timestamp: get_timestamp(),
            error_type: error_type.to_string(),
            action,
            success,
        });
    }
    
    /// Get recovery history
    pub fn get_history(&self) -> &[RecoveryHistoryEntry] {
        &self.history
    }
    
    /// Get recovery success rate
    pub fn get_success_rate(&self) -> f64 {
        if self.history.is_empty() {
            return 1.0;
        }
        
        let successful = self.history.iter().filter(|e| e.success).count();
        successful as f64 / self.history.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ospf_refactored::error_handling::ErrorContext;
    
    #[test]
    fn test_default_recovery_strategy() {
        let strategy = DefaultRecoveryStrategy::new();
        let context = ErrorContext::new(1).with_neighbor(2);
        
        // Test checksum error -> log and continue
        let action = strategy.determine_action("PacketError::ChecksumMismatch", &context);
        assert_eq!(action, RecoveryAction::LogAndContinue);
        
        // Test invalid format -> retry
        let action = strategy.determine_action("PacketError::InvalidFormat", &context);
        assert_eq!(action, RecoveryAction::Retry);
    }
    
    #[test]
    fn test_recovery_coordinator() {
        let mut coordinator = RecoveryCoordinator::new();
        let context = ErrorContext::new(1);
        
        // Test error handling
        let result = coordinator.handle_error("PacketError::ChecksumMismatch", &context);
        assert!(result.is_ok());
        
        // Check history
        assert_eq!(coordinator.history.len(), 1);
        assert_eq!(coordinator.history[0].error_type, "PacketError::ChecksumMismatch");
        assert_eq!(coordinator.history[0].action, RecoveryAction::LogAndContinue);
        
        // Check success rate
        assert_eq!(coordinator.get_success_rate(), 1.0);
    }
}

// Helper function for getting timestamp that works in both WASM and native
fn get_timestamp() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() / 1000.0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
    }
}