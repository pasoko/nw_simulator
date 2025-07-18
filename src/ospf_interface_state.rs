use std::collections::HashSet;
use serde::{Serialize, Deserialize};
use crate::ospf_options::OSPFOptions;
use crate::network_type::OSPFNetworkType;

/// OSPFv2 Interface State Management (RFC 2328 Section 9)
/// 
/// This module provides comprehensive interface state management for OSPF interfaces,
/// including state transitions, neighbor tracking, and protocol parameters.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OSPFInterfaceState {
    /// Interface is down or administratively disabled
    Down = 0,
    /// Interface is looped back
    Loopback = 1,
    /// Interface is waiting for timer to determine DR/BDR
    Waiting = 2,
    /// Interface is on point-to-point network
    PointToPoint = 3,
    /// Interface is on broadcast network but not DR/BDR
    DROther = 4,
    /// Interface is Backup Designated Router
    Backup = 5,
    /// Interface is Designated Router
    DR = 6,
}

impl OSPFInterfaceState {
    /// Check if interface can form adjacencies
    pub fn can_form_adjacency(&self) -> bool {
        matches!(self, 
            OSPFInterfaceState::PointToPoint |
            OSPFInterfaceState::Backup |
            OSPFInterfaceState::DR
        )
    }
    
    /// Check if interface participates in DR election
    pub fn participates_in_dr_election(&self) -> bool {
        matches!(self, 
            OSPFInterfaceState::Waiting |
            OSPFInterfaceState::DROther |
            OSPFInterfaceState::Backup |
            OSPFInterfaceState::DR
        )
    }
    
    /// Check if interface should flood LSAs
    pub fn should_flood_lsa(&self) -> bool {
        matches!(self, 
            OSPFInterfaceState::PointToPoint |
            OSPFInterfaceState::Backup |
            OSPFInterfaceState::DR
        )
    }
}

/// Extended interface state with RFC 2328 compliance
#[derive(Debug, Clone)]
pub struct ExtendedInterfaceState {
    /// Current OSPF interface state
    pub state: OSPFInterfaceState,
    
    /// Network type for this interface
    pub network_type: OSPFNetworkType,
    
    /// Interface IP address
    pub interface_ip: String,
    
    /// Network mask for this interface
    pub network_mask: String,
    
    /// Area ID this interface belongs to
    pub area_id: String,
    
    /// OSPF options for this interface
    pub options: OSPFOptions,
    
    /// Interface priority for DR election
    pub priority: u8,
    
    /// Hello interval in seconds
    pub hello_interval: u16,
    
    /// Dead interval in seconds
    pub dead_interval: u32,
    
    /// Interface transmission delay
    pub inf_trans_delay: u16,
    
    /// Retransmission interval
    pub rxmt_interval: u16,
    
    /// Interface cost
    pub cost: u32,
    
    /// Designated Router IP
    pub designated_router: String,
    
    /// Backup Designated Router IP
    pub backup_designated_router: String,
    
    /// List of neighbors on this interface
    pub neighbors: HashSet<String>,
    
    /// List of fully adjacent neighbors
    pub fully_adjacent_neighbors: HashSet<String>,
    
    /// Interface state change timestamp
    pub last_state_change: f64,
    
    /// Wait timer for DR election
    pub wait_timer: Option<f64>,
    
    /// Interface authentication enabled
    pub auth_enabled: bool,
    
    /// Interface MTU
    pub mtu: u16,
    
    /// Whether this interface is a stub interface
    pub is_stub: bool,
    
    /// Whether this interface is passive (no hello packets)
    pub is_passive: bool,
    
    /// Statistics
    pub stats: InterfaceStatistics,
}

#[derive(Debug, Clone, Default)]
pub struct InterfaceStatistics {
    /// Number of hello packets sent
    pub hello_packets_sent: u64,
    
    /// Number of hello packets received
    pub hello_packets_received: u64,
    
    /// Number of DD packets sent
    pub dd_packets_sent: u64,
    
    /// Number of DD packets received
    pub dd_packets_received: u64,
    
    /// Number of LSA updates sent
    pub lsa_updates_sent: u64,
    
    /// Number of LSA updates received
    pub lsa_updates_received: u64,
    
    /// Number of LSA acknowledgments sent
    pub lsa_acks_sent: u64,
    
    /// Number of LSA acknowledgments received
    pub lsa_acks_received: u64,
    
    /// Number of state changes
    pub state_changes: u32,
    
    /// Number of neighbor adjacencies formed
    pub adjacencies_formed: u32,
    
    /// Number of neighbor adjacencies lost
    pub adjacencies_lost: u32,
    
    /// Time spent in current state
    pub time_in_current_state: f64,
}

