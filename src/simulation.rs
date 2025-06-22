use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use crate::network::NetworkTopology;
use crate::protocol::{ProtocolEngine, PacketEvent, ProtocolPacket};
use crate::ospf::{OSPFPacket, OSPFPacketType, OSPFPacketData, HelloPacket};
use crate::ospf_engine::OSPFEngine;
use crate::spf::SPFCalculator;

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
    pub time_scale: f64,
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
            time_scale: 1.0,
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
        self.topology.connect_routers(router1_id, router2_id, cost)?;
        
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
        let ospf_engine = OSPFEngine::new(router_ip.clone(), "0.0.0.0".to_string());
        self.ospf_engines.insert(router_id, ospf_engine);
        
        self.log_event(SimulationEvent {
            timestamp: self.simulation_time,
            event_type: SimulationEventType::OSPFEnabled { router_id },
            description: format!("OSPF enabled on router {} at simulation time {}", router_id, self.simulation_time),
        });
        
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
        
        // Schedule initial hello packets immediately
        for router_id in router_ids {
            self.schedule_initial_hello_packets(router_id);
        }
    }
    
    fn schedule_initial_hello_packets(&mut self, router_id: u32) {
        // Send first hello packet immediately (0.1s delay)
        // Make sure the time is in the future by checking against protocol engine's current time
        let min_time = self.simulation_time.max(self.protocol_engine.current_time);
        let initial_hello_time = min_time + 0.1;
        
        if let Some(router) = self.topology.routers.get(&router_id) {
            if let Some(_ospf_state) = &router.ospf_state {
                let neighbors = self.topology.get_neighbors(router_id);
                
                
                for neighbor_id in neighbors {
                    // Send hello packets to all neighbors regardless of their OSPF state
                    // Real OSPF doesn't wait for neighbors to enable OSPF
                    let packet = self.create_hello_packet(router_id);
                    let event = PacketEvent {
                        timestamp: initial_hello_time,
                        from_router_id: router_id,
                        to_router_id: neighbor_id,
                        packet: ProtocolPacket::OSPF(packet),
                    };
                    
                    self.protocol_engine.schedule_event(event);
                }
                
                // Schedule regular hello packets to start after initial
                let self_event = PacketEvent {
                    timestamp: initial_hello_time,
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
                self.protocol_engine.schedule_event(self_event);
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
    }

    fn schedule_hello_packets(&mut self, router_id: u32) {
        let hello_interval = 10.0;
        let next_hello_time = self.simulation_time + hello_interval;
        
        if let Some(router) = self.topology.routers.get(&router_id) {
            if let Some(_ospf_state) = &router.ospf_state {
                let neighbors = self.topology.get_neighbors(router_id);
                
                for neighbor_id in neighbors {
                    // Send hello packets to all neighbors regardless of their OSPF state
                    let packet = self.create_hello_packet(router_id);
                    let event = PacketEvent {
                        timestamp: next_hello_time,
                        from_router_id: router_id,
                        to_router_id: neighbor_id,
                        packet: ProtocolPacket::OSPF(packet),
                    };
                    
                    self.protocol_engine.schedule_event(event);
                }
                
                // Schedule a self-event to trigger next hello packet scheduling
                let self_event = PacketEvent {
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
                self.protocol_engine.schedule_event(self_event);
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
        // Check if this is a self-event to trigger hello packet scheduling
        if event.from_router_id == event.to_router_id && 
           self.topology.routers.contains_key(&event.from_router_id) {
            self.schedule_hello_packets(event.from_router_id);
            return; // Don't process self-events as regular packets
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
                
                // Simulate packet delivery delay
                let delivery_time = event.timestamp + 0.05; // 50ms delay
                
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
                
                self.log_event(SimulationEvent {
                    timestamp: delivery_time,
                    event_type: SimulationEventType::PacketReceived {
                        router_id: event.to_router_id,
                        packet_type: packet_type.to_string(),
                    },
                    description: format!("Router {} received OSPF {} from router {} - {}", 
                        event.to_router_id, packet_type, event.from_router_id, packet_details),
                });
                
                // Process packet in OSPF engine
                self.process_ospf_packet(ospf_packet.clone(), event.from_router_id, event.to_router_id);
            }
        }
    }
    
    fn process_ospf_packet(&mut self, packet: OSPFPacket, from_router_id: u32, to_router_id: u32) {
        if let Some(engine) = self.ospf_engines.get_mut(&to_router_id) {
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
                _ => Vec::new(),
            };
            
            // Schedule any response packets
            for mut event in new_events {
                event.timestamp = self.simulation_time + 0.1;
                self.protocol_engine.schedule_event(event);
            }
            
            // Check for neighbor state changes
            let neighbor_states = engine.get_neighbor_states();
            for (neighbor_id, state) in neighbor_states {
                self.log_event(SimulationEvent {
                    timestamp: self.simulation_time,
                    event_type: SimulationEventType::NeighborStateChanged {
                        router_id: to_router_id,
                        neighbor_id,
                        new_state: format!("{:?}", state),
                    },
                    description: format!("Router {} neighbor {} state changed to {:?}", 
                        to_router_id, neighbor_id, state),
                });
                
                // If neighbor reached Full state, recalculate routes
                if matches!(state, crate::router::OSPFNeighborState::Full) {
                    self.calculate_routes_for_router(to_router_id);
                }
            }
        }
    }
    
    fn calculate_routes_for_router(&mut self, router_id: u32) {
        let routes = SPFCalculator::calculate_routes(&self.topology, router_id);
        
        if let Some(router) = self.topology.routers.get_mut(&router_id) {
            for (_dest_id, route) in routes {
                router.update_routing_table(route);
            }
            
            self.log_event(SimulationEvent {
                timestamp: self.simulation_time,
                event_type: SimulationEventType::RoutingTableUpdated { router_id },
                description: format!("Router {} routing table updated", router_id),
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
}