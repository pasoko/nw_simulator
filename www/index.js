import init, { NetworkSimulator } from './pkg/nw_simulator.js';
import { PacketVisualizer } from './packet-visualizer.js';

let simulator;
let canvas;
let ctx;
let mode = 'add-router';
let selectedRouters = [];
let routers = [];
let connections = [];
let simulationRunning = false;
let simulationInterval = null;
let packetVisualizer;
let simulationTime = 0;
let logEntries = [];

async function run() {
    await init();
    
    simulator = new NetworkSimulator();
    canvas = document.getElementById('network-canvas');
    ctx = canvas.getContext('2d');
    
    setupCanvas();
    setupEventListeners();
    
    packetVisualizer = new PacketVisualizer(canvas, ctx);
    
    render();
    
    log('Network Simulator initialized');
}

function setupCanvas() {
    const container = document.getElementById('canvas-container');
    canvas.width = container.clientWidth;
    canvas.height = container.clientHeight;
    
    window.addEventListener('resize', () => {
        canvas.width = container.clientWidth;
        canvas.height = container.clientHeight;
        render();
    });
}

function setupEventListeners() {
    canvas.addEventListener('click', handleCanvasClick);
    
    document.getElementById('add-router-btn').addEventListener('click', () => {
        setMode('add-router');
    });
    
    document.getElementById('connect-routers-btn').addEventListener('click', () => {
        setMode('connect-routers');
    });
    
    document.getElementById('simulate-btn').addEventListener('click', startSimulation);
    document.getElementById('export-log-btn').addEventListener('click', exportLog);
    document.getElementById('clear-log-btn').addEventListener('click', clearLog);
    document.getElementById('router-select').addEventListener('change', handleRouterSelect);
}

function setMode(newMode) {
    mode = newMode;
    selectedRouters = [];
    const indicator = document.getElementById('mode-indicator');
    
    switch(mode) {
        case 'add-router':
            indicator.textContent = 'Mode: Add Router';
            canvas.style.cursor = 'crosshair';
            break;
        case 'connect-routers':
            indicator.textContent = 'Mode: Connect Routers (Select 2)';
            canvas.style.cursor = 'pointer';
            break;
    }
    
    render();
}

function handleCanvasClick(event) {
    const rect = canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    
    if (mode === 'add-router') {
        const name = prompt('Enter router name:');
        if (name) {
            const id = simulator.add_router(name, x, y);
            routers.push({ id, name, x, y, ospf_enabled: false });
            updateRoutersList();
            render();
        }
    } else if (mode === 'connect-routers') {
        const clickedRouter = findRouterAt(x, y);
        if (clickedRouter) {
            if (selectedRouters.includes(clickedRouter.id)) {
                selectedRouters = selectedRouters.filter(id => id !== clickedRouter.id);
            } else {
                selectedRouters.push(clickedRouter.id);
                if (selectedRouters.length === 2) {
                    const cost = parseInt(prompt('Enter link cost:', '1') || '1');
                    simulator.connect_routers(selectedRouters[0], selectedRouters[1], cost);
                    connections.push({
                        from: selectedRouters[0],
                        to: selectedRouters[1],
                        cost
                    });
                    selectedRouters = [];
                    render();
                }
            }
        }
    }
}

function findRouterAt(x, y) {
    const radius = 20;
    return routers.find(router => {
        const dx = router.x - x;
        const dy = router.y - y;
        return dx * dx + dy * dy < radius * radius;
    });
}

function updateRoutersList() {
    const list = document.getElementById('routers-list');
    list.innerHTML = '';
    
    // Update router select dropdown
    const select = document.getElementById('router-select');
    const currentValue = select.value;
    select.innerHTML = '<option value="">Select a router...</option>';
    
    routers.forEach(router => {
        const item = document.createElement('div');
        item.className = 'router-item' + (router.ospf_enabled ? ' ospf-enabled' : '');
        item.innerHTML = `
            <strong>${router.name}</strong> (ID: ${router.id})
            <button class="button" onclick="toggleOSPF(${router.id})">
                ${router.ospf_enabled ? 'Disable' : 'Enable'} OSPF
            </button>
        `;
        list.appendChild(item);
        
        // Add to select dropdown
        const option = document.createElement('option');
        option.value = router.id;
        option.textContent = `${router.name} (ID: ${router.id})`;
        select.appendChild(option);
    });
    
    // Restore selection if it still exists
    if (currentValue && Array.from(select.options).some(opt => opt.value === currentValue)) {
        select.value = currentValue;
    }
}