impl ExtendedInterfaceState {
    /// Create new interface state with default values
    pub fn new(
        interface_ip: String,
        network_mask: String,
        area_id: String,
        network_type: OSPFNetworkType,
    ) -> Self {
        ExtendedInterfaceState {
            state: OSPFInterfaceState::Down,
            network_type,
            interface_ip,
            network_mask,
            area_id,
            options: OSPFOptions::standard_area_options(),
            priority: 1,
            hello_interval: 10,
            dead_interval: 40,
            inf_trans_delay: 1,
            rxmt_interval: 5,
            cost: 10,
            designated_router: "0.0.0.0".to_string(),
            backup_designated_router: "0.0.0.0".to_string(),
            neighbors: HashSet::new(),
            fully_adjacent_neighbors: HashSet::new(),
            last_state_change: 0.0,
            wait_timer: None,
            auth_enabled: false,
            mtu: 1500,
            is_stub: false,
            is_passive: false,
            stats: InterfaceStatistics::default(),
        }
    }
    
    /// Transition to a new interface state
    pub fn transition_to_state(&mut self, new_state: OSPFInterfaceState, current_time: f64) {
        if self.state != new_state {
            let old_state = self.state;
            self.state = new_state;
            self.last_state_change = current_time;
            self.stats.state_changes += 1;
            
            // Clear wait timer when leaving Waiting state
            if old_state == OSPFInterfaceState::Waiting && new_state != OSPFInterfaceState::Waiting {
                self.wait_timer = None;
            }
        }
    }
    
    /// Check if interface should send hello packets
    pub fn should_send_hello(&self) -> bool {
        !self.is_passive && 
        !matches!(self.state, OSPFInterfaceState::Down | OSPFInterfaceState::Loopback)
    }
    
    /// Check if interface should participate in flooding
    pub fn should_participate_in_flooding(&self) -> bool {
        self.state.should_flood_lsa()
    }
    
    /// Add a neighbor to the interface
    pub fn add_neighbor(&mut self, neighbor_id: String) {
        self.neighbors.insert(neighbor_id);
    }
    
    /// Remove a neighbor from the interface
    pub fn remove_neighbor(&mut self, neighbor_id: &str) {
        self.neighbors.remove(neighbor_id);
        if self.fully_adjacent_neighbors.contains(neighbor_id) {
            self.fully_adjacent_neighbors.remove(neighbor_id);
            self.stats.adjacencies_lost += 1;
        }
    }
    
    /// Mark a neighbor as fully adjacent
    pub fn mark_neighbor_full(&mut self, neighbor_id: String) {
        if self.neighbors.contains(&neighbor_id) {
            self.fully_adjacent_neighbors.insert(neighbor_id);
            self.stats.adjacencies_formed += 1;
        }
    }
    
    /// Mark a neighbor as not fully adjacent
    pub fn mark_neighbor_not_full(&mut self, neighbor_id: &str) {
        if self.fully_adjacent_neighbors.contains(neighbor_id) {
            self.fully_adjacent_neighbors.remove(neighbor_id);
            self.stats.adjacencies_lost += 1;
        }
    }
    
    /// Get the number of fully adjacent neighbors
    pub fn get_adjacency_count(&self) -> usize {
        self.fully_adjacent_neighbors.len()
    }
    
    /// Check if this interface is the DR
    pub fn is_dr(&self) -> bool {
        matches!(self.state, OSPFInterfaceState::DR)
    }
    
    /// Check if this interface is the BDR
    pub fn is_bdr(&self) -> bool {
        matches!(self.state, OSPFInterfaceState::Backup)
    }
    
    /// Check if this interface is DR or BDR
    pub fn is_dr_or_bdr(&self) -> bool {
        self.is_dr() || self.is_bdr()
    }
    
    /// Update DR and BDR information
    pub fn update_dr_bdr(&mut self, dr_ip: String, bdr_ip: String) {
        self.designated_router = dr_ip;
        self.backup_designated_router = bdr_ip;
    }
    
    /// Start wait timer for DR election
    pub fn start_wait_timer(&mut self, current_time: f64) {
        self.wait_timer = Some(current_time + self.dead_interval as f64);
    }
    
    /// Check if wait timer has expired
    pub fn is_wait_timer_expired(&self, current_time: f64) -> bool {
        if let Some(timer) = self.wait_timer {
            current_time >= timer
        } else {
            false
        }
    }
    
    /// Update interface cost
    pub fn update_cost(&mut self, new_cost: u32) {
        self.cost = new_cost;
    }
    
    /// Update interface priority
    pub fn update_priority(&mut self, new_priority: u8) {
        self.priority = new_priority;
    }
    
    /// Update hello interval
    pub fn update_hello_interval(&mut self, new_interval: u16) {
        self.hello_interval = new_interval;
    }
    
    /// Update dead interval
    pub fn update_dead_interval(&mut self, new_interval: u32) {
        self.dead_interval = new_interval;
    }
    
    /// Update interface options
    pub fn update_options(&mut self, new_options: OSPFOptions) {
        self.options = new_options;
    }
    
    /// Set interface as stub
    pub fn set_stub(&mut self, is_stub: bool) {
        self.is_stub = is_stub;
    }
    
    /// Set interface as passive
    pub fn set_passive(&mut self, is_passive: bool) {
        self.is_passive = is_passive;
    }
    
