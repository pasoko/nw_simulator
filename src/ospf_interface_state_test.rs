#[cfg(test)]
mod tests {
    use crate::ospf_interface_state::{ExtendedInterfaceState, InterfaceStateManager, OSPFInterfaceState};
    use crate::ospf_options::OSPFOptions;
    use crate::network_type::OSPFNetworkType;
    use crate::ospf_engine::OSPFEngine;
    use crate::simulation::NetworkSimulation;
    use crate::console_log;

    #[test]
    fn test_interface_state_transitions() {
        let mut state = ExtendedInterfaceState::new(
            "192.168.1.1".to_string(),
            "255.255.255.0".to_string(),
            "0.0.0.0".to_string(),
            OSPFNetworkType::Broadcast,
        );
        
        // Initial state should be Down
        assert_eq!(state.state, OSPFInterfaceState::Down);
        assert_eq!(state.stats.state_changes, 0);
        
        // Transition to Waiting
        state.transition_to_state(OSPFInterfaceState::Waiting, 1.0);
        assert_eq!(state.state, OSPFInterfaceState::Waiting);
        assert_eq!(state.stats.state_changes, 1);
        assert_eq!(state.last_state_change, 1.0);
        
        // Transition to DR
        state.transition_to_state(OSPFInterfaceState::DR, 5.0);
        assert_eq!(state.state, OSPFInterfaceState::DR);
        assert_eq!(state.stats.state_changes, 2);
        assert_eq!(state.last_state_change, 5.0);
        
        // Check state capabilities
        assert!(state.is_dr());
        assert!(!state.is_bdr());
        assert!(state.is_dr_or_bdr());
        assert!(state.should_participate_in_flooding());
    }

    #[test]
    fn test_interface_state_capabilities() {
        let mut state = ExtendedInterfaceState::new(
            "192.168.1.1".to_string(),
            "255.255.255.0".to_string(),
            "0.0.0.0".to_string(),
            OSPFNetworkType::Broadcast,
        );
        
        // Test Down state
        state.transition_to_state(OSPFInterfaceState::Down, 0.0);
        assert!(!state.should_send_hello());
        assert!(!state.state.can_form_adjacency());
        assert!(!state.state.participates_in_dr_election());
        
        // Test Waiting state
        state.transition_to_state(OSPFInterfaceState::Waiting, 1.0);
        assert!(state.should_send_hello());
        assert!(!state.state.can_form_adjacency());
        assert!(state.state.participates_in_dr_election());
        
        // Test DR state
        state.transition_to_state(OSPFInterfaceState::DR, 2.0);
        assert!(state.should_send_hello());
        assert!(state.state.can_form_adjacency());
        assert!(state.state.participates_in_dr_election());
        assert!(state.state.should_flood_lsa());
        
        // Test Point-to-Point state
        state.transition_to_state(OSPFInterfaceState::PointToPoint, 3.0);
        assert!(state.should_send_hello());
        assert!(state.state.can_form_adjacency());
        assert!(!state.state.participates_in_dr_election());
        assert!(state.state.should_flood_lsa());
    }

    #[test]
    fn test_neighbor_management() {
        let mut state = ExtendedInterfaceState::new(
            "192.168.1.1".to_string(),
            "255.255.255.0".to_string(),
            "0.0.0.0".to_string(),
            OSPFNetworkType::Broadcast,
        );
        
        // Add neighbors
        state.add_neighbor("2.2.2.2".to_string());
        state.add_neighbor("3.3.3.3".to_string());
        assert_eq!(state.neighbors.len(), 2);
        assert_eq!(state.fully_adjacent_neighbors.len(), 0);
        
        // Mark one neighbor as full
        state.mark_neighbor_full("2.2.2.2".to_string());
        assert_eq!(state.fully_adjacent_neighbors.len(), 1);
        assert_eq!(state.get_adjacency_count(), 1);
        assert_eq!(state.stats.adjacencies_formed, 1);
        
        // Remove neighbor
        state.remove_neighbor("2.2.2.2");
        assert_eq!(state.neighbors.len(), 1);
        assert_eq!(state.fully_adjacent_neighbors.len(), 0);
        assert_eq!(state.stats.adjacencies_lost, 1);
        
        // Try to mark non-existent neighbor as full
        state.mark_neighbor_full("4.4.4.4".to_string());
        assert_eq!(state.fully_adjacent_neighbors.len(), 0);
    }

    #[test]
    fn test_wait_timer() {
        let mut state = ExtendedInterfaceState::new(
            "192.168.1.1".to_string(),
            "255.255.255.0".to_string(),
            "0.0.0.0".to_string(),
            OSPFNetworkType::Broadcast,
        );
        
        // Initially no wait timer
        assert!(state.wait_timer.is_none());
        assert!(!state.is_wait_timer_expired(10.0));
        
        // Start wait timer
        state.start_wait_timer(5.0);
        assert!(state.wait_timer.is_some());
        assert_eq!(state.wait_timer.unwrap(), 45.0); // 5.0 + 40 (dead_interval)
        
        // Check timer not expired
        assert!(!state.is_wait_timer_expired(40.0));
        
        // Check timer expired
        assert!(state.is_wait_timer_expired(50.0));
        
        // Transition to Waiting state first
        state.transition_to_state(OSPFInterfaceState::Waiting, 45.0);
        assert!(state.wait_timer.is_some());
        
        // Then transition out of Waiting clears timer
        state.transition_to_state(OSPFInterfaceState::DR, 50.0);
        assert!(state.wait_timer.is_none());
    }

