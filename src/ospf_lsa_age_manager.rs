use std::collections::HashMap;
use crate::router::{LSA, LSAHeader};
use crate::console_log;

/// LSA Age Management for OSPFv2 (RFC 2328 Section 14)
/// 
/// This module provides enhanced LSA age management functionality including:
/// - Accurate age calculation with InfTransDelay
/// - MaxAge LSA handling
/// - Age rollover protection
/// - LSA refresh mechanisms

pub const MAX_AGE: u16 = 3600;
pub const LS_REFRESH_TIME: u16 = 1800;
pub const MIN_LS_INTERVAL: u16 = 5;
pub const MIN_LS_ARRIVAL: u16 = 1;
pub const CHECK_AGE: u16 = 300;
pub const MAX_AGE_DIFF: u16 = 900;
pub const LS_INFINITY: u32 = 0xFFFFFF;
pub const INITIAL_SEQUENCE_NUMBER: u32 = 0x80000001;
pub const MAX_SEQUENCE_NUMBER: u32 = 0x7FFFFFFF;

#[derive(Debug, Clone)]
pub struct LSAAgeInfo {
    /// The time when this LSA was received or originated
    pub base_time: f64,
    
    /// The age value when the LSA was received (0 for self-originated)
    pub received_age: u16,
    
    /// The interface through which the LSA was received
    pub receiving_interface: Option<u32>,
    
    /// Whether this LSA is self-originated
    pub self_originated: bool,
    
    /// Time when the LSA should be refreshed (for self-originated LSAs)
    pub refresh_time: Option<f64>,
    
    /// Whether this LSA has been flushed (MaxAge)
    pub flushed: bool,
    
    /// The actual age value last calculated
    pub last_calculated_age: u16,
    
    /// Time when age was last calculated
    pub last_calculation_time: f64,
}

pub struct LSAAgeManager {
    /// Tracking information for each LSA
    age_info: HashMap<String, LSAAgeInfo>,
    
    /// Current simulation time
    current_time: f64,
    
    /// InfTransDelay values per interface
    interface_delays: HashMap<u32, u16>,
    
    /// LSAs that have reached MaxAge and are pending removal
    maxage_lsas: HashMap<String, f64>,
    
    /// LSAs that need to be refreshed
    refresh_pending: HashMap<String, f64>,
}

impl LSAAgeManager {
    pub fn new() -> Self {
        LSAAgeManager {
            age_info: HashMap::new(),
            current_time: 0.0,
            interface_delays: HashMap::new(),
            maxage_lsas: HashMap::new(),
            refresh_pending: HashMap::new(),
        }
    }
    
    /// Update current time
    pub fn update_time(&mut self, time: f64) {
        self.current_time = time;
    }
    
    /// Set InfTransDelay for an interface
    pub fn set_interface_delay(&mut self, interface_id: u32, delay: u16) {
        self.interface_delays.insert(interface_id, delay);
    }
    
    /// Get InfTransDelay for an interface
    pub fn get_interface_delay(&self, interface_id: u32) -> u16 {
        self.interface_delays.get(&interface_id).copied().unwrap_or(1)
    }
    
    /// Record a self-originated LSA
    pub fn record_self_originated_lsa(&mut self, lsa_key: String, _lsa: &LSA) {
        let refresh_time = self.current_time + (LS_REFRESH_TIME as f64);
        
        let info = LSAAgeInfo {
            base_time: self.current_time,
            received_age: 0,
            receiving_interface: None,
            self_originated: true,
            refresh_time: Some(refresh_time),
            flushed: false,
            last_calculated_age: 0,
            last_calculation_time: self.current_time,
        };
        
        self.age_info.insert(lsa_key.clone(), info);
        self.refresh_pending.insert(lsa_key.clone(), refresh_time);
        
        console_log!("Recorded self-originated LSA {} with refresh time {:.0}s",
            lsa_key, refresh_time);
    }
    
    /// Record a received LSA
    pub fn record_received_lsa(&mut self, lsa_key: String, lsa: &LSA, interface_id: Option<u32>) {
        let received_age = lsa.header.ls_age;
        
        // Add InfTransDelay if received on an interface
        let adjusted_age = if let Some(iface_id) = interface_id {
            let delay = self.get_interface_delay(iface_id);
            std::cmp::min(received_age.saturating_add(delay), MAX_AGE)
        } else {
            received_age
        };
        
        let info = LSAAgeInfo {
            base_time: self.current_time,
            received_age: adjusted_age,
            receiving_interface: interface_id,
            self_originated: false,
            refresh_time: None,
            flushed: adjusted_age >= MAX_AGE,
            last_calculated_age: adjusted_age,
            last_calculation_time: self.current_time,
        };
        
        if adjusted_age >= MAX_AGE {
            self.maxage_lsas.insert(lsa_key.clone(), self.current_time);
        }
        
        self.age_info.insert(lsa_key, info);
        
        console_log!("Recorded received LSA with age {} (adjusted to {})",
            received_age, adjusted_age);
    }
    