    /// Get interface state as string
    pub fn state_as_string(&self) -> &'static str {
        match self.state {
            OSPFInterfaceState::Down => "Down",
            OSPFInterfaceState::Loopback => "Loopback",
            OSPFInterfaceState::Waiting => "Waiting",
            OSPFInterfaceState::PointToPoint => "Point-to-Point",
            OSPFInterfaceState::DROther => "DROther",
            OSPFInterfaceState::Backup => "Backup",
            OSPFInterfaceState::DR => "DR",
        }
    }
    
    /// Get interface information as a summary string
    pub fn get_summary(&self) -> String {
        format!(
            "State: {}, Type: {:?}, IP: {}, Neighbors: {}, Adjacent: {}, DR: {}, BDR: {}",
            self.state_as_string(),
            self.network_type,
            self.interface_ip,
            self.neighbors.len(),
            self.fully_adjacent_neighbors.len(),
            self.designated_router,
            self.backup_designated_router
        )
    }
    
    /// Record hello packet sent
    pub fn record_hello_sent(&mut self) {
        self.stats.hello_packets_sent += 1;
    }
    
    /// Record hello packet received
    pub fn record_hello_received(&mut self) {
        self.stats.hello_packets_received += 1;
    }
    
    /// Record DD packet sent
    pub fn record_dd_sent(&mut self) {
        self.stats.dd_packets_sent += 1;
    }
    
    /// Record DD packet received
    pub fn record_dd_received(&mut self) {
        self.stats.dd_packets_received += 1;
    }
    
    /// Record LSA update sent
    pub fn record_lsa_update_sent(&mut self) {
        self.stats.lsa_updates_sent += 1;
    }
    
    /// Record LSA update received
    pub fn record_lsa_update_received(&mut self) {
        self.stats.lsa_updates_received += 1;
    }
    
    /// Record LSA acknowledgment sent
    pub fn record_lsa_ack_sent(&mut self) {
        self.stats.lsa_acks_sent += 1;
    }
    
    /// Record LSA acknowledgment received
    pub fn record_lsa_ack_received(&mut self) {
        self.stats.lsa_acks_received += 1;
    }
    
    /// Reset statistics
    pub fn reset_statistics(&mut self) {
        self.stats = InterfaceStatistics::default();
    }
}

/// Interface state manager for handling multiple interfaces
#[derive(Debug)]
pub struct InterfaceStateManager {
    interfaces: std::collections::HashMap<u32, ExtendedInterfaceState>,
    current_time: f64,
}

impl InterfaceStateManager {
    /// Create new interface state manager
    pub fn new() -> Self {
        InterfaceStateManager {
            interfaces: std::collections::HashMap::new(),
            current_time: 0.0,
        }
    }
    
    /// Update current time
    pub fn update_time(&mut self, time: f64) {
        self.current_time = time;
    }
    
    /// Add interface to management
    pub fn add_interface(&mut self, interface_id: u32, state: ExtendedInterfaceState) {
        self.interfaces.insert(interface_id, state);
    }
    
    /// Remove interface from management
    pub fn remove_interface(&mut self, interface_id: u32) {
        self.interfaces.remove(&interface_id);
    }
    
    /// Get interface state
    pub fn get_interface(&self, interface_id: u32) -> Option<&ExtendedInterfaceState> {
        self.interfaces.get(&interface_id)
    }
    
    /// Get mutable interface state
    pub fn get_interface_mut(&mut self, interface_id: u32) -> Option<&mut ExtendedInterfaceState> {
        self.interfaces.get_mut(&interface_id)
    }
    
    /// Get all interfaces
    pub fn get_all_interfaces(&self) -> &std::collections::HashMap<u32, ExtendedInterfaceState> {
        &self.interfaces
    }
    
    /// Get interfaces by state
    pub fn get_interfaces_by_state(&self, state: OSPFInterfaceState) -> Vec<(u32, &ExtendedInterfaceState)> {
        self.interfaces
            .iter()
            .filter(|(_, iface)| iface.state == state)
            .map(|(id, iface)| (*id, iface))
            .collect()
    }
    
    /// Get DR interfaces
    pub fn get_dr_interfaces(&self) -> Vec<(u32, &ExtendedInterfaceState)> {
        self.get_interfaces_by_state(OSPFInterfaceState::DR)
    }
    
    /// Get BDR interfaces
    pub fn get_bdr_interfaces(&self) -> Vec<(u32, &ExtendedInterfaceState)> {
        self.get_interfaces_by_state(OSPFInterfaceState::Backup)
    }
    
    /// Check for expired wait timers
    pub fn check_wait_timers(&mut self) -> Vec<u32> {
        let current_time = self.current_time;
        self.interfaces
            .iter()
            .filter_map(|(id, iface)| {
                if iface.is_wait_timer_expired(current_time) {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Get interface count by state
    pub fn get_interface_count_by_state(&self, state: OSPFInterfaceState) -> usize {
        self.interfaces.values().filter(|iface| iface.state == state).count()
    }
    
    /// Get total adjacency count
    pub fn get_total_adjacency_count(&self) -> usize {
        self.interfaces.values().map(|iface| iface.get_adjacency_count()).sum()
    }
}