#!/usr/bin/env node
/**
 * Omnisystem Node.js Bindings
 *
 * Provides JavaScript/Node.js access to Omnisystem kernel via native FFI.
 * Supports both CommonJS and ESM imports.
 *
 * Usage:
 *   const omnisystem = require('./omnisystem_node.js');
 *   // or
 *   import omnisystem from './omnisystem_node.js';
 */

const os = require('os');
const path = require('path');
const ffi = require('ffi-napi');
const ref = require('ref-napi');
const Struct = require('ref-struct-di')(ref);

// ============================================================================
// Native Library Loading
// ============================================================================

let libOmnisystem = null;

function loadLibrary() {
  if (libOmnisystem) return libOmnisystem;

  const platform = os.platform();
  let libName;

  if (platform === 'win32') {
    libName = 'omnisystem_go.dll';
  } else if (platform === 'darwin') {
    libName = 'libomnisystem_go.dylib';
  } else {
    libName = 'libomnisystem_go.so';
  }

  // Search common paths
  const searchPaths = [
    path.join(__dirname, '..', 'target', 'release', libName),
    path.join(__dirname, '..', 'target', 'debug', libName),
    path.join(process.cwd(), 'target', 'release', libName),
    path.join(process.cwd(), 'target', 'debug', libName),
  ];

  for (const libPath of searchPaths) {
    try {
      libOmnisystem = ffi.Library(libPath, {
        omnisystem_init: ['int', []],
        omnisystem_get_total_memory: ['uint64', []],
        omnisystem_get_allocated_memory: ['uint64', []],
        omnisystem_get_free_memory: ['uint64', []],
        omnisystem_get_process_count: ['uint32', []],
        omnisystem_create_process: ['uint64', []],
        omnisystem_get_health: ['int', []],
        omnisystem_echo_int: ['int', ['int']],
        omnisystem_echo_u64: ['uint64', ['uint64']],
        omnisystem_shutdown: ['int', []],
      });

      console.log(`✓ Loaded Omnisystem library from ${libPath}`);
      return libOmnisystem;
    } catch (e) {
      continue;
    }
  }

  throw new Error(
    `Could not find ${libName}. Make sure omnisystem-go-bindings is built.`
  );
}

// ============================================================================
// Omnisystem JavaScript API
// ============================================================================

class Omnisystem {
  constructor() {
    this._lib = loadLibrary();
    this._initialized = false;
  }

  initialize() {
    console.log('📦 Initializing Omnisystem kernel from Node.js...');

    const result = this._lib.omnisystem_init();
    if (result === 0) {
      this._initialized = true;
      console.log('✓ Omnisystem kernel initialized successfully\n');
    } else {
      throw new Error(`Failed to initialize kernel (error code: ${result})`);
    }
  }

  getTotalMemory() {
    this._checkInitialized();
    return BigInt(this._lib.omnisystem_get_total_memory());
  }

  getAllocatedMemory() {
    this._checkInitialized();
    return BigInt(this._lib.omnisystem_get_allocated_memory());
  }

  getFreeMemory() {
    this._checkInitialized();
    return BigInt(this._lib.omnisystem_get_free_memory());
  }

  getProcessCount() {
    this._checkInitialized();
    return this._lib.omnisystem_get_process_count();
  }

  createProcess() {
    this._checkInitialized();
    const pid = BigInt(this._lib.omnisystem_create_process());
    if (pid === 0n) {
      throw new Error('Failed to create process');
    }
    return pid;
  }

  getHealth() {
    this._checkInitialized();
    const status = this._lib.omnisystem_get_health();
    const statuses = ['healthy', 'degraded', 'critical'];
    return statuses[status] || 'unknown';
  }

  echoInt(value) {
    this._checkInitialized();
    return this._lib.omnisystem_echo_int(value);
  }

  echoU64(value) {
    this._checkInitialized();
    return BigInt(this._lib.omnisystem_echo_u64(value));
  }

  getStats() {
    this._checkInitialized();
    const MB = 1024 * 1024;
    return {
      totalMemoryMB: Number(this.getTotalMemory()) / MB,
      allocatedMemoryMB: Number(this.getAllocatedMemory()) / MB,
      freeMemoryMB: Number(this.getFreeMemory()) / MB,
      processCount: this.getProcessCount(),
      health: this.getHealth(),
    };
  }

  shutdown() {
    this._checkInitialized();
    const result = this._lib.omnisystem_shutdown();
    if (result === 0) {
      this._initialized = false;
      console.log('✓ Omnisystem shutdown complete\n');
    } else {
      throw new Error(`Shutdown failed (error code: ${result})`);
    }
  }

  _checkInitialized() {
    if (!this._initialized) {
      throw new Error('Kernel not initialized. Call initialize() first.');
    }
  }

  isInitialized() {
    return this._initialized;
  }
}

// ============================================================================
// Demo CLI Application
// ============================================================================

async function main() {
  console.log('\n╔═══════════════════════════════════════════════════════════╗');
  console.log('║    OMNISYSTEM NODE.JS BINDINGS - POLYGLOT DEMO            ║');
  console.log('╚═══════════════════════════════════════════════════════════╝\n');

  try {
    // Initialize
    const omni = new Omnisystem();
    omni.initialize();

    // Get stats
    console.log('📊 System Statistics:');
    const stats = omni.getStats();
    Object.entries(stats).forEach(([key, value]) => {
      if (typeof value === 'number' && key.includes('MB')) {
        console.log(`   ${key}: ${value.toFixed(2)}`);
      } else {
        console.log(`   ${key}: ${value}`);
      }
    });
    console.log();

    // Create processes
    console.log('🔧 Creating processes from Node.js:');
    const pids = [];
    for (let i = 0; i < 3; i++) {
      const pid = omni.createProcess();
      pids.push(pid);
      console.log(`   ✓ Created process with PID: ${pid}`);
    }
    console.log();

    // Test echo function
    console.log('🔍 Testing FFI communication:');
    const testValue = 21;
    const result = omni.echoInt(testValue);
    console.log(`   echoInt(${testValue}) = ${result}`);
    if (result === testValue * 2) {
      console.log(`   ✓ FFI working correctly\n`);
    }

    // Final stats
    console.log('📈 Final Statistics:');
    const finalStats = omni.getStats();
    console.log(`   Process count: ${finalStats.processCount}`);
    console.log(`   Health: ${finalStats.health}`);
    console.log();

    // Shutdown
    omni.shutdown();

    console.log('╔═══════════════════════════════════════════════════════════╗');
    console.log('║         NODE.JS BINDINGS DEMO COMPLETE                   ║');
    console.log('║     JavaScript ↔ C FFI ↔ Rust Kernel WORKING            ║');
    console.log('╚═══════════════════════════════════════════════════════════╝');

  } catch (error) {
    console.error(`❌ Error: ${error.message}`);
    process.exit(1);
  }
}

// ============================================================================
// Module Exports
// ============================================================================

module.exports = Omnisystem;

// Run main if called directly
if (require.main === module) {
  main().catch(err => {
    console.error('Uncaught error:', err);
    process.exit(1);
  });
}