    /// Calculate current age of an LSA
    pub fn calculate_age(&mut self, lsa_key: &str) -> u16 {
        if let Some(info) = self.age_info.get_mut(lsa_key) {
            if info.flushed {
                return MAX_AGE;
            }
            
            let elapsed = (self.current_time - info.base_time) as u16;
            let current_age = info.received_age.saturating_add(elapsed);
            
            // Check if reached MaxAge
            if current_age >= MAX_AGE {
                info.flushed = true;
                info.last_calculated_age = MAX_AGE;
                info.last_calculation_time = self.current_time;
                self.maxage_lsas.insert(lsa_key.to_string(), self.current_time);
                MAX_AGE
            } else {
                info.last_calculated_age = current_age;
                info.last_calculation_time = self.current_time;
                current_age
            }
        } else {
            0
        }
    }
    
    /// Update LSA header with current age
    pub fn update_lsa_age(&mut self, lsa_key: &str, header: &mut LSAHeader) {
        let age = self.calculate_age(lsa_key);
        header.ls_age = age;
    }
    
    /// Check if an LSA needs to be refreshed
    pub fn needs_refresh(&self, lsa_key: &str) -> bool {
        if let Some(info) = self.age_info.get(lsa_key) {
            if info.self_originated && !info.flushed {
                if let Some(refresh_time) = info.refresh_time {
                    return self.current_time >= refresh_time;
                }
            }
        }
        false
    }
    
    /// Get LSAs that need to be refreshed
    pub fn get_lsas_needing_refresh(&self) -> Vec<String> {
        self.refresh_pending
            .iter()
            .filter(|(_, &time)| self.current_time >= time)
            .map(|(key, _)| key.clone())
            .collect()
    }
    
    /// Mark an LSA as refreshed
    pub fn mark_refreshed(&mut self, lsa_key: &str, new_sequence: u32) {
        if let Some(info) = self.age_info.get_mut(lsa_key) {
            info.base_time = self.current_time;
            info.received_age = 0;
            info.last_calculated_age = 0;
            info.last_calculation_time = self.current_time;
            
            let new_refresh_time = self.current_time + (LS_REFRESH_TIME as f64);
            info.refresh_time = Some(new_refresh_time);
            
            self.refresh_pending.insert(lsa_key.to_string(), new_refresh_time);
            
            console_log!("LSA {} refreshed with sequence {:#x}, next refresh at {:.0}s",
                lsa_key, new_sequence, new_refresh_time);
        }
    }
    
    /// Force an LSA to MaxAge (for flushing)
    pub fn flush_lsa(&mut self, lsa_key: &str) {
        if let Some(info) = self.age_info.get_mut(lsa_key) {
            info.flushed = true;
            info.last_calculated_age = MAX_AGE;
            info.last_calculation_time = self.current_time;
            self.maxage_lsas.insert(lsa_key.to_string(), self.current_time);
            
            console_log!("LSA {} flushed to MaxAge", lsa_key);
        }
    }
    
    /// Get all MaxAge LSAs
    pub fn get_maxage_lsas(&self) -> Vec<String> {
        self.maxage_lsas.keys().cloned().collect()
    }
    
    /// Remove an LSA from tracking
    pub fn remove_lsa(&mut self, lsa_key: &str) {
        self.age_info.remove(lsa_key);
        self.maxage_lsas.remove(lsa_key);
        self.refresh_pending.remove(lsa_key);
    }
    
    /// Check if two LSAs have significantly different ages (MaxAgeDiff)
    pub fn is_age_diff_significant(&self, age1: u16, age2: u16) -> bool {
        let diff = if age1 > age2 {
            age1 - age2
        } else {
            age2 - age1
        };
        diff > MAX_AGE_DIFF
    }
    
    /// Check if an LSA should be accepted based on age
    pub fn should_accept_lsa(&self, lsa_key: &str, new_age: u16) -> bool {
        if let Some(info) = self.age_info.get(lsa_key) {
            let current_age = info.last_calculated_age;
            
            // Always accept if current is MaxAge
            if current_age >= MAX_AGE {
                return true;
            }
            
            // Accept if new is MaxAge
            if new_age >= MAX_AGE {
                return true;
            }
            
            // Check age difference
            !self.is_age_diff_significant(current_age, new_age)
        } else {
            // No existing LSA, accept
            true
        }
    }
    
    /// Get age information for an LSA
    pub fn get_age_info(&self, lsa_key: &str) -> Option<&LSAAgeInfo> {
        self.age_info.get(lsa_key)
    }
    