window.toggleOSPF = function(routerId) {
    simulator.enable_ospf(routerId);
    const router = routers.find(r => r.id === routerId);
    if (router) {
        router.ospf_enabled = true;
        updateRoutersList();
        render();
    }
};

function render() {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    
    // Draw connections
    ctx.strokeStyle = '#666';
    ctx.lineWidth = 2;
    connections.forEach(conn => {
        const from = routers.find(r => r.id === conn.from_router_id);
        const to = routers.find(r => r.id === conn.to_router_id);
        if (from && to) {
            ctx.beginPath();
            ctx.moveTo(from.x, from.y);
            ctx.lineTo(to.x, to.y);
            ctx.stroke();
            
            // Draw cost label
            const midX = (from.x + to.x) / 2;
            const midY = (from.y + to.y) / 2;
            ctx.fillStyle = '#000';
            ctx.fillText(`Cost: ${conn.cost}`, midX, midY);
        }
    });
    
    // Draw packets
    if (packetVisualizer) {
        packetVisualizer.draw();
    }
    
    // Draw routers
    routers.forEach(router => {
        const isSelected = selectedRouters.includes(router.id);
        
        ctx.beginPath();
        ctx.arc(router.x, router.y, 20, 0, 2 * Math.PI);
        ctx.fillStyle = router.ospf_enabled ? '#4CAF50' : '#2196F3';
        ctx.fill();
        
        if (isSelected) {
            ctx.strokeStyle = '#ff0000';
            ctx.lineWidth = 3;
            ctx.stroke();
        }
        
        ctx.fillStyle = '#fff';
        ctx.font = 'bold 12px Arial';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(router.name, router.x, router.y);
    });
    
    // Draw packet statistics
    if (packetVisualizer && simulationRunning) {
        const stats = packetVisualizer.getPacketsByType();
        const activeCount = packetVisualizer.getActivePacketCount();
        
        ctx.fillStyle = '#000';
        ctx.font = '12px Arial';
        ctx.textAlign = 'left';
        ctx.fillText(`Active Packets: ${activeCount}`, 10, 20);
        
        let y = 40;
        Object.entries(stats).forEach(([type, count]) => {
            ctx.fillStyle = packetVisualizer.packetColors[type] || '#666';
            ctx.fillText(`${type}: ${count}`, 10, y);
            y += 20;
        });
    }
}

function log(message) {
    const logContent = document.getElementById('log-content');
    const entry = document.createElement('div');
    entry.className = 'log-entry';
    const timestamp = new Date().toLocaleTimeString();
    const fullMessage = `[${timestamp}] ${message}`;
    entry.textContent = fullMessage;
    logContent.appendChild(entry);
    logContent.scrollTop = logContent.scrollHeight;
    
    // Store log entry for export
    logEntries.push({
        timestamp: new Date().toISOString(),
        simulationTime: simulationTime,
        message: message
    });
}

