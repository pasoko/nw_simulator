// Debug version of index.js with error handling

// Create debug console
const debugDiv = document.createElement('div');
debugDiv.id = 'debug-console';
debugDiv.style.cssText = 'position: fixed; bottom: 0; left: 0; right: 0; height: 200px; background: black; color: lime; overflow-y: auto; padding: 10px; font-family: monospace; z-index: 9999;';
document.body.appendChild(debugDiv);

function debugLog(message, type = 'info') {
    const time = new Date().toLocaleTimeString();
    const color = type === 'error' ? 'red' : type === 'success' ? 'lime' : 'yellow';
    debugDiv.innerHTML += `<div style="color: ${color}">[${time}] ${message}</div>`;
    debugDiv.scrollTop = debugDiv.scrollHeight;
    console.log(message);
}

// Override console.error
const originalError = console.error;
console.error = function(...args) {
    debugLog('ERROR: ' + args.join(' '), 'error');
    originalError.apply(console, args);
};

debugLog('Debug mode started');

// Test basic functionality
try {
    debugLog('Testing button presence...');
    const buttons = [
        'add-router-btn',
        'connect-routers-btn',
        'simulate-btn',
        'delete-router-btn',
        'disconnect-routers-btn'
    ];
    
    buttons.forEach(id => {
        const btn = document.getElementById(id);
        if (btn) {
            debugLog(`✓ Found button: ${id}`, 'success');
            // Add test click handler
            btn.addEventListener('click', () => {
                debugLog(`Button clicked: ${id}`, 'success');
            });
        } else {
            debugLog(`✗ Missing button: ${id}`, 'error');
        }
    });
} catch (err) {
    debugLog(`Error in button test: ${err.message}`, 'error');
}

// Import modules with error handling
debugLog('Loading modules...');

import('./pkg/nw_simulator.js').then(async (module) => {
    debugLog('WASM module loaded', 'success');
    
    try {
        await module.default();
        debugLog('WASM initialized', 'success');
        
        const simulator = new module.NetworkSimulator();
        debugLog('NetworkSimulator created', 'success');
        
        // Make it globally available for testing
        window.testSimulator = simulator;
        debugLog('Simulator available as window.testSimulator', 'success');
        
        // Import PacketVisualizer
        const { PacketVisualizer } = await import('./packet-visualizer.js');
        debugLog('PacketVisualizer loaded', 'success');
        
        // Setup canvas
        const canvas = document.getElementById('network-canvas');
        if (canvas) {
            const ctx = canvas.getContext('2d');
            const packetViz = new PacketVisualizer(canvas, ctx);
            debugLog('Canvas and PacketVisualizer initialized', 'success');
        } else {
            debugLog('Canvas not found!', 'error');
        }
        
        debugLog('All modules loaded successfully!', 'success');
        
        // Now run the main application
        debugLog('Starting main application...');
        // Here you would call your main init function
        
    } catch (err) {
        debugLog(`Error during initialization: ${err.message}`, 'error');
        console.error(err);
    }
    
}).catch(err => {
    debugLog(`Failed to load WASM module: ${err.message}`, 'error');
    console.error(err);
});