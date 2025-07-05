use std::collections::HashMap;
use crate::router::{OSPFNeighbor, OSPFNeighborState};
use crate::console_log;

/// OSPF Neighbor State Management
/// 
/// Handles OSPF neighbor discovery, state transitions, and dead neighbor detection.
/// Implements the OSPF neighbor state machine according to RFC 2328.
/// 
/// States: Down -> Init -> 2Way -> ExStart -> Exchange -> Loading -> Full
pub struct OSPFNeighborManager {
    neighbors: HashMap<u32, OSPFNeighbor>,
    neighbor_last_hello: HashMap<u32, f64>,
    neighbor_previous_state: HashMap<u32, OSPFNeighborState>,
    dead_interval: u32,
    current_time: f64,
}

impl OSPFNeighborManager {
    pub fn new(dead_interval: u32) -> Self {
        OSPFNeighborManager {
            neighbors: HashMap::new(),
            neighbor_last_hello: HashMap::new(),
            neighbor_previous_state: HashMap::new(),
            dead_interval,
            current_time: 0.0,
        }
    }
    
    pub fn update_time(&mut self, time: f64) {
        self.current_time = time;
        self.check_dead_neighbors();
    }
    
    pub fn add_or_update_neighbor(&mut self, neighbor_id: u32, interface_id: u32, priority: u8) -> bool {
        let is_new = !self.neighbors.contains_key(&neighbor_id);
        
        if is_new {
            let new_neighbor = OSPFNeighbor {
                router_id: format!("{}.{}.{}.{}", 1, 1, 1, neighbor_id),
                state: OSPFNeighborState::Down,
                interface_id,
                priority,
            };
            self.neighbors.insert(neighbor_id, new_neighbor);
            self.neighbor_previous_state.insert(neighbor_id, OSPFNeighborState::Down);
        } else {
            // Update existing neighbor priority
            if let Some(neighbor) = self.neighbors.get_mut(&neighbor_id) {
                neighbor.priority = priority;
            }
        }
        
        self.neighbor_last_hello.insert(neighbor_id, self.current_time);
        is_new
    }
    
    pub fn update_neighbor_state(&mut self, neighbor_id: u32, new_state: OSPFNeighborState) -> bool {
        if let Some(neighbor) = self.neighbors.get_mut(&neighbor_id) {
            let old_state = neighbor.state.clone();
            self.neighbor_previous_state.insert(neighbor_id, old_state.clone());
            neighbor.state = new_state.clone();
            
            // Only log significant state changes
            if old_state != new_state {
                console_log!("Neighbor {} state changed: {:?} -> {:?}", 
                    neighbor_id, old_state, new_state);
            }
            
            return old_state != new_state;
        }
        false
    }
    
    pub fn get_neighbor_state(&self, neighbor_id: u32) -> Option<OSPFNeighborState> {
        self.neighbors.get(&neighbor_id).map(|n| n.state.clone())
    }
    
    pub fn remove_neighbor(&mut self, neighbor_id: u32) -> bool {
        let removed = self.neighbors.remove(&neighbor_id).is_some();
        if removed {
            self.neighbor_last_hello.remove(&neighbor_id);
            self.neighbor_previous_state.remove(&neighbor_id);
            console_log!("Neighbor {} removed", neighbor_id);
        }
        removed
    }
    
    pub fn get_neighbors_in_state(&self, state: OSPFNeighborState) -> Vec<u32> {
        self.neighbors.iter()
            .filter(|(_, neighbor)| neighbor.state == state)
            .map(|(id, _)| *id)
            .collect()
    }
    
    pub fn get_all_active_neighbors(&self) -> Vec<String> {
        let neighbors: Vec<String> = self.neighbors.iter()
            .filter(|(_, n)| n.state != OSPFNeighborState::Down)
            .map(|(id, _)| format!("{}.{}.{}.{}", 1, 1, 1, id))
            .collect();
        
        // Remove frequent logging to improve performance
        
        neighbors
    }
    
    pub fn get_neighbor_count(&self) -> usize {
        self.neighbors.len()
    }
    
    /// Get all neighbor IDs (regardless of state)
    pub fn get_all_neighbor_ids(&self) -> Vec<u32> {
        self.neighbors.keys().cloned().collect()
    }
    
    pub fn get_neighbors(&self) -> &HashMap<u32, OSPFNeighbor> {
        &self.neighbors
    }
    