function exportLog() {
    if (logEntries.length === 0) {
        alert('No log entries to export');
        return;
    }
    
    // Create log data with metadata
    const logData = {
        simulationName: 'OSPF Network Simulation',
        exportTime: new Date().toISOString(),
        totalEntries: logEntries.length,
        entries: logEntries
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
    
    log('Log exported successfully');
}

function clearLog() {
    const logContent = document.getElementById('log-content');
    logContent.innerHTML = '';
    logEntries = [];
    log('Log cleared');
}

function startSimulation() {
    if (simulationRunning) {
        stopSimulation();
        return;
    }
    
    log('Starting OSPF simulation...');
    simulator.start_simulation();
    simulationRunning = true;
    simulationTime = 0;
    packetVisualizer.clear();
    
    const btn = document.getElementById('simulate-btn');
    btn.textContent = 'Stop Simulation';
    btn.classList.add('running');
    
    // Update simulation every 100ms
    simulationInterval = setInterval(() => {
        simulator.step_simulation(0.1);
        simulationTime += 0.1;
        updateSimulationDisplay();
        
        // Update packet positions
        packetVisualizer.update(simulationTime);
        render();
    }, 100);
}

function stopSimulation() {
    if (!simulationRunning) return;
    
    log('Stopping simulation...');
    simulator.stop_simulation();
    simulationRunning = false;
    
    const btn = document.getElementById('simulate-btn');
    btn.textContent = 'Start Simulation';
    btn.classList.remove('running');
    
    if (simulationInterval) {
        clearInterval(simulationInterval);
        simulationInterval = null;
    }
}

function updateSimulationDisplay() {
    // Update routers and connections from simulator
    const routersJson = simulator.get_routers_json();
    const connectionsJson = simulator.get_connections_json();
    
    if (routersJson) {
        routers = JSON.parse(routersJson);
        updateRoutersList();
    }
    
    if (connectionsJson) {
        connections = JSON.parse(connectionsJson);
    }
    
    // Get recent events and display them
    const eventsJson = simulator.get_recent_events_json(10);
    if (eventsJson) {
        const events = JSON.parse(eventsJson);
        events.forEach(event => {
            if (event.description) {
                log(`[${event.timestamp.toFixed(2)}s] ${event.description}`);
                
                // Add packet visualization for packet events
                if (event.event_type && event.event_type.PacketSent) {
                    const fromRouter = routers.find(r => r.id === event.event_type.PacketSent.from_router);
                    const toRouter = routers.find(r => r.id === event.event_type.PacketSent.to_router);
                    if (fromRouter && toRouter) {
                        packetVisualizer.addPacket(
                            fromRouter, 
                            toRouter, 
                            event.event_type.PacketSent.packet_type,
                            event.timestamp
                        );
                    }
                }
            }
        });
    }
}

function handleRouterSelect(event) {
    const routerId = parseInt(event.target.value);
    if (!routerId) {
        document.getElementById('router-details').innerHTML = '';
        return;
    }
    
    const detailsJson = simulator.get_router_details_json(routerId);
    if (detailsJson) {
        const details = JSON.parse(detailsJson);
        displayRouterDetails(details);
    }
}

function displayRouterDetails(details) {
    const container = document.getElementById('router-details');
    let html = `<h4>${details.name} (ID: ${details.id})</h4>`;
    
    // Display interfaces
    if (details.interfaces && Object.keys(details.interfaces).length > 0) {
        html += '<h5>Interfaces:</h5><div class="interface-list">';
        Object.values(details.interfaces).forEach(iface => {
            html += `<div class="interface-item">
                Interface ${iface.id}: ${iface.ip_address}/${iface.netmask}
                ${iface.connected_router_id ? ` → Router ${iface.connected_router_id}` : ''}
                (Cost: ${iface.cost})
            </div>`;
        });
        html += '</div>';
    }
    
    // Display routing table
    if (details.routing_table && details.routing_table.length > 0) {
        html += '<h5>Routing Table:</h5>';
        html += '<table class="routing-table">';
        html += '<thead><tr><th>Destination</th><th>Next Hop</th><th>Interface</th><th>Metric</th><th>Protocol</th></tr></thead>';
        html += '<tbody>';
        details.routing_table.forEach(entry => {
            html += `<tr>
                <td>${entry.destination}/${entry.netmask}</td>
                <td>${entry.next_hop}</td>
                <td>Interface ${entry.interface_id}</td>
                <td>${entry.metric}</td>
                <td>${entry.protocol}</td>
            </tr>`;
        });
        html += '</tbody></table>';
    } else {
        html += '<p>No routes in routing table</p>';
    }
    
    // Display OSPF status
    html += `<h5>OSPF Status:</h5>`;
    html += `<p>OSPF Enabled: ${details.ospf_enabled ? 'Yes' : 'No'}</p>`;
    if (details.ospf_enabled) {
        html += `<p>Number of Neighbors: ${details.ospf_neighbors}</p>`;
    }
    
    container.innerHTML = html;
}

run();