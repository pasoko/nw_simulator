/**
 * Display Updater Module
 * Handles updating UI elements with simulation data
 */

import stateManager from './state-manager.js';
import eventLogger from './event-logger.js';
import canvasRenderer from './canvas-renderer.js';
import animationEffects from './animation-effects.js';

class DisplayUpdater {
    constructor() {
        this.processedEvents = new Set();
    }

    updateSimulationDisplay() {
        if (!stateManager.simulator) return;

        // Update routers and connections from simulator
        this.updateRoutersFromSimulator();
        this.updateConnectionsFromSimulator();
        
        // Update hosts from simulator
        this.updateHostsFromSimulator();
        
        // Update router summaries for display
        this.updateRouterSummaries();
        
        // Process and display new events
        this.processNewEvents();
    }

    updateRoutersFromSimulator() {
        const routersJson = stateManager.simulator.get_routers_json();
        
        if (routersJson) {
            const newRouters = JSON.parse(routersJson);
            
            // Only update if there's a change in routers
            if (JSON.stringify(stateManager.routers) !== JSON.stringify(newRouters)) {
                console.log('Updating routers:', newRouters);
                stateManager.routers = newRouters;
            }
        }
    }

    updateConnectionsFromSimulator() {
        const connectionsJson = stateManager.simulator.get_connections_json();
        
        if (connectionsJson) {
            const newConnections = JSON.parse(connectionsJson);
            
            // Only update if there's a change in connections
            if (JSON.stringify(stateManager.connections) !== JSON.stringify(newConnections)) {
                console.log('Updating connections:', newConnections);
                stateManager.connections = newConnections;
            }
        }
    }

    updateRouterSummaries() {
        stateManager.routers.forEach(router => {
            const summaryJson = stateManager.simulator.get_router_summary_json(router.id);
            if (summaryJson) {
                try {
                    router.summary = JSON.parse(summaryJson);
                } catch (error) {
                    console.error(`Error parsing router summary for router ${router.id}:`, error);
                }
            }
        });
    }

    processNewEvents() {
        const eventsJson = stateManager.simulator.get_recent_events_json(50);
        if (!eventsJson) return;

        try {
            const events = JSON.parse(eventsJson);
            
            // Initialize event tracking if not exists
            if (!this.processedEvents) {
                this.processedEvents = new Set();
            }
            
            // Process only new events
            events.forEach(event => {
                this.processEvent(event);
            });
            
            // Keep set size manageable by removing old events
            this.cleanupProcessedEvents();
            
        } catch (error) {
            console.error('Error processing events:', error);
        }
    }

    processEvent(event) {
        // Create unique event key based on timestamp and description
        const eventKey = `${event.timestamp.toFixed(4)}_${event.description}`;
        
        if (!this.processedEvents.has(eventKey) && event.description) {
            this.processedEvents.add(eventKey);
            
            // Log the event
            eventLogger.log(`[${event.timestamp.toFixed(2)}s] ${event.description}`);
            
            // Add packet visualization for packet events
            this.handlePacketVisualization(event);
            
            // Handle OSPF state change animations
            this.handleOSPFStateAnimation(event);
        }
    }

    handlePacketVisualization(event) {
        if (!event.event_type || !event.event_type.PacketSent) return;
        if (!stateManager.packetVisualizer) return;

        const fromRouter = stateManager.routers.find(r => r.id === event.event_type.PacketSent.from_router);
        const toRouter = stateManager.routers.find(r => r.id === event.event_type.PacketSent.to_router);
        
        if (fromRouter && toRouter) {
            stateManager.packetVisualizer.addPacket(
                fromRouter, 
                toRouter, 
                event.event_type.PacketSent.packet_type,
                event.timestamp
            );
            
            // Add packet arrival effect - temporarily disabled to avoid errors
            // TODO: Fix packet color configuration access
            /*
            if (stateManager.canvasRenderer && stateManager.canvasRenderer.ctx && stateManager.packetVisualizer) {
                const packetType = event.event_type.PacketSent.packet_type;
                let packetColor = '#666666'; // Default color
                
                // Use hardcoded colors for now
                const colorMap = {
                    'Hello': '#4CAF50',
                    'Database Description': '#2196F3',
                    'Link State Request': '#FF9800',
                    'Link State Update': '#9C27B0',
                    'Link State Acknowledgment': '#00BCD4'
                };
                
                packetColor = colorMap[packetType] || '#666666';
                
                animationEffects.animatePacketBurst(
                    stateManager.canvasRenderer.ctx,
                    toRouter.x,
                    toRouter.y,
                    packetColor
                );
            }
            */
        }
    }
    
