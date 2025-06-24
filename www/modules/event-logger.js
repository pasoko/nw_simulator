/**
 * Event Logger Module
 * Manages simulation event logging and export functionality
 */

import stateManager from './state-manager.js';

class EventLogger {
    constructor() {
        this.maxLogEntries = 1000;
    }
    
    log(message) {
        const logContent = document.getElementById('log-content');
        const entry = document.createElement('div');
        entry.className = 'log-entry';
        const timestamp = new Date().toLocaleTimeString();
        const fullMessage = `[${timestamp}] ${message}`;
        entry.textContent = fullMessage;
        logContent.appendChild(entry);
        logContent.scrollTop = logContent.scrollHeight;
        
        // Store log entry for export
        stateManager.addLogEntry({
            timestamp: new Date().toISOString(),
            simulationTime: stateManager.simulationTime,
            message: message
        });
        
        // Limit log entries to prevent memory issues
        if (stateManager.logEntries.length > this.maxLogEntries) {
            stateManager.logEntries = stateManager.logEntries.slice(-this.maxLogEntries);
        }
    }
    
    clearLog() {
        const logContent = document.getElementById('log-content');
        logContent.innerHTML = '';
        stateManager.clearLog();
        this.log('Log cleared');
    }
    
    exportLog() {
        if (stateManager.logEntries.length === 0) {
            alert('No log entries to export');
            return;
        }
        
        // Create log data with metadata
        const logData = {
            simulationName: 'OSPF Network Simulation',
            exportTime: new Date().toISOString(),
            totalEntries: stateManager.logEntries.length,
            entries: stateManager.logEntries
        };
        
        // Convert to JSON and create blob
        const jsonStr = JSON.stringify(logData, null, 2);
        const blob = new Blob([jsonStr], { type: 'application/json' });
        
        // Create download link
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `ospf_simulation_log_${new Date().getTime()}.json`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        
        this.log('Log exported successfully');
    }
    
    processSimulationEvents(eventsJson) {
        if (!eventsJson) return [];
        
        const events = JSON.parse(eventsJson);
        const packetEvents = [];
        
        // Process only new events
        events.forEach(event => {
            // Create unique event key based on timestamp and description
            const eventKey = `${event.timestamp.toFixed(4)}_${event.description}`;
            
            if (!stateManager.hasProcessedEvent(eventKey) && event.description) {
                stateManager.markEventProcessed(eventKey);
                this.log(`[${event.timestamp.toFixed(2)}s] ${event.description}`);
                
                // Collect packet visualization data
                if (event.event_type && event.event_type.PacketSent) {
                    const fromRouter = stateManager.findRouterById(event.event_type.PacketSent.from_router);
                    const toRouter = stateManager.findRouterById(event.event_type.PacketSent.to_router);
                    if (fromRouter && toRouter) {
                        packetEvents.push({
                            type: 'packet',
                            from: fromRouter,
                            to: toRouter,
                            packetType: event.event_type.PacketSent.packet_type,
                            timestamp: event.timestamp
                        });
                    }
                }
            }
        });
        
        return packetEvents;
    }
}

// Export as singleton
export default new EventLogger();