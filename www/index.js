/**
 * Main Application Entry Point (Refactored)
 * 
 * This is the simplified main entry point that coordinates the various modules.
 * The complex functionality has been extracted into focused modules.
 */

import appInitializer from './modules/app-initializer.js';
import eventLogger from './modules/event-logger.js';

async function main() {
    try {
        // Initialize the application
        eventLogger.log('Starting OSPF Network Simulator...');
        await appInitializer.init();
        eventLogger.log('Application loaded successfully');
        
    } catch (error) {
        console.error('Failed to start application:', error);
        eventLogger.log(`Failed to start application: ${error.message}`);
    }
}

// Error handling for unhandled errors
window.addEventListener('error', (event) => {
    console.error('Unhandled error:', event.error);
    eventLogger.log(`Unhandled error: ${event.error.message}`);
});

window.addEventListener('unhandledrejection', (event) => {
    console.error('Unhandled promise rejection:', event.reason);
    eventLogger.log(`Unhandled promise rejection: ${event.reason}`);
    event.preventDefault();
});

// Cleanup on page unload
window.addEventListener('beforeunload', () => {
    appInitializer.cleanup();
});

// Start the application when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', main);
} else {
    // DOM is already loaded
    main();
}