    #[test]
    fn test_dr_bdr_updates() {
        let mut state = ExtendedInterfaceState::new(
            "192.168.1.1".to_string(),
            "255.255.255.0".to_string(),
            "0.0.0.0".to_string(),
            OSPFNetworkType::Broadcast,
        );
        
        // Initial DR/BDR should be 0.0.0.0
        assert_eq!(state.designated_router, "0.0.0.0");
        assert_eq!(state.backup_designated_router, "0.0.0.0");
        
        // Update DR/BDR
        state.update_dr_bdr("192.168.1.2".to_string(), "192.168.1.3".to_string());
        assert_eq!(state.designated_router, "192.168.1.2");
        assert_eq!(state.backup_designated_router, "192.168.1.3");
    }

    #[test]
    fn test_interface_configuration() {
        let mut state = ExtendedInterfaceState::new(
            "192.168.1.1".to_string(),
            "255.255.255.0".to_string(),
            "0.0.0.0".to_string(),
            OSPFNetworkType::Broadcast,
        );
        
        // Test cost update
        assert_eq!(state.cost, 10);
        state.update_cost(100);
        assert_eq!(state.cost, 100);
        
        // Test priority update
        assert_eq!(state.priority, 1);
        state.update_priority(255);
        assert_eq!(state.priority, 255);
        
        // Test hello interval update
        assert_eq!(state.hello_interval, 10);
        state.update_hello_interval(30);
        assert_eq!(state.hello_interval, 30);
        
        // Test dead interval update
        assert_eq!(state.dead_interval, 40);
        state.update_dead_interval(120);
        assert_eq!(state.dead_interval, 120);
        
        // Test options update
        let mut custom_options = OSPFOptions::new();
        custom_options.set_mc_bit(true);
        state.update_options(custom_options);
        assert!(state.options.get_mc_bit());
        
        // Test stub configuration
        assert!(!state.is_stub);
        state.set_stub(true);
        assert!(state.is_stub);
        
        // Test passive configuration
        assert!(!state.is_passive);
        state.set_passive(true);
        assert!(state.is_passive);
        assert!(!state.should_send_hello());
    }

    #[test]
    fn test_statistics_tracking() {
        let mut state = ExtendedInterfaceState::new(
            "192.168.1.1".to_string(),
            "255.255.255.0".to_string(),
            "0.0.0.0".to_string(),
            OSPFNetworkType::Broadcast,
        );
        
        // Test packet statistics
        state.record_hello_sent();
        state.record_hello_received();
        state.record_dd_sent();
        state.record_dd_received();
        state.record_lsa_update_sent();
        state.record_lsa_update_received();
        state.record_lsa_ack_sent();
        state.record_lsa_ack_received();
        
        assert_eq!(state.stats.hello_packets_sent, 1);
        assert_eq!(state.stats.hello_packets_received, 1);
        assert_eq!(state.stats.dd_packets_sent, 1);
        assert_eq!(state.stats.dd_packets_received, 1);
        assert_eq!(state.stats.lsa_updates_sent, 1);
        assert_eq!(state.stats.lsa_updates_received, 1);
        assert_eq!(state.stats.lsa_acks_sent, 1);
        assert_eq!(state.stats.lsa_acks_received, 1);
        
        // Test reset
        state.reset_statistics();
        assert_eq!(state.stats.hello_packets_sent, 0);
        assert_eq!(state.stats.hello_packets_received, 0);
    }

    #[test]
    fn test_interface_state_manager() {
        let mut manager = InterfaceStateManager::new();
        
        // Add interfaces
        let state1 = ExtendedInterfaceState::new(
            "192.168.1.1".to_string(),
            "255.255.255.0".to_string(),
            "0.0.0.0".to_string(),
            OSPFNetworkType::Broadcast,
        );
        let mut state2 = ExtendedInterfaceState::new(
            "192.168.2.1".to_string(),
            "255.255.255.0".to_string(),
            "0.0.0.0".to_string(),
            OSPFNetworkType::Broadcast,
        );
        state2.transition_to_state(OSPFInterfaceState::DR, 0.0);
        
        manager.add_interface(1, state1);
        manager.add_interface(2, state2);
        
        // Test retrieval
        assert!(manager.get_interface(1).is_some());
        assert!(manager.get_interface(2).is_some());
        assert!(manager.get_interface(3).is_none());
        
        // Test by state
        let down_interfaces = manager.get_interfaces_by_state(OSPFInterfaceState::Down);
        assert_eq!(down_interfaces.len(), 1);
        
        let dr_interfaces = manager.get_dr_interfaces();
        assert_eq!(dr_interfaces.len(), 1);
        assert_eq!(dr_interfaces[0].0, 2);
        
        let bdr_interfaces = manager.get_bdr_interfaces();
        assert_eq!(bdr_interfaces.len(), 0);
        
        // Test counts
        assert_eq!(manager.get_interface_count_by_state(OSPFInterfaceState::Down), 1);
        assert_eq!(manager.get_interface_count_by_state(OSPFInterfaceState::DR), 1);
        
        // Test removal
        manager.remove_interface(1);
        assert!(manager.get_interface(1).is_none());
        assert_eq!(manager.get_interfaces_by_state(OSPFInterfaceState::Down).len(), 0);
    }

