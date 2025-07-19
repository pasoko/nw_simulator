#!/usr/bin/env node

/**
 * Test script for Terminal Manager GUI functionality
 */

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

console.log('Testing Terminal Manager GUI Integration...\n');

// Check if the terminal manager files exist
const baseDir = '/home/hyamada/claude/nw_simulator';
const filesToCheck = [
    path.join(baseDir, 'www/modules/terminal-manager.js'),
    path.join(baseDir, 'www/styles/terminal-manager.css'),
    path.join(baseDir, 'src/terminal_device.rs'),
    path.join(baseDir, 'src/terminal_manager.rs'),
    path.join(baseDir, 'src/enhanced_ping.rs')
];

console.log('1. Checking required files:');
filesToCheck.forEach(file => {
    const exists = fs.existsSync(file);
    console.log(`   ${exists ? '✓' : '✗'} ${file}`);
});

// Check if terminal manager is imported in the app
console.log('\n2. Checking module imports:');
const appInitializer = fs.readFileSync(path.join(baseDir, 'www/modules/app-initializer.js'), 'utf8');
const hasTerminalImport = appInitializer.includes("import terminalManager from './terminal-manager.js'");
console.log(`   ${hasTerminalImport ? '✓' : '✗'} Terminal manager imported in app-initializer.js`);

const canvasRenderer = fs.readFileSync(path.join(baseDir, 'www/modules/canvas-renderer.js'), 'utf8');
const hasTerminalDraw = canvasRenderer.includes('terminalManager.drawTerminals');
console.log(`   ${hasTerminalDraw ? '✓' : '✗'} Terminal drawing in canvas-renderer.js`);

// Check CSS inclusion
console.log('\n3. Checking CSS inclusion:');
const indexHtml = fs.readFileSync(path.join(baseDir, 'www/index.html'), 'utf8');
const hasTerminalCSS = indexHtml.includes('terminal-manager.css');
console.log(`   ${hasTerminalCSS ? '✓' : '✗'} Terminal CSS linked in index.html`);

// Run webpack build
console.log('\n4. Running webpack build:');
try {
    execSync(`cd ${path.join(baseDir, 'www')} && yarn build`, { stdio: 'inherit' });
    console.log('   ✓ Webpack build successful');
} catch (error) {
    console.log('   ✗ Webpack build failed');
    process.exit(1);
}

// Check for terminal API methods in lib.rs
console.log('\n5. Checking WASM API methods:');
const libRs = fs.readFileSync(path.join(baseDir, 'src/lib.rs'), 'utf8');
const terminalMethods = [
    'add_terminal',
    'remove_terminal',
    'connect_terminal_to_router',
    'disconnect_terminal',
    'send_ping_from_terminal',
    'set_terminal_failed',
    'get_terminal_info_json',
    'get_all_terminals_json',
    'start_enhanced_ping',
    'stop_ping_session',
    'get_ping_session_details'
];

terminalMethods.forEach(method => {
    const hasMethod = libRs.includes(`pub fn ${method}`);
    console.log(`   ${hasMethod ? '✓' : '✗'} ${method}()`);
});

console.log('\n✅ Terminal Manager GUI integration test complete!');
console.log('\nTo test the GUI manually:');
console.log('1. Open http://localhost:8000 in your browser');
console.log('2. Click "Add Terminal" button in the toolbar');
console.log('3. Add a terminal device with IP configuration');
console.log('4. Connect the terminal to a router');
console.log('5. Use the enhanced ping feature from terminal details');
console.log('\nThe terminal should appear as a computer icon on the canvas.');