    pub fn get_state_transitions(&mut self) -> HashMap<u32, (OSPFNeighborState, OSPFNeighborState)> {
        let mut transitions = HashMap::new();
        
        for (id, neighbor) in &self.neighbors {
            if let Some(prev_state) = self.neighbor_previous_state.get(id) {
                // Only report if state actually changed
                if prev_state != &neighbor.state {
                    transitions.insert(*id, (prev_state.clone(), neighbor.state.clone()));
                }
            }
        }
        
        // Update previous states to current states for next comparison
        for (id, neighbor) in &self.neighbors {
            self.neighbor_previous_state.insert(*id, neighbor.state.clone());
        }
        
        transitions
    }
    
    fn check_dead_neighbors(&mut self) {
        let mut dead_neighbors = Vec::new();
        
        for (id, last_hello) in &self.neighbor_last_hello {
            let time_since_hello = self.current_time - last_hello;
            if time_since_hello > self.dead_interval as f64 {
                dead_neighbors.push(*id);
                console_log!("Marking neighbor {} as dead - last hello {:.1}s ago", 
                    id, time_since_hello);
            }
        }
        
        for id in dead_neighbors {
            if let Some(neighbor) = self.neighbors.get_mut(&id) {
                if neighbor.state != OSPFNeighborState::Down {
                    self.neighbor_previous_state.insert(id, neighbor.state.clone());
                    neighbor.state = OSPFNeighborState::Down;
                    console_log!("Neighbor {} went down due to dead timer", id);
                }
            }
            // Don't remove from neighbor_last_hello here - let the neighbor be properly removed
        }
    }
    
    /// State machine progression logic
    pub fn progress_neighbor_state(&mut self, neighbor_id: u32, hello_neighbors: &[String], router_id: &str) -> bool {
        if let Some(current_state) = self.get_neighbor_state(neighbor_id) {
            let new_state = match current_state {
                OSPFNeighborState::Down => {
                    // Move to Init state - we heard from them
                    console_log!("Neighbor {} progressing from Down to Init", neighbor_id);
                    Some(OSPFNeighborState::Init)
                }
                OSPFNeighborState::Init => {
                    // Check if we are in neighbor's hello packet
                    if hello_neighbors.contains(&router_id.to_string()) {
                        console_log!("Neighbor {} progressing from Init to TwoWay (bidirectional communication confirmed)", neighbor_id);
                        Some(OSPFNeighborState::TwoWay)
                    } else {
                        console_log!("Neighbor {} staying in Init state (bidirectional communication not confirmed)", neighbor_id);
                        None
                    }
                }
                OSPFNeighborState::TwoWay => {
                    // In TwoWay state, maintain bidirectional communication
                    if !hello_neighbors.contains(&router_id.to_string()) {
                        console_log!("Neighbor {} lost bidirectional communication, moving back to Init", neighbor_id);
                        Some(OSPFNeighborState::Init)
                    } else {
                        None
                    }
                }
                OSPFNeighborState::ExStart | OSPFNeighborState::Exchange | 
                OSPFNeighborState::Loading | OSPFNeighborState::Full => {
                    // Higher states should NEVER be downgraded by hello processing
                    // These states are managed by DD exchange and LSA synchronization
                    if !hello_neighbors.contains(&router_id.to_string()) {
                        console_log!("Warning: Neighbor {} in state {:?} but bidirectional communication lost - NOT downgrading state", 
                            neighbor_id, current_state);
                    }
                    None // Don't change state for active adjacencies
                }
            };
            
            if let Some(state) = new_state {
                return self.update_neighbor_state(neighbor_id, state);
            }
        }
        false
    }
    
    /// Check if neighbor should form adjacency (simplified for point-to-point)
    pub fn should_form_adjacency(&self, neighbor_id: u32) -> bool {
        matches!(self.get_neighbor_state(neighbor_id), Some(OSPFNeighborState::TwoWay))
    }
    
    /// Start adjacency formation
    pub fn start_adjacency(&mut self, neighbor_id: u32) -> bool {
        self.update_neighbor_state(neighbor_id, OSPFNeighborState::ExStart)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neighbor_lifecycle() {
        let mut manager = OSPFNeighborManager::new(40);
        
        // Add neighbor
        assert!(manager.add_or_update_neighbor(1, 1, 1));
        assert_eq!(manager.get_neighbor_count(), 1);
        
        // Progress through states
        assert!(manager.update_neighbor_state(1, OSPFNeighborState::Init));
        assert_eq!(manager.get_neighbor_state(1), Some(OSPFNeighborState::Init));
        
        // Remove neighbor
        assert!(manager.remove_neighbor(1));
        assert_eq!(manager.get_neighbor_count(), 0);
    }
}