    /// Get all LSAs with their current ages
    pub fn get_all_lsa_ages(&mut self) -> HashMap<String, u16> {
        let keys: Vec<String> = self.age_info.keys().cloned().collect();
        let mut ages = HashMap::new();
        
        for key in keys {
            let age = self.calculate_age(&key);
            ages.insert(key, age);
        }
        
        ages
    }
    
    /// Increment sequence number with rollover protection
    pub fn increment_sequence_number(current: u32) -> Option<u32> {
        if current == MAX_SEQUENCE_NUMBER {
            // Need to flush the LSA and start over
            None
        } else {
            Some(current + 1)
        }
    }
    
    /// Compare sequence numbers according to RFC 2328
    pub fn compare_sequence_numbers(s1: u32, s2: u32) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        
        // Check if either is the initial value
        if s1 == INITIAL_SEQUENCE_NUMBER && s2 != INITIAL_SEQUENCE_NUMBER {
            return Ordering::Less;
        }
        if s2 == INITIAL_SEQUENCE_NUMBER && s1 != INITIAL_SEQUENCE_NUMBER {
            return Ordering::Greater;
        }
        
        // Normal comparison
        s1.cmp(&s2)
    }
    
    /// Check if MinLSInterval has elapsed since last update
    pub fn check_min_ls_interval(&self, _lsa_key: &str, last_update_time: f64) -> bool {
        self.current_time - last_update_time >= (MIN_LS_INTERVAL as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router::LSAData;
    
    #[test]
    fn test_age_calculation() {
        let mut manager = LSAAgeManager::new();
        manager.update_time(100.0);
        
        // Create a dummy LSA
        let lsa = LSA {
            header: LSAHeader {
                ls_age: 10,
                ls_type: crate::router::LSAType::RouterLSA,
                link_state_id: "1.1.1.1".to_string(),
                advertising_router: "1.1.1.1".to_string(),
                ls_sequence_number: INITIAL_SEQUENCE_NUMBER,
                ls_checksum: 0,
                length: 20,
            },
            data: LSAData::Router(crate::router::RouterLSA {
                flags: 0,
                num_links: 0,
                links: vec![],
            }),
        };
        
        // Record as received with InfTransDelay
        manager.set_interface_delay(1, 5);
        manager.record_received_lsa("test_lsa".to_string(), &lsa, Some(1));
        
        // Check initial age (should include InfTransDelay)
        let age = manager.calculate_age("test_lsa");
        assert_eq!(age, 15); // 10 + 5
        
        // Advance time and check age
        manager.update_time(200.0);
        let age = manager.calculate_age("test_lsa");
        assert_eq!(age, 115); // 15 + 100
    }
    
    #[test]
    fn test_maxage_handling() {
        let mut manager = LSAAgeManager::new();
        manager.update_time(0.0);
        
        let lsa = LSA {
            header: LSAHeader {
                ls_age: MAX_AGE - 100,
                ls_type: crate::router::LSAType::RouterLSA,
                link_state_id: "1.1.1.1".to_string(),
                advertising_router: "1.1.1.1".to_string(),
                ls_sequence_number: INITIAL_SEQUENCE_NUMBER,
                ls_checksum: 0,
                length: 20,
            },
            data: LSAData::Router(crate::router::RouterLSA {
                flags: 0,
                num_links: 0,
                links: vec![],
            }),
        };
        
        manager.record_received_lsa("test_lsa".to_string(), &lsa, None);
        
        // Advance time past MaxAge
        manager.update_time(200.0);
        let age = manager.calculate_age("test_lsa");
        assert_eq!(age, MAX_AGE);
        
        // Check that it's in MaxAge list
        let maxage_lsas = manager.get_maxage_lsas();
        assert!(maxage_lsas.contains(&"test_lsa".to_string()));
    }
    
    #[test]
    fn test_refresh_timing() {
        let mut manager = LSAAgeManager::new();
        manager.update_time(0.0);
        
        let lsa = LSA {
            header: LSAHeader {
                ls_age: 0,
                ls_type: crate::router::LSAType::RouterLSA,
                link_state_id: "1.1.1.1".to_string(),
                advertising_router: "1.1.1.1".to_string(),
                ls_sequence_number: INITIAL_SEQUENCE_NUMBER,
                ls_checksum: 0,
                length: 20,
            },
            data: LSAData::Router(crate::router::RouterLSA {
                flags: 0,
                num_links: 0,
                links: vec![],
            }),
        };
        
        // Record as self-originated
        manager.record_self_originated_lsa("test_lsa".to_string(), &lsa);
        
        // Should not need refresh yet
        assert!(!manager.needs_refresh("test_lsa"));
        
        // Advance to refresh time
        manager.update_time(LS_REFRESH_TIME as f64);
        assert!(manager.needs_refresh("test_lsa"));
        
        // Check refresh list
        let refresh_list = manager.get_lsas_needing_refresh();
        assert!(refresh_list.contains(&"test_lsa".to_string()));
    }
}