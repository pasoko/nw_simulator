// Import without bundling WASM
import initWasm, { NetworkSimulator } from './pkg/nw_simulator.js';
import { PacketVisualizer } from './packet-visualizer.js';

// Override the default WASM path
const init = () => initWasm('./pkg/nw_simulator_bg.wasm');

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
let lastMouseX = 0;
let lastMouseY = 0;
let draggingRouter = null;
let dragOffset = { x: 0, y: 0 };

async function run() {
    try {
        await init();
        
        simulator = new NetworkSimulator();
        
        canvas = document.getElementById('network-canvas');
        if (!canvas) {
            throw new Error('Canvas element not found');
        }
        ctx = canvas.getContext('2d');
        
        setupCanvas();
        setupEventListeners();
        
        packetVisualizer = new PacketVisualizer(canvas, ctx);
        
        render();
        
        log('Network Simulator initialized');
    } catch (error) {
        console.error('Error during initialization:', error);
        log(`Error: ${error.message}`);
        // Show error in UI
        const errorDiv = document.createElement('div');
        errorDiv.style.cssText = 'position: fixed; top: 10px; left: 50%; transform: translateX(-50%); background: red; color: white; padding: 10px; border-radius: 5px; z-index: 9999;';
        errorDiv.textContent = `Initialization Error: ${error.message}`;
        document.body.appendChild(errorDiv);
    }
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
    canvas.addEventListener('mousedown', handleMouseDown);
    canvas.addEventListener('mousemove', handleMouseMove);
    canvas.addEventListener('mouseup', handleMouseUp);
    canvas.addEventListener('mouseleave', handleMouseUp); // Stop dragging when mouse leaves canvas
    
    // Check if buttons exist before adding listeners
    const addRouterBtn = document.getElementById('add-router-btn');
    if (addRouterBtn) {
        addRouterBtn.addEventListener('click', () => {
            setMode('add-router');
        });
    }
    
    const connectBtn = document.getElementById('connect-routers-btn');
    if (connectBtn) {
        connectBtn.addEventListener('click', () => {
            setMode('connect-routers');
        });
    }
    
    const simulateBtn = document.getElementById('simulate-btn');
    if (simulateBtn) {
        simulateBtn.addEventListener('click', startSimulation);
    }
    
    const exportBtn = document.getElementById('export-log-btn');
    if (exportBtn) {
        exportBtn.addEventListener('click', exportLog);
    }
    
    const clearBtn = document.getElementById('clear-log-btn');
    if (clearBtn) {
        clearBtn.addEventListener('click', clearLog);
    }
    
    const routerSelect = document.getElementById('router-select');
    if (routerSelect) {
        routerSelect.addEventListener('change', handleRouterSelect);
    }
    
    const deleteBtn = document.getElementById('delete-router-btn');
    if (deleteBtn) {
        deleteBtn.addEventListener('click', () => {
            setMode('delete-router');
        });
    }
    
    const disconnectBtn = document.getElementById('disconnect-routers-btn');
    if (disconnectBtn) {
        disconnectBtn.addEventListener('click', () => {
            setMode('disconnect-routers');
        });
    }
}

function setMode(newMode) {
    mode = newMode;
    selectedRouters = [];
    const indicator = document.getElementById('mode-indicator');
    
    switch(mode) {
        case 'add-router':
            indicator.textContent = 'Mode: Add Router - Click on canvas to place router';
            indicator.style.backgroundColor = '#ffc107';
            canvas.style.cursor = 'crosshair';
            break;
        case 'connect-routers':
            indicator.textContent = 'Mode: Connect Routers - Select first router';
            indicator.style.backgroundColor = '#17a2b8';
            canvas.style.cursor = 'pointer';
            break;
        case 'delete-router':
            indicator.textContent = 'Mode: Delete Router - Click on router to delete';
            indicator.style.backgroundColor = '#dc3545';
            canvas.style.cursor = 'pointer';
            break;
        case 'disconnect-routers':
            indicator.textContent = 'Mode: Disconnect Routers - Select first router';
            indicator.style.backgroundColor = '#dc3545';
            canvas.style.cursor = 'pointer';
            break;
    }
    
    render();
}