    handleOSPFStateAnimation(event) {
        if (!event.event_type || !event.event_type.StateChange) return;
        if (!stateManager.canvasRenderer || !stateManager.canvasRenderer.ctx) return;
        
        const stateChange = event.event_type.StateChange;
        const router = stateManager.routers.find(r => r.id === stateChange.router_id);
        
        if (router && stateChange.new_state) {
            animationEffects.animateOSPFStateChange(
                stateManager.canvasRenderer.ctx,
                router,
                stateChange.new_state
            );
        }
    }

    cleanupProcessedEvents() {
        if (this.processedEvents.size > 1000) {
            const sortedEvents = Array.from(this.processedEvents).sort();
            const toRemove = sortedEvents.slice(0, sortedEvents.length - 500);
            toRemove.forEach(key => this.processedEvents.delete(key));
        }
    }

    updateSimulationTime(time) {
        stateManager.simulationTime = time;
        
        // Update packet positions
        if (stateManager.packetVisualizer) {
            stateManager.packetVisualizer.update(time);
        }
        
        // Trigger re-render
        canvasRenderer.render();
    }

    resetEventTracking() {
        this.processedEvents.clear();
        console.log('Event tracking reset');
    }

    // Method to sync initial state when simulation starts
    syncInitialState() {
        if (!stateManager.simulator) return;

        console.log('Syncing initial state...');
        
        // Reset event tracking
        stateManager.lastEventTime = -1;
        this.resetEventTracking();
        
        // Sync routers and connections with simulator
        this.updateRoutersFromSimulator();
        this.updateConnectionsFromSimulator();
        
        console.log('Initial routers:', stateManager.routers);
        console.log('Initial connections:', stateManager.connections);
        
        // Clear packet visualizer
        if (stateManager.packetVisualizer) {
            stateManager.packetVisualizer.clear();
        }
    }

    // Method to get current simulation statistics
    getSimulationStats() {
        if (!stateManager.simulator) return null;

        try {
            const statsJson = stateManager.simulator.get_simulation_stats_json();
            return statsJson ? JSON.parse(statsJson) : null;
        } catch (error) {
            console.error('Error getting simulation stats:', error);
            return null;
        }
    }

    // Method to update a specific router's details in the UI
    updateRouterInUI(routerId) {
        if (!stateManager.simulator) return;

        try {
            const summaryJson = stateManager.simulator.get_router_summary_json(routerId);
            const detailsJson = stateManager.simulator.get_router_details_json(routerId);
            
            if (summaryJson && detailsJson) {
                const summary = JSON.parse(summaryJson);
                const details = JSON.parse(detailsJson);
                
                // Update the router object
                const router = stateManager.routers.find(r => r.id === routerId);
                if (router) {
                    router.summary = summary;
                    router.details = details;
                }
                
                // Update the UI element if it exists
                const detailsContainer = document.getElementById(`router-details-${routerId}`);
                if (detailsContainer) {
                    this.updateRouterDetailsElement(detailsContainer, summary, details);
                }
            }
        } catch (error) {
            console.error(`Error updating router ${routerId} in UI:`, error);
        }
    }

    updateRouterDetailsElement(container, summary, details) {
        container.innerHTML = `
            <div class="detail-row">
                <span class="detail-label">OSPF Neighbors:</span>
                <span class="detail-value">${summary.neighbor_count}</span>
            </div>
            <div class="detail-row">
                <span class="detail-label">Routing Table Entries:</span>
                <span class="detail-value">${summary.route_count}</span>
            </div>
            <div class="detail-row">
                <span class="detail-label">LSA Database Size:</span>
                <span class="detail-value">${details.lsa_database_size || 0}</span>
            </div>
            <div class="detail-row">
                <span class="detail-label">Latest Event:</span>
                <span class="detail-value">${summary.latest_event}</span>
            </div>
        `;
    }

    updateHostsFromSimulator() {
        // ホストの更新をhostManagerに委譲
        window.dispatchEvent(new Event('hostsUpdated'));
    }
}

// Create singleton instance
const displayUpdater = new DisplayUpdater();

export default displayUpdater;