use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use crate::network::NetworkTopology;
use crate::protocol::{ProtocolEngine, PacketEvent, ProtocolPacket};
use crate::ospf::{OSPFPacket, OSPFPacketType, OSPFPacketData, HelloPacket};
use crate::ospf_engine::OSPFEngine;
use crate::spf::SPFCalculator;
use crate::router::LSAData;
use crate::console_log;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationEvent {
    pub timestamp: f64,
    pub event_type: SimulationEventType,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimulationEventType {
    RouterAdded { router_id: u32, name: String },
    LinkCreated { from_router: u32, to_router: u32, cost: u32 },
    OSPFEnabled { router_id: u32 },
    PacketSent { from_router: u32, to_router: u32, packet_type: String },
    PacketReceived { router_id: u32, packet_type: String },
    RoutingTableUpdated { router_id: u32 },
    NeighborStateChanged { router_id: u32, neighbor_id: u32, new_state: String },
}

pub struct NetworkSimulation {
    pub topology: NetworkTopology,
    pub protocol_engine: ProtocolEngine,
    pub simulation_log: Vec<SimulationEvent>,
    pub simulation_time: f64,
    pub running: bool,
    ospf_engines: HashMap<u32, OSPFEngine>,
}

impl NetworkSimulation {
    pub fn new() -> Self {
        NetworkSimulation {
            topology: NetworkTopology::new(),
            protocol_engine: ProtocolEngine::new(),
            simulation_log: Vec::new(),
            simulation_time: 0.0,
            running: false,
            ospf_engines: HashMap::new(),
        }
    }

    pub fn add_router(&mut self, name: String, _x: f64, _y: f64) -> u32 {
        let router_id = self.topology.add_router(name.clone());
        
        self.log_event(SimulationEvent {
            timestamp: self.simulation_time,
            event_type: SimulationEventType::RouterAdded { 
                router_id, 
                name: name.clone() 
            },
            description: format!("Router '{}' added with ID {}", name, router_id),
        });
        
        router_id
    }

    pub fn connect_routers(&mut self, router1_id: u32, router2_id: u32, cost: u32) -> Result<(), String> {
        let link_id = self.topology.connect_routers(router1_id, router2_id, cost)?;
        
        // Update OSPF engines with new link information
        if let Some(link) = self.topology.links.get(&link_id) {
            if let Some(engine1) = self.ospf_engines.get_mut(&router1_id) {
                engine1.add_router_link(router2_id, link.router1_interface_id, cost);
                // Regenerate Router LSA with the new link
                console_log!("Router {} regenerating LSA after link addition", router1_id);
                let events = engine1.regenerate_router_lsa();
                // Note: We don't schedule these events immediately as simulation might not be running
                // They will be flooded when simulation starts
            }
            if let Some(engine2) = self.ospf_engines.get_mut(&router2_id) {
                engine2.add_router_link(router1_id, link.router2_interface_id, cost);
                // Regenerate Router LSA with the new link
                console_log!("Router {} regenerating LSA after link addition", router2_id);
                let events = engine2.regenerate_router_lsa();
                // Note: We don't schedule these events immediately as simulation might not be running
                // They will be flooded when simulation starts
            }
        }
        
        self.log_event(SimulationEvent {
            timestamp: self.simulation_time,
            event_type: SimulationEventType::LinkCreated { 
                from_router: router1_id, 
                to_router: router2_id, 
                cost 
            },
            description: format!("Link created between routers {} and {} with cost {}", 
                router1_id, router2_id, cost),
        });
        
        Ok(())
    }

    pub fn delete_router(&mut self, router_id: u32) -> bool {
        // Stop simulation if running
        if self.running {
            self.running = false;
        }
        
        // Remove router from topology
        if self.topology.routers.remove(&router_id).is_some() {
            // Remove all links connected to this router
            let links_to_remove: Vec<u32> = self.topology.links
                .iter()
                .filter(|(_, link)| link.router1_id == router_id || link.router2_id == router_id)
                .map(|(id, _)| *id)
                .collect();
            
            for link_id in links_to_remove {
                self.topology.links.remove(&link_id);
            }
            
            // Remove OSPF engine
            self.ospf_engines.remove(&router_id);
            
            self.log_event(SimulationEvent {
                timestamp: self.simulation_time,
                event_type: SimulationEventType::RouterAdded { 
                    router_id, 
                    name: format!("Deleted Router {}", router_id) 
                },
                description: format!("Router {} deleted", router_id),
            });
            
            true
        } else {
            false
        }
    }
    
    pub fn disconnect_routers(&mut self, router1_id: u32, router2_id: u32) -> bool {
        let link_to_remove = self.topology.links
            .iter()
            .find(|(_, link)| {
                (link.router1_id == router1_id && link.router2_id == router2_id) ||
                (link.router1_id == router2_id && link.router2_id == router1_id)
            })
            .map(|(id, _)| *id);
        
        if let Some(link_id) = link_to_remove {
            self.topology.links.remove(&link_id);
            
            // Notify OSPF engines about the link failure
            if let Some(engine1) = self.ospf_engines.get_mut(&router1_id) {
                if engine1.remove_neighbor(router2_id) {
                    self.log_event(SimulationEvent {
                        timestamp: self.simulation_time,
                        event_type: SimulationEventType::NeighborStateChanged {
                            router_id: router1_id,
                            neighbor_id: router2_id,
                            new_state: "Down".to_string(),
                        },
                        description: format!("Router {} removed neighbor {} due to link failure", router1_id, router2_id),
                    });
                }
            }
            
            if let Some(engine2) = self.ospf_engines.get_mut(&router2_id) {
                if engine2.remove_neighbor(router1_id) {
                    self.log_event(SimulationEvent {
                        timestamp: self.simulation_time,
                        event_type: SimulationEventType::NeighborStateChanged {
                            router_id: router2_id,
                            neighbor_id: router1_id,
                            new_state: "Down".to_string(),
                        },
                        description: format!("Router {} removed neighbor {} due to link failure", router2_id, router1_id),
                    });
                }
            }
            
            // Remove any scheduled packet events between these routers
            self.protocol_engine.events.retain(|event| {
                !((event.from_router_id == router1_id && event.to_router_id == router2_id) ||
                  (event.from_router_id == router2_id && event.to_router_id == router1_id))
            });
            
            // Recalculate routes for affected routers
            self.calculate_routes_for_router(router1_id);
            self.calculate_routes_for_router(router2_id);
            
            self.log_event(SimulationEvent {
                timestamp: self.simulation_time,
                event_type: SimulationEventType::LinkCreated { 
                    from_router: router1_id, 
                    to_router: router2_id, 
                    cost: 0 
                },
                description: format!("Disconnected routers {} and {}", router1_id, router2_id),
            });
            
            true
        } else {
            false
        }
    }
    
    pub fn enable_ospf(&mut self, router_id: u32) -> Result<(), String> {
        self.topology.enable_ospf_on_router(router_id)?;
        
        // Create OSPF engine for this router
        let router_ip = format!("{}.{}.{}.{}", 1, 1, 1, router_id);
        let mut ospf_engine = OSPFEngine::new(router_ip.clone(), "0.0.0.0".to_string());
        
        // Add router links to OSPF engine
        for link in self.topology.links.values() {
            if link.router1_id == router_id {
                ospf_engine.add_router_link(link.router2_id, link.router1_interface_id, link.cost);
            } else if link.router2_id == router_id {
                ospf_engine.add_router_link(link.router1_id, link.router2_interface_id, link.cost);
            }
        }
        
        // Generate initial Router LSA immediately upon OSPF enabling
        ospf_engine.generate_router_lsa();
        console_log!("Router {} initial Router LSA generated", router_id);
        
        self.ospf_engines.insert(router_id, ospf_engine);
        
        self.log_event(SimulationEvent {
            timestamp: self.simulation_time,
            event_type: SimulationEventType::OSPFEnabled { router_id },
            description: format!("OSPF enabled on router {} at simulation time {}", router_id, self.simulation_time),
        });
        
        // Calculate initial routes after OSPF is enabled
        console_log!("OSPF enabled on router {}, calculating initial routes", router_id);
        self.calculate_routes_for_router(router_id);
        
        // If simulation is running, schedule hello packets immediately
        if self.running {
            self.schedule_initial_hello_packets(router_id);
        }
        
        Ok(())
    }

    pub fn start_simulation(&mut self) {
        self.running = true;
        self.simulation_time = 0.0;
        
        let router_ids: Vec<u32> = self.topology.routers
            .iter()
            .filter(|(_, router)| router.ospf_state.is_some())
            .map(|(id, _)| *id)
            .collect();
        
        console_log!("Starting simulation with {} OSPF-enabled routers", router_ids.len());
        
        // Schedule initial hello packets immediately
        for router_id in router_ids {
            console_log!("Scheduling initial hello packets for router {}", router_id);
            self.schedule_initial_hello_packets(router_id);
        }
    }
    
    fn schedule_initial_hello_packets(&mut self, router_id: u32) {
        // Send first hello packet with minimal delay
        let initial_hello_time = self.simulation_time + 0.01;
        
        if let Some(router) = self.topology.routers.get(&router_id) {
            if let Some(_ospf_state) = &router.ospf_state {
                // Schedule the first hello timer event (not an actual packet)
                let timer_event = PacketEvent {
                    timestamp: initial_hello_time,
                    from_router_id: router_id,
                    to_router_id: router_id,  // Self-event for timer
                    packet: ProtocolPacket::OSPF(OSPFPacket {
                        version: 2,
                        packet_type: OSPFPacketType::Hello,
                        router_id: format!("{}.{}.{}.{}", 1, 1, 1, router_id),
                        area_id: "0.0.0.0".to_string(),
                        checksum: 0,
                        auth_type: 0,
                        authentication: 0,
                        data: OSPFPacketData::Hello(HelloPacket {
                            network_mask: "255.255.255.252".to_string(),
                            hello_interval: 10,
                            options: 0,
                            router_priority: 1,
                            router_dead_interval: 40,
                            designated_router: "0.0.0.0".to_string(),
                            backup_designated_router: "0.0.0.0".to_string(),
                            neighbors: Vec::new(),
                        }),
                    }),
                };
                self.protocol_engine.schedule_event(timer_event);
            }
        }
    }

    pub fn stop_simulation(&mut self) {
        self.running = false;
    }

    pub fn step_simulation(&mut self, time_delta: f64) {
        if !self.running {
            return;
        }

        let target_time = self.simulation_time + time_delta;
        
        while let Some(event) = self.protocol_engine.process_next_event() {
            if event.timestamp > target_time {
                self.protocol_engine.events.insert(0, event);
                break;
            }
            
            self.simulation_time = event.timestamp;
            self.process_packet_event(event);
        }
        
        self.simulation_time = target_time;
        
        // Update all OSPF engines' time after processing events
        for engine in self.ospf_engines.values_mut() {
            engine.update_time(self.simulation_time);
        }
    }

    fn schedule_hello_packets(&mut self, router_id: u32) {
        let hello_interval = 10.0;
        let next_hello_time = self.simulation_time + hello_interval;
        
        if let Some(router) = self.topology.routers.get(&router_id) {
            // Check if router is failed
            if router.is_failed {
                console_log!("Router {} is failed, not scheduling hello packets", router_id);
                return;
            }
            
            if let Some(_ospf_state) = &router.ospf_state {
                let neighbors = self.topology.get_neighbors(router_id);
                
                console_log!("Router {} scheduling hello packets at time {:.1}s for {} neighbors",
                    router_id, self.simulation_time, neighbors.len());
                
                for neighbor_id in neighbors {
                    // Check if the link is failed
                    let link_failed = self.topology.links.values().any(|link| {
                        ((link.router1_id == router_id && link.router2_id == neighbor_id) ||
                         (link.router1_id == neighbor_id && link.router2_id == router_id)) &&
                        link.is_failed
                    });
                    
                    if link_failed {
                        console_log!("  Skipping hello to router {} - link is failed", neighbor_id);
                        continue;
                    }
                    
                    // Only send hello packets to neighbors that have OSPF enabled and are not failed
                    if let Some(neighbor_router) = self.topology.routers.get(&neighbor_id) {
                        if neighbor_router.is_failed {
                            console_log!("  Skipping hello to router {} - router is failed", neighbor_id);
                            continue;
                        }
                        
                        if neighbor_router.ospf_state.is_some() {
                            let packet = self.create_hello_packet(router_id);
                            let event = PacketEvent {
                                timestamp: next_hello_time,
                                from_router_id: router_id,
                                to_router_id: neighbor_id,
                                packet: ProtocolPacket::OSPF(packet),
                            };
                            
                            self.protocol_engine.schedule_event(event);
                            console_log!("  Scheduled hello to router {} at time {:.1}s",
                                neighbor_id, next_hello_time);
                        } else {
                            console_log!("  Skipping hello to router {} - OSPF not enabled",
                                neighbor_id);
                        }
                    }
                }
                
                // Schedule next hello timer event
                let timer_event = PacketEvent {
                    timestamp: next_hello_time,
                    from_router_id: router_id,
                    to_router_id: router_id,
                    packet: ProtocolPacket::OSPF(OSPFPacket {
                        version: 2,
                        packet_type: OSPFPacketType::Hello,
                        router_id: format!("{}.{}.{}.{}", 1, 1, 1, router_id),
                        area_id: "0.0.0.0".to_string(),
                        checksum: 0,
                        auth_type: 0,
                        authentication: 0,
                        data: OSPFPacketData::Hello(HelloPacket {
                            network_mask: "255.255.255.252".to_string(),
                            hello_interval: 10,
                            options: 0,
                            router_priority: 1,
                            router_dead_interval: 40,
                            designated_router: "0.0.0.0".to_string(),
                            backup_designated_router: "0.0.0.0".to_string(),
                            neighbors: Vec::new(),
                        }),
                    }),
                };
                self.protocol_engine.schedule_event(timer_event);
            }
        }
    }

    fn create_hello_packet(&self, router_id: u32) -> OSPFPacket {
        use crate::ospf::{OSPFPacketData};
        
        let router = &self.topology.routers[&router_id];
        let ospf_state = router.ospf_state.as_ref().unwrap();
        
        // Use OSPF engine to generate hello packet
        let hello_packet = if let Some(engine) = self.ospf_engines.get(&router_id) {
            engine.generate_hello_packet()
        } else {
            // Fallback if engine not found
            use crate::ospf::HelloPacket;
            HelloPacket {
                network_mask: "255.255.255.252".to_string(),
                hello_interval: 10,
                options: 0,
                router_priority: 1,
                router_dead_interval: 40,
                designated_router: "0.0.0.0".to_string(),
                backup_designated_router: "0.0.0.0".to_string(),
                neighbors: Vec::new(),
            }
        };
        
        OSPFPacket {
            version: 2,
            packet_type: OSPFPacketType::Hello,
            router_id: ospf_state.router_id.clone(),
            area_id: ospf_state.area_id.clone(),
            checksum: 0,
            auth_type: 0,
            authentication: 0,
            data: OSPFPacketData::Hello(hello_packet),
        }
    }

    fn process_packet_event(&mut self, event: PacketEvent) {
        // Check if this is a timer event for hello packet scheduling
        if event.from_router_id == event.to_router_id && 
           self.topology.routers.contains_key(&event.from_router_id) {
            let ProtocolPacket::OSPF(ref ospf_packet) = event.packet;
            if matches!(ospf_packet.packet_type, OSPFPacketType::Hello) {
                self.schedule_hello_packets(event.from_router_id);
                return; // Don't process timer events as regular packets
            }
        }
        
        // Check if source router is failed
        if let Some(router) = self.topology.routers.get(&event.from_router_id) {
            if router.is_failed {
                console_log!("Dropping packet from failed router {}", event.from_router_id);
                return;
            }
        }
        
        // Check if destination router is failed
        if let Some(router) = self.topology.routers.get(&event.to_router_id) {
            if router.is_failed {
                console_log!("Dropping packet to failed router {}", event.to_router_id);
                return;
            }
        }
        
        // Check if the link is failed
        let link_failed = self.topology.links.values().any(|link| {
            ((link.router1_id == event.from_router_id && link.router2_id == event.to_router_id) ||
             (link.router1_id == event.to_router_id && link.router2_id == event.from_router_id)) &&
            link.is_failed
        });
        
        if link_failed {
            console_log!("Dropping packet on failed link between router {} and router {}", 
                event.from_router_id, event.to_router_id);
            return;
        }
        
        match &event.packet {
            ProtocolPacket::OSPF(ospf_packet) => {
                let packet_type = match &ospf_packet.packet_type {
                    OSPFPacketType::Hello => "Hello",
                    OSPFPacketType::DatabaseDescription => "Database Description",
                    OSPFPacketType::LinkStateRequest => "Link State Request",
                    OSPFPacketType::LinkStateUpdate => "Link State Update",
                    OSPFPacketType::LinkStateAcknowledgment => "Link State Acknowledgment",
                };
                
                // Only log and visualize actual packets (not timer events)
                if event.from_router_id != event.to_router_id {
                    self.log_event(SimulationEvent {
                        timestamp: event.timestamp,
                        event_type: SimulationEventType::PacketSent {
                            from_router: event.from_router_id,
                            to_router: event.to_router_id,
                            packet_type: packet_type.to_string(),
                        },
                        description: format!("OSPF {} packet sent from router {} to router {}", 
                            packet_type, event.from_router_id, event.to_router_id),
                    });
                }
                
                // Simulate packet delivery delay
                let delivery_time = event.timestamp + 0.01; // 10ms delay for faster visualization
                
                // Log detailed packet information
                let packet_details = match &ospf_packet.data {
                    OSPFPacketData::Hello(hello) => {
                        format!("Hello packet - Interval: {}s, Dead: {}s, Priority: {}, DR: {}, BDR: {}, Neighbors: [{}]",
                            hello.hello_interval,
                            hello.router_dead_interval,
                            hello.router_priority,
                            hello.designated_router,
                            hello.backup_designated_router,
                            hello.neighbors.join(", ")
                        )
                    },
                    OSPFPacketData::DatabaseDescription(dd) => {
                        format!("Database Description - MTU: {}, Flags: {:#04x}, Seq: {}, LSA headers: {}",
                            dd.interface_mtu,
                            dd.flags,
                            dd.dd_sequence_number,
                            dd.lsa_headers.len()
                        )
                    },
                    OSPFPacketData::LinkStateRequest(lsr) => {
                        format!("Link State Request - Requesting {} LSAs", lsr.requests.len())
                    },
                    OSPFPacketData::LinkStateUpdate(lsu) => {
                        format!("Link State Update - Contains {} LSAs", lsu.lsas.len())
                    },
                    OSPFPacketData::LinkStateAcknowledgment(lsack) => {
                        format!("Link State Acknowledgment - Acknowledging {} LSAs", lsack.lsa_headers.len())
                    },
                };
                
                // Only log reception of actual packets (not timer events)
                if event.from_router_id != event.to_router_id {
                    self.log_event(SimulationEvent {
                        timestamp: delivery_time,
                        event_type: SimulationEventType::PacketReceived {
                            router_id: event.to_router_id,
                            packet_type: packet_type.to_string(),
                        },
                        description: format!("Router {} received OSPF {} from router {} - {}", 
                            event.to_router_id, packet_type, event.from_router_id, packet_details),
                    });
                }
                
                // Process packet in OSPF engine
                self.process_ospf_packet(ospf_packet.clone(), event.from_router_id, event.to_router_id);
            }
        }
    }
    
    fn process_ospf_packet(&mut self, packet: OSPFPacket, from_router_id: u32, to_router_id: u32) {
        let (new_events, lsa_updated, lsa_count, lsa_database_changed, state_transitions) = if let Some(engine) = self.ospf_engines.get_mut(&to_router_id) {
            // Update engine time before processing packet
            engine.update_time(self.simulation_time);
            
            // Get LSA count before processing
            let lsa_count_before = engine.get_lsa_count();
            
            let new_events = match &packet.data {
                OSPFPacketData::Hello(hello) => {
                    // Get interface ID for this connection
                    let interface_id = self.topology.links.values()
                        .find(|link| {
                            (link.router1_id == from_router_id && link.router2_id == to_router_id) ||
                            (link.router1_id == to_router_id && link.router2_id == from_router_id)
                        })
                        .map(|link| {
                            if link.router1_id == to_router_id {
                                link.router1_interface_id
                            } else {
                                link.router2_interface_id
                            }
                        })
                        .unwrap_or(0);
                    
                    engine.process_hello_packet(hello, from_router_id, interface_id)
                }
                OSPFPacketData::DatabaseDescription(dd) => {
                    engine.process_dd_packet(dd, from_router_id)
                }
                OSPFPacketData::LinkStateRequest(lsr) => {
                    engine.process_lsr_packet(lsr, from_router_id)
                }
                OSPFPacketData::LinkStateUpdate(lsu) => {
                    engine.process_lsu_packet(lsu, from_router_id)
                }
                OSPFPacketData::LinkStateAcknowledgment(lsack) => {
                    engine.process_lsack_packet(lsack, from_router_id)
                }
            };
            
            // Check if LSA database was updated
            let lsa_updated = matches!(&packet.data, OSPFPacketData::LinkStateUpdate(_)) 
                || matches!(&packet.data, OSPFPacketData::DatabaseDescription(_));
            let lsa_count = engine.get_lsa_count();
            // Also check if LSA count changed
            let lsa_database_changed = lsa_count != lsa_count_before;
            let state_transitions = engine.get_neighbor_state_transitions();
            
            (new_events, lsa_updated, lsa_count, lsa_database_changed, state_transitions)
        } else {
            return;
        };
        
        // Schedule any response packets
        for mut event in new_events {
            event.timestamp = self.simulation_time + 0.1;
            self.protocol_engine.schedule_event(event);
        }
        
        // If LSAs were updated, recalculate routes
        console_log!("Router {} LSA trigger check: lsa_updated={}, lsa_database_changed={}, lsa_count={}", 
            to_router_id, lsa_updated, lsa_database_changed, lsa_count);
        if (lsa_updated || lsa_database_changed) && lsa_count > 0 {
            console_log!("Router {} LSA database changed, running SPF calculation", to_router_id);
            self.calculate_routes_for_router(to_router_id);
        } else {
            console_log!("Router {} SPF NOT triggered - condition not met", to_router_id);
        }
        
        // Process state transitions
            for (neighbor_id, (prev_state, new_state)) in state_transitions {
                // Skip if state hasn't changed
                if prev_state == new_state {
                    continue;
                }
                
                // Get state names
                let prev_state_name = match prev_state {
                    crate::router::OSPFNeighborState::Down => "Down",
                    crate::router::OSPFNeighborState::Init => "Init",
                    crate::router::OSPFNeighborState::TwoWay => "TwoWay",
                    crate::router::OSPFNeighborState::ExStart => "ExStart",
                    crate::router::OSPFNeighborState::Exchange => "Exchange",
                    crate::router::OSPFNeighborState::Loading => "Loading",
                    crate::router::OSPFNeighborState::Full => "Full",
                };
                
                let new_state_name = match new_state {
                    crate::router::OSPFNeighborState::Down => "Down",
                    crate::router::OSPFNeighborState::Init => "Init",
                    crate::router::OSPFNeighborState::TwoWay => "TwoWay",
                    crate::router::OSPFNeighborState::ExStart => "ExStart",
                    crate::router::OSPFNeighborState::Exchange => "Exchange",
                    crate::router::OSPFNeighborState::Loading => "Loading",
                    crate::router::OSPFNeighborState::Full => "Full",
                };
                
                // Log state transition with from->to format
                self.log_event(SimulationEvent {
                    timestamp: self.simulation_time,
                    event_type: SimulationEventType::NeighborStateChanged {
                        router_id: to_router_id,
                        neighbor_id,
                        new_state: new_state_name.to_string(),
                    },
                    description: format!("Router {} neighbor {} state changed: {} → {}", 
                        to_router_id, neighbor_id, prev_state_name, new_state_name),
                });
                
                // Recalculate routes when adjacency is established or lost
                match new_state {
                    crate::router::OSPFNeighborState::Full => {
                        // When Full adjacency is established, recalculate routes for both routers
                        self.calculate_routes_for_router(to_router_id);
                        self.calculate_routes_for_router(from_router_id);
                        
                        // Also trigger route calculation for all other OSPF-enabled routers
                        // since they may have received new LSAs
                        let ospf_routers: Vec<u32> = self.ospf_engines.keys().cloned().collect();
                        for router_id in ospf_routers {
                            if router_id != to_router_id && router_id != from_router_id {
                                self.calculate_routes_for_router(router_id);
                            }
                        }
                    }
                    crate::router::OSPFNeighborState::Down => {
                        self.calculate_routes_for_router(to_router_id);
                    }
                    _ => {}
                }
            }
    }
    
    fn calculate_routes_for_router(&mut self, router_id: u32) {
        // Debug: Log when route calculation is triggered
        console_log!("=== CALCULATING ROUTES FOR ROUTER {} ===", router_id);
        
        self.log_event(SimulationEvent {
            timestamp: self.simulation_time,
            event_type: SimulationEventType::RoutingTableUpdated { router_id },
            description: format!("Router {} starting route calculation", router_id),
        });
        
        // Get LSA database from OSPF engine
        let (routes, lsa_count) = if let Some(engine) = self.ospf_engines.get(&router_id) {
            let lsa_count = engine.get_lsa_count();
            console_log!("Router {} has {} LSAs in database", router_id, lsa_count);
            console_log!("Router {} DEBUG TEST ACTIVE", router_id);
            
            // Enhanced debugging for ALL routers
            console_log!("=== ROUTER {} SPF DEBUG START ===", router_id);
            console_log!("Router {} LSA count: {}", router_id, lsa_count);
            console_log!("Router {} LSA database keys:", router_id);
            for (key, lsa) in engine.get_lsa_database() {
                console_log!("  Key: {} - Type: {:?} - Adv Router: {}", 
                    key, lsa.header.ls_type, lsa.header.advertising_router);
                if let LSAData::Router(ref rlsa) = lsa.data {
                    console_log!("    Router LSA with {} links:", rlsa.links.len());
                    for (i, link) in rlsa.links.iter().enumerate() {
                        console_log!("      Link {}: ID={}, Type={:?}, Metric={}", 
                            i, link.link_id, link.link_type, link.metric);
                    }
                }
            }
            console_log!("=== ROUTER {} SPF DEBUG END ===", router_id);
            
            let routes = SPFCalculator::calculate_routes_from_lsa(
                engine.get_lsa_database(),
                router_id,
                &self.topology
            );
            
            console_log!("Router {} SPF returned {} routes", router_id, routes.len());
            if routes.is_empty() {
                console_log!("  WARNING: No routes calculated for router {}", router_id);
                // Check router's interfaces
                if let Some(router) = self.topology.routers.get(&router_id) {
                    console_log!("  Router {} has {} interfaces:", router_id, router.interfaces.len());
                    for (id, interface) in &router.interfaces {
                        console_log!("    Interface {}: IP {}", id, interface.ip_address);
                    }
                }
            } else {
                for (dest_id, route) in &routes {
                    console_log!("  Route to {}: {} -> {} (metric {}) interface {}", 
                        dest_id, route.destination, route.next_hop, route.metric, route.interface_id);
                }
            }
            
            (routes, Some(lsa_count))
        } else {
            console_log!("Router {} has no OSPF engine, using topology-based routing", router_id);
            // Fallback to topology-based calculation if no OSPF engine
            (SPFCalculator::calculate_routes(&self.topology, router_id), None)
        };
        
        // Log LSA count if OSPF is enabled
        if let Some(count) = lsa_count {
            self.log_event(SimulationEvent {
                timestamp: self.simulation_time,
                event_type: SimulationEventType::RoutingTableUpdated { router_id },
                description: format!("Router {} has {} LSAs in database", router_id, count),
            });
        }
        
        if let Some(router) = self.topology.routers.get_mut(&router_id) {
            // Store old routing table for comparison
            let old_routes = router.routing_table.clone();
            console_log!("Router {} old routing table has {} entries", router_id, old_routes.len());
            
            // Update routing table
            console_log!("Router {} updating routing table with {} new routes", router_id, routes.len());
            for (_dest_id, route) in &routes {
                console_log!("  Updating route: {} -> {} via {}", 
                    route.destination, route.next_hop, route.interface_id);
                router.update_routing_table(route.clone());
            }
            console_log!("Router {} new routing table has {} entries", router_id, router.routing_table.len());
            
            // Build detailed description of routing table changes
            let mut route_details = Vec::new();
            
            // Check for new or updated routes
            for (_dest_id, new_route) in &routes {
                let is_new = !old_routes.iter().any(|r| 
                    r.destination == new_route.destination && r.netmask == new_route.netmask
                );
                
                if is_new {
                    route_details.push(format!("  + Added: {}/{} via {} metric {}", 
                        new_route.destination, new_route.netmask, 
                        new_route.next_hop, new_route.metric));
                } else {
                    // Check if route changed
                    if let Some(old_route) = old_routes.iter().find(|r| 
                        r.destination == new_route.destination && r.netmask == new_route.netmask
                    ) {
                        if old_route.next_hop != new_route.next_hop || old_route.metric != new_route.metric {
                            route_details.push(format!("  ≈ Updated: {}/{} via {} metric {} (was: via {} metric {})", 
                                new_route.destination, new_route.netmask, 
                                new_route.next_hop, new_route.metric,
                                old_route.next_hop, old_route.metric));
                        }
                    }
                }
            }
            
            // Check for removed routes
            for old_route in &old_routes {
                let still_exists = router.routing_table.iter().any(|r| 
                    r.destination == old_route.destination && r.netmask == old_route.netmask
                );
                
                if !still_exists {
                    route_details.push(format!("  - Removed: {}/{} via {} metric {}", 
                        old_route.destination, old_route.netmask, 
                        old_route.next_hop, old_route.metric));
                }
            }
            
            // Log routing table update with details
            let description = if route_details.is_empty() {
                format!("Router {} routing table recalculated (no changes)", router_id)
            } else {
                format!("Router {} routing table updated:\n{}", 
                    router_id, route_details.join("\n"))
            };
            
            self.log_event(SimulationEvent {
                timestamp: self.simulation_time,
                event_type: SimulationEventType::RoutingTableUpdated { router_id },
                description,
            });
        }
    }

    fn log_event(&mut self, event: SimulationEvent) {
        self.simulation_log.push(event);
    }

    pub fn get_recent_events(&self, count: usize) -> Vec<SimulationEvent> {
        let start = self.simulation_log.len().saturating_sub(count);
        self.simulation_log[start..].to_vec()
    }
    
    pub fn get_ospf_neighbor_count(&self, router_id: u32) -> usize {
        self.ospf_engines.get(&router_id)
            .map(|engine| engine.get_neighbor_count())
            .unwrap_or(0)
    }
    
    pub fn get_ospf_lsa_count(&self, router_id: u32) -> usize {
        self.ospf_engines.get(&router_id)
            .map(|engine| engine.get_lsa_count())
            .unwrap_or(0)
    }
    
    pub fn toggle_link_failure(&mut self, from_id: u32, to_id: u32) -> bool {
        // Find the link
        let link_id = self.topology.links
            .iter()
            .find(|(_, link)| {
                (link.router1_id == from_id && link.router2_id == to_id) ||
                (link.router1_id == to_id && link.router2_id == from_id)
            })
            .map(|(id, _)| *id);
        
        if let Some(link_id) = link_id {
            let (link_failed, link_cost) = if let Some(link) = self.topology.links.get_mut(&link_id) {
                link.is_failed = !link.is_failed;
                (link.is_failed, link.cost)
            } else {
                return false;
            };
            
            // Log the event
            let event_type = if link_failed {
                "Link Failure"
            } else {
                "Link Recovery"
            };
            
            self.log_event(SimulationEvent {
                timestamp: self.simulation_time,
                event_type: SimulationEventType::LinkCreated {
                    from_router: from_id,
                    to_router: to_id,
                    cost: link_cost,
                },
                description: format!("{}: Link between Router {} and Router {}", 
                    event_type, from_id, to_id),
            });
            
            // If failed, trigger neighbor down events and remove links
            if link_failed {
                // Get link information before removing neighbors
                let (link_info1, link_info2) = {
                    let link = self.topology.links.get(&link_id).unwrap();
                    (
                        (link.router1_id, link.router2_id, link.router1_interface_id, link.cost),
                        (link.router2_id, link.router1_id, link.router2_interface_id, link.cost)
                    )
                };
                
                // Notify OSPF engines about link failure
                let mut events_to_log = Vec::new();
                
                if let Some(engine1) = self.ospf_engines.get_mut(&link_info1.0) {
                    if engine1.remove_neighbor(link_info1.1) {
                        events_to_log.push(SimulationEvent {
                            timestamp: self.simulation_time,
                            event_type: SimulationEventType::NeighborStateChanged {
                                router_id: link_info1.0,
                                neighbor_id: link_info1.1,
                                new_state: "Down".to_string(),
                            },
                            description: format!("Router {} neighbor {} went down due to link failure", 
                                link_info1.0, link_info1.1),
                        });
                    }
                    // Remove the link from router_links
                    engine1.remove_link(link_info1.1);
                }
                
                if let Some(engine2) = self.ospf_engines.get_mut(&link_info2.0) {
                    if engine2.remove_neighbor(link_info2.1) {
                        events_to_log.push(SimulationEvent {
                            timestamp: self.simulation_time,
                            event_type: SimulationEventType::NeighborStateChanged {
                                router_id: link_info2.0,
                                neighbor_id: link_info2.1,
                                new_state: "Down".to_string(),
                            },
                            description: format!("Router {} neighbor {} went down due to link failure", 
                                link_info2.0, link_info2.1),
                        });
                    }
                    // Remove the link from router_links
                    engine2.remove_link(link_info2.1);
                }
                
                // Log events after releasing mutable borrows
                for event in events_to_log {
                    self.log_event(event);
                }
                
                // Regenerate Router LSAs for affected routers
                let mut new_events = Vec::new();
                if let Some(engine1) = self.ospf_engines.get_mut(&from_id) {
                    let events = engine1.regenerate_router_lsa();
                    for mut event in events {
                        event.timestamp = self.simulation_time + 0.1; // Schedule LSA flood
                        new_events.push(event);
                    }
                }
                
                if let Some(engine2) = self.ospf_engines.get_mut(&to_id) {
                    let events = engine2.regenerate_router_lsa();
                    for mut event in events {
                        event.timestamp = self.simulation_time + 0.1; // Schedule LSA flood
                        new_events.push(event);
                    }
                }
                
                // Schedule the new events
                for event in new_events {
                    self.protocol_engine.schedule_event(event);
                }
                
                // Recalculate routes
                self.calculate_routes_for_router(from_id);
                self.calculate_routes_for_router(to_id);
            } else {
                // Link recovery - add links back
                let link_info = {
                    let link = self.topology.links.get(&link_id).unwrap();
                    (
                        (link.router1_id, link.router2_id, link.router1_interface_id, link.cost),
                        (link.router2_id, link.router1_id, link.router2_interface_id, link.cost)
                    )
                };
                
                // Add links back to OSPF engines
                if let Some(engine1) = self.ospf_engines.get_mut(&link_info.0.0) {
                    engine1.add_link(link_info.0.1, link_info.0.2, link_info.0.3);
                }
                
                if let Some(engine2) = self.ospf_engines.get_mut(&link_info.1.0) {
                    engine2.add_link(link_info.1.1, link_info.1.2, link_info.1.3);
                }
                
                // Note: Neighbor relationships will be re-established through Hello protocol
            }
            
            return true;
        }
        false
    }
    
    pub fn toggle_router_failure(&mut self, router_id: u32) -> bool {
        let (router_failed, router_name, has_ospf) = if let Some(router) = self.topology.routers.get_mut(&router_id) {
            router.is_failed = !router.is_failed;
            (router.is_failed, router.name.clone(), router.ospf_state.is_some())
        } else {
            return false;
        };
        
        // Log the event
        let event_type = if router_failed {
            "Router Failure"
        } else {
            "Router Recovery"
        };
        
        self.log_event(SimulationEvent {
            timestamp: self.simulation_time,
            event_type: SimulationEventType::RouterAdded {
                router_id,
                name: router_name.clone(),
            },
            description: format!("{}: Router {} ({})", event_type, router_id, router_name),
        });
        
        if router_failed {
            // Clear routing table
            if let Some(router) = self.topology.routers.get_mut(&router_id) {
                router.routing_table.clear();
            }
            
            // Remove OSPF engine (it will be recreated on recovery)
            self.ospf_engines.remove(&router_id);
                
                // Notify all neighbors about this router going down
                let neighbors: Vec<u32> = self.topology.links
                    .values()
                    .filter_map(|link| {
                        if link.router1_id == router_id {
                            Some(link.router2_id)
                        } else if link.router2_id == router_id {
                            Some(link.router1_id)
                        } else {
                            None
                        }
                    })
                    .collect();
                
                for neighbor_id in neighbors {
                    if let Some(engine) = self.ospf_engines.get_mut(&neighbor_id) {
                        if engine.remove_neighbor(router_id) {
                            self.log_event(SimulationEvent {
                                timestamp: self.simulation_time,
                                event_type: SimulationEventType::NeighborStateChanged {
                                    router_id: neighbor_id,
                                    neighbor_id: router_id,
                                    new_state: "Down".to_string(),
                                },
                                description: format!("Router {} neighbor {} went down due to router failure", 
                                    neighbor_id, router_id),
                            });
                        }
                    }
                    self.calculate_routes_for_router(neighbor_id);
                }
        } else {
            // Router recovery - recreate OSPF engine if enabled
            if has_ospf {
                let router_ip = format!("{}.{}.{}.{}", 1, 1, 1, router_id);
                let mut ospf_engine = OSPFEngine::new(router_ip.clone(), "0.0.0.0".to_string());
                
                // Add router links to OSPF engine
                for link in self.topology.links.values() {
                    if !link.is_failed {
                        if link.router1_id == router_id {
                            ospf_engine.add_router_link(link.router2_id, link.router1_interface_id, link.cost);
                        } else if link.router2_id == router_id {
                            ospf_engine.add_router_link(link.router1_id, link.router2_interface_id, link.cost);
                        }
                    }
                }
                
                self.ospf_engines.insert(router_id, ospf_engine);
                
                // If simulation is running, start sending hello packets
                if self.running {
                    self.schedule_initial_hello_packets(router_id);
                }
            }
        }
        
        return true;
    }
}