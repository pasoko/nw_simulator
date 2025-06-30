use crate::console_log;
use crate::network_type::OSPFNetworkType;

/// Represents a router's eligibility for DR/BDR election
#[derive(Debug, Clone)]
pub struct DRElectionCandidate {
    pub router_id: String,
    pub router_priority: u8,
    pub current_dr: String,
    pub current_bdr: String,
    pub interface_ip: String,
}

/// DR/BDR Election Manager
/// 
/// Implements the 2-stage DR/BDR election algorithm from RFC 2328 Section 9.4
pub struct DRElectionManager {
    router_id: String,
    router_priority: u8,
    designated_router: String,
    backup_designated_router: String,
    network_type: OSPFNetworkType,
}

impl DRElectionManager {
    pub fn new(router_id: String, router_priority: u8, network_type: OSPFNetworkType) -> Self {
        DRElectionManager {
            router_id: router_id.clone(),
            router_priority,
            designated_router: "0.0.0.0".to_string(),
            backup_designated_router: "0.0.0.0".to_string(),
            network_type,
        }
    }
    
    /// Check if DR/BDR election is required for the network type
    pub fn is_election_required(&self) -> bool {
        self.network_type.requires_dr_election()
    }
    
    /// Get current DR
    pub fn get_dr(&self) -> &str {
        &self.designated_router
    }
    
    /// Get current BDR
    pub fn get_bdr(&self) -> &str {
        &self.backup_designated_router
    }
    
    /// Run the DR/BDR election algorithm (RFC 2328 Section 9.4)
    pub fn run_election(&mut self, candidates: Vec<DRElectionCandidate>) -> (bool, String, String) {
        if !self.is_election_required() {
            console_log!("Router {} skipping DR election - network type {:?} doesn't require it", 
                self.router_id, self.network_type);
            return (false, self.designated_router.clone(), self.backup_designated_router.clone());
        }
        
        console_log!("Router {} running DR/BDR election with {} candidates", 
            self.router_id, candidates.len());
        
        // Stage 1: Calculate initial BDR (exclude routers declaring themselves as DR)
        let mut bdr_candidates: Vec<&DRElectionCandidate> = candidates.iter()
            .filter(|c| c.current_dr != c.router_id)  // Not declaring self as DR
            .filter(|c| c.router_priority > 0)  // Priority > 0
            .collect();
        
        let stage1_bdr = self.select_best_candidate(&mut bdr_candidates, true);
        
        // Stage 2: Calculate DR
        let mut dr_candidates: Vec<&DRElectionCandidate> = candidates.iter()
            .filter(|c| c.router_priority > 0)  // Priority > 0
            .filter(|c| {
                // Include if declaring self as DR or was selected as BDR in stage 1
                c.current_dr == c.router_id || 
                (stage1_bdr.is_some() && c.router_id == *stage1_bdr.as_ref().unwrap())
            })
            .collect();
        
        let new_dr = self.select_best_candidate(&mut dr_candidates, false);
        
        // Stage 3: Recalculate BDR (exclude new DR)
        let mut final_bdr_candidates: Vec<&DRElectionCandidate> = candidates.iter()
            .filter(|c| c.router_priority > 0)  // Priority > 0
            .filter(|c| {
                // Exclude new DR
                new_dr.is_none() || c.router_id != *new_dr.as_ref().unwrap()
            })
            .collect();
        
        let new_bdr = self.select_best_candidate(&mut final_bdr_candidates, true);
        
        // Update local state
        let old_dr = self.designated_router.clone();
        let old_bdr = self.backup_designated_router.clone();
        
        self.designated_router = new_dr.unwrap_or_else(|| "0.0.0.0".to_string());
        self.backup_designated_router = new_bdr.unwrap_or_else(|| "0.0.0.0".to_string());
        
        let changed = old_dr != self.designated_router || old_bdr != self.backup_designated_router;
        
        if changed {
            console_log!("Router {} DR election results changed: DR={}, BDR={} (was DR={}, BDR={})",
                self.router_id, self.designated_router, self.backup_designated_router,
                old_dr, old_bdr);
        }
        
        (changed, self.designated_router.clone(), self.backup_designated_router.clone())
    }
    
    /// Select the best candidate based on priority and router ID
    fn select_best_candidate(&self, candidates: &mut Vec<&DRElectionCandidate>, _for_bdr: bool) -> Option<String> {
        if candidates.is_empty() {
            return None;
        }
        
        // Sort by priority (descending), then by router ID (descending)
        candidates.sort_by(|a, b| {
            match b.router_priority.cmp(&a.router_priority) {
                std::cmp::Ordering::Equal => {
                    // Compare router IDs as IP addresses
                    let a_parts: Vec<u32> = a.router_id.split('.')
                        .filter_map(|s| s.parse().ok())
                        .collect();
                    let b_parts: Vec<u32> = b.router_id.split('.')
                        .filter_map(|s| s.parse().ok())
                        .collect();
                    
                    if a_parts.len() == 4 && b_parts.len() == 4 {
                        for i in 0..4 {
                            match b_parts[i].cmp(&a_parts[i]) {
                                std::cmp::Ordering::Equal => continue,
                                other => return other,
                            }
                        }
                    }
                    std::cmp::Ordering::Equal
                }
                other => other,
            }
        });
        
        candidates.first().map(|c| c.router_id.clone())
    }
    
    /// Update router priority
    pub fn set_priority(&mut self, priority: u8) {
        self.router_priority = priority;
    }
    
    /// Get router priority
    pub fn get_priority(&self) -> u8 {
        self.router_priority
    }
    
    /// Check if this router is the DR
    pub fn is_dr(&self) -> bool {
        self.designated_router == self.router_id && self.designated_router != "0.0.0.0"
    }
    
    /// Check if this router is the BDR
    pub fn is_bdr(&self) -> bool {
        self.backup_designated_router == self.router_id && self.backup_designated_router != "0.0.0.0"
    }
    
    /// Get the network type for this interface
    pub fn get_network_type(&self) -> OSPFNetworkType {
        self.network_type.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dr_election_basic() {
        let mut manager = DRElectionManager::new(
            "1.1.1.1".to_string(), 
            1, 
            OSPFNetworkType::Broadcast
        );
        
        let candidates = vec![
            DRElectionCandidate {
                router_id: "1.1.1.1".to_string(),
                router_priority: 1,
                current_dr: "0.0.0.0".to_string(),
                current_bdr: "0.0.0.0".to_string(),
                interface_ip: "10.0.0.1".to_string(),
            },
            DRElectionCandidate {
                router_id: "1.1.1.2".to_string(),
                router_priority: 2,
                current_dr: "0.0.0.0".to_string(),
                current_bdr: "0.0.0.0".to_string(),
                interface_ip: "10.0.0.2".to_string(),
            },
        ];
        
        let (changed, dr, bdr) = manager.run_election(candidates);
        assert!(changed);
        assert_eq!(dr, "1.1.1.2"); // Higher priority
        assert_eq!(bdr, "1.1.1.1");
    }
    
    #[test]
    fn test_no_election_for_p2p() {
        let mut manager = DRElectionManager::new(
            "1.1.1.1".to_string(), 
            1, 
            OSPFNetworkType::PointToPoint
        );
        
        let candidates = vec![];
        let (changed, dr, bdr) = manager.run_election(candidates);
        assert!(!changed);
        assert_eq!(dr, "0.0.0.0");
        assert_eq!(bdr, "0.0.0.0");
    }
}