    #[test]
    fn test_wait_timer_management() {
        let mut manager = InterfaceStateManager::new();
        manager.update_time(0.0);
        
        // Add interface with wait timer
        let mut state = ExtendedInterfaceState::new(
            "192.168.1.1".to_string(),
            "255.255.255.0".to_string(),
            "0.0.0.0".to_string(),
            OSPFNetworkType::Broadcast,
        );
        state.start_wait_timer(5.0);
        manager.add_interface(1, state);
        
        // Check timer not expired
        manager.update_time(30.0);
        let expired = manager.check_wait_timers();
        assert_eq!(expired.len(), 0);
        
        // Check timer expired
        manager.update_time(50.0);
        let expired = manager.check_wait_timers();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], 1);
    }

    #[test]
    fn test_ospf_engine_integration() {
        let mut engine = OSPFEngine::new("1.1.1.1".to_string(), "0.0.0.0".to_string());
        
        // Initialize interface state
        engine.initialize_interface_state(1, "192.168.1.1".to_string(), "255.255.255.0".to_string());
        
        // Check extended state was created
        assert!(engine.get_extended_interface_state(1).is_some());
        
        // Test state transition
        engine.transition_interface_state(1, OSPFInterfaceState::Waiting);
        let state = engine.get_extended_interface_state(1).unwrap();
        assert_eq!(state.state, OSPFInterfaceState::Waiting);
        
        // Test neighbor management
        engine.update_interface_neighbor(1, "2.2.2.2".to_string(), false);
        engine.update_interface_neighbor(1, "2.2.2.2".to_string(), true);
        let state = engine.get_extended_interface_state(1).unwrap();
        assert_eq!(state.neighbors.len(), 1);
        assert_eq!(state.fully_adjacent_neighbors.len(), 1);
        
        // Test neighbor removal
        engine.remove_interface_neighbor(1, "2.2.2.2");
        let state = engine.get_extended_interface_state(1).unwrap();
        assert_eq!(state.neighbors.len(), 0);
        assert_eq!(state.fully_adjacent_neighbors.len(), 0);
        
        // Test interface configuration
        engine.set_interface_passive(1, true);
        let state = engine.get_extended_interface_state(1).unwrap();
        assert!(state.is_passive);
        
        engine.set_interface_stub(1, true);
        let state = engine.get_extended_interface_state(1).unwrap();
        assert!(state.is_stub);
        
        // Test DR/BDR updates
        engine.update_interface_dr_bdr(1, "192.168.1.2".to_string(), "192.168.1.3".to_string());
        let state = engine.get_extended_interface_state(1).unwrap();
        assert_eq!(state.designated_router, "192.168.1.2");
        assert_eq!(state.backup_designated_router, "192.168.1.3");
    }

    #[test]
    fn test_interface_state_summary() {
        let mut state = ExtendedInterfaceState::new(
            "192.168.1.1".to_string(),
            "255.255.255.0".to_string(),
            "0.0.0.0".to_string(),
            OSPFNetworkType::Broadcast,
        );
        
        state.transition_to_state(OSPFInterfaceState::DR, 0.0);
        state.add_neighbor("2.2.2.2".to_string());
        state.mark_neighbor_full("2.2.2.2".to_string());
        state.update_dr_bdr("192.168.1.1".to_string(), "192.168.1.2".to_string());
        
        let summary = state.get_summary();
        assert!(summary.contains("State: DR"));
        assert!(summary.contains("Type: Broadcast"));
        assert!(summary.contains("IP: 192.168.1.1"));
        assert!(summary.contains("Neighbors: 1"));
        assert!(summary.contains("Adjacent: 1"));
        assert!(summary.contains("DR: 192.168.1.1"));
        assert!(summary.contains("BDR: 192.168.1.2"));
    }

    #[test]
    fn test_simulation_integration() {
        let mut sim = NetworkSimulation::new();
        
        // Add routers
        let r1 = sim.add_router("R1".to_string(), 0.0, 0.0);
        let r2 = sim.add_router("R2".to_string(), 100.0, 0.0);
        
        // Connect routers
        sim.connect_routers(r1, r2, 10).unwrap();
        
        // Enable OSPF
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        // Test interface state access through engine
        if let Some(engine) = sim.get_ospf_engine(r1) {
            let interface_states = engine.get_all_interface_states();
            assert!(!interface_states.is_empty());
            
            console_log!("R1 interface states: {:?}", interface_states);
        }
    }
}