function handleMouseDown(event) {
    const rect = canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    
    // Check if clicking on a router for dragging
    const clickedRouter = findRouterAt(x, y);
    if (clickedRouter && mode !== 'delete-router' && mode !== 'connect-routers' && mode !== 'disconnect-routers') {
        draggingRouter = clickedRouter;
        dragOffset.x = x - clickedRouter.x;
        dragOffset.y = y - clickedRouter.y;
        canvas.style.cursor = 'grabbing';
        event.preventDefault(); // Prevent text selection
    }
}

function handleMouseMove(event) {
    const rect = canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    
    lastMouseX = x;
    lastMouseY = y;
    
    if (draggingRouter) {
        // Update router position
        draggingRouter.x = Math.max(20, Math.min(canvas.width - 20, x - dragOffset.x));
        draggingRouter.y = Math.max(20, Math.min(canvas.height - 20, y - dragOffset.y));
        
        // Update position in simulator
        simulator.update_router_position(draggingRouter.id, draggingRouter.x, draggingRouter.y);
        
        render();
    } else {
        // Update cursor based on hover
        const hoverRouter = findRouterAt(x, y);
        if (hoverRouter && mode !== 'delete-router' && mode !== 'connect-routers' && mode !== 'disconnect-routers') {
            canvas.style.cursor = 'grab';
        } else if (mode === 'delete-router' && hoverRouter) {
            canvas.style.cursor = 'pointer';
        } else if ((mode === 'connect-routers' || mode === 'disconnect-routers') && hoverRouter) {
            canvas.style.cursor = 'pointer';
        } else if (mode === 'add-router') {
            canvas.style.cursor = 'crosshair';
        } else {
            canvas.style.cursor = 'default';
        }
        
        // Re-render for hover effects
        if (mode === 'delete-router') {
            render();
        }
    }
}

function handleMouseUp(event) {
    if (draggingRouter) {
        draggingRouter = null;
        canvas.style.cursor = 'default';
    }
}

function handleCanvasClick(event) {
    // Ignore clicks if we're dragging
    if (draggingRouter) return;
    
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
            const indicator = document.getElementById('mode-indicator');
            
            if (selectedRouters.includes(clickedRouter.id)) {
                // Deselect if clicking the same router
                selectedRouters = selectedRouters.filter(id => id !== clickedRouter.id);
                if (selectedRouters.length === 0) {
                    indicator.textContent = 'Mode: Connect Routers - Select first router';
                } else {
                    indicator.textContent = 'Mode: Connect Routers - Select second router';
                }
            } else {
                selectedRouters.push(clickedRouter.id);
                if (selectedRouters.length === 1) {
                    indicator.textContent = `Mode: Connect Routers - First router selected (${clickedRouter.name}). Select second router`;
                    indicator.style.backgroundColor = '#28a745';
                } else if (selectedRouters.length === 2) {
                    const cost = parseInt(prompt('Enter link cost:', '1') || '1');
                    simulator.connect_routers(selectedRouters[0], selectedRouters[1], cost);
                    
                    // Add connection to local state immediately
                    connections.push({
                        from_router_id: selectedRouters[0],
                        to_router_id: selectedRouters[1],
                        cost: cost
                    });
                    
                    const from = routers.find(r => r.id === selectedRouters[0]);
                    const to = routers.find(r => r.id === selectedRouters[1]);
                    log(`Connected routers ${from.name} and ${to.name} with cost ${cost}`);
                    
                    selectedRouters = [];
                    indicator.textContent = 'Mode: Connect Routers - Connection created! Select first router for next connection';
                    indicator.style.backgroundColor = '#17a2b8';
                }
            }
            render();
        }
    } else if (mode === 'delete-router') {
        const clickedRouter = findRouterAt(x, y);
        if (clickedRouter) {
            if (confirm(`Are you sure you want to delete router "${clickedRouter.name}"?`)) {
                simulator.delete_router(clickedRouter.id);
                // Update local state
                routers = routers.filter(r => r.id !== clickedRouter.id);
                connections = connections.filter(c => 
                    c.from_router_id !== clickedRouter.id && c.to_router_id !== clickedRouter.id
                );
                updateRoutersList();
                render();
                log(`Router ${clickedRouter.name} deleted`);
            }
        }
    } else if (mode === 'disconnect-routers') {
        const clickedRouter = findRouterAt(x, y);
        if (clickedRouter) {
            const indicator = document.getElementById('mode-indicator');
            
            if (selectedRouters.includes(clickedRouter.id)) {
                // Deselect if clicking the same router
                selectedRouters = selectedRouters.filter(id => id !== clickedRouter.id);
                if (selectedRouters.length === 0) {
                    indicator.textContent = 'Mode: Disconnect Routers - Select first router';
                } else {
                    indicator.textContent = 'Mode: Disconnect Routers - Select second router';
                }
            } else {
                selectedRouters.push(clickedRouter.id);
                if (selectedRouters.length === 1) {
                    indicator.textContent = `Mode: Disconnect Routers - First router selected (${clickedRouter.name}). Select second router`;
                    indicator.style.backgroundColor = '#dc3545';
                } else if (selectedRouters.length === 2) {
                    // Check if connection exists
                    const connectionExists = connections.some(c => 
                        (c.from_router_id === selectedRouters[0] && c.to_router_id === selectedRouters[1]) ||
                        (c.from_router_id === selectedRouters[1] && c.to_router_id === selectedRouters[0])
                    );
                    
                    if (connectionExists) {
                        simulator.disconnect_routers(selectedRouters[0], selectedRouters[1]);
                        // Update local state
                        connections = connections.filter(c => 
                            !((c.from_router_id === selectedRouters[0] && c.to_router_id === selectedRouters[1]) ||
                              (c.from_router_id === selectedRouters[1] && c.to_router_id === selectedRouters[0]))
                        );
                        indicator.textContent = 'Mode: Disconnect Routers - Connection removed! Select first router for next disconnection';
                        log(`Disconnected routers ${selectedRouters[0]} and ${selectedRouters[1]}`);
                    } else {
                        indicator.textContent = 'Mode: Disconnect Routers - No connection exists between these routers';
                    }
                    selectedRouters = [];
                }
            }
            render();
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
        const isDragging = draggingRouter && draggingRouter.id === router.id;
        
        // Draw dragging highlight
        if (isDragging) {
            ctx.beginPath();
            ctx.arc(router.x, router.y, 25, 0, 2 * Math.PI);
            ctx.strokeStyle = '#2196F3';
            ctx.lineWidth = 3;
            ctx.stroke();
        }
        
        // Draw selection ring for various modes
        if (isSelected && (mode === 'connect-routers' || mode === 'disconnect-routers')) {
            ctx.beginPath();
            ctx.arc(router.x, router.y, 25, 0, 2 * Math.PI);
            if (mode === 'connect-routers') {
                ctx.strokeStyle = selectedRouters.indexOf(router.id) === 0 ? '#ff9800' : '#4caf50';
            } else {
                ctx.strokeStyle = selectedRouters.indexOf(router.id) === 0 ? '#dc3545' : '#f44336';
            }
            ctx.lineWidth = 4;
            ctx.stroke();
        }
        
        // Highlight router on hover in delete mode
        if (mode === 'delete-router') {
            const mouseX = lastMouseX || 0;
            const mouseY = lastMouseY || 0;
            const dx = router.x - mouseX;
            const dy = router.y - mouseY;
            if (dx * dx + dy * dy < 400) { // 20px radius
                ctx.beginPath();
                ctx.arc(router.x, router.y, 25, 0, 2 * Math.PI);
                ctx.strokeStyle = '#dc3545';
                ctx.lineWidth = 3;
                ctx.setLineDash([5, 5]);
                ctx.stroke();
                ctx.setLineDash([]);
            }
        }
        
        ctx.beginPath();
        ctx.arc(router.x, router.y, 20, 0, 2 * Math.PI);
        ctx.fillStyle = router.ospf_enabled ? '#4CAF50' : '#2196F3';
        ctx.fill();
        
        if (isSelected && mode === 'connect-routers') {
            ctx.strokeStyle = '#000';
            ctx.lineWidth = 2;
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
        const newRouters = JSON.parse(routersJson);
        // Only update if there's a change in routers
        if (JSON.stringify(routers) !== JSON.stringify(newRouters)) {
            routers = newRouters;
            updateRoutersList();
        }
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

// Wait for DOM to be fully loaded
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', run);
} else {
    // DOM is already loaded
    run();
}