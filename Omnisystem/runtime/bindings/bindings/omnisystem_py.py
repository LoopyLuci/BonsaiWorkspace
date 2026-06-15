#!/usr/bin/env python3
"""
Omnisystem Python Bindings

Pure Python bindings to the Omnisystem kernel via ctypes.
Provides Pythonic API for accessing Omnisystem from Python code.
"""

import ctypes
import os
import sys
from typing import Optional, Dict, Any
from pathlib import Path


class OmnisystemError(Exception):
    """Base exception for Omnisystem errors"""
    pass


class OmnisystemNotInitializedError(OmnisystemError):
    """Raised when Omnisystem kernel is not initialized"""
    pass


class OmnisystemLibrary:
    """Load and manage Omnisystem native library"""

    def __init__(self):
        self.lib = None
        self._load_library()

    def _load_library(self):
        """Load the omnisystem_go native library"""
        # Determine platform and library name
        if sys.platform == "win32":
            lib_name = "omnisystem_go.dll"
        elif sys.platform == "darwin":
            lib_name = "libomnisystem_go.dylib"
        else:
            lib_name = "libomnisystem_go.so"

        # Search in common locations
        search_paths = [
            Path.cwd() / "target" / "release" / lib_name,
            Path.cwd() / "target" / "debug" / lib_name,
            Path(__file__).parent.parent / "target" / "release" / lib_name,
            Path(__file__).parent.parent / "target" / "debug" / lib_name,
        ]

        for path in search_paths:
            if path.exists():
                try:
                    self.lib = ctypes.CDLL(str(path))
                    print(f"✓ Loaded Omnisystem library from {path}")
                    return
                except OSError:
                    continue

        raise OmnisystemError(
            f"Could not find {lib_name}. "
            "Make sure omnisystem-go-bindings is built with 'cargo build --release'"
        )

    def get_function(self, name: str, argtypes: list, restype: type):
        """Get and configure a C function"""
        if not self.lib:
            raise OmnisystemNotInitializedError("Library not loaded")

        func = getattr(self.lib, name)
        func.argtypes = argtypes
        func.restype = restype
        return func


# Global library instance
_lib_instance = None

def _get_lib() -> OmnisystemLibrary:
    """Get or create global library instance"""
    global _lib_instance
    if _lib_instance is None:
        _lib_instance = OmnisystemLibrary()
    return _lib_instance


class Omnisystem:
    """High-level Python API for Omnisystem kernel"""

    def __init__(self):
        self._lib = _get_lib()
        self._initialized = False

    def initialize(self) -> None:
        """Initialize the Omnisystem kernel"""
        print("📦 Initializing Omnisystem kernel from Python...")

        init_func = self._lib.get_function(
            "omnisystem_init",
            [],
            ctypes.c_int
        )

        result = init_func()
        if result == 0:
            self._initialized = True
            print("✓ Omnisystem kernel initialized successfully")
        else:
            raise OmnisystemError(f"Failed to initialize kernel (error code: {result})")

    def get_total_memory(self) -> int:
        """Get total system memory in bytes"""
        self._check_initialized()

        func = self._lib.get_function(
            "omnisystem_get_total_memory",
            [],
            ctypes.c_uint64
        )

        return func()

    def get_allocated_memory(self) -> int:
        """Get allocated memory in bytes"""
        self._check_initialized()

        func = self._lib.get_function(
            "omnisystem_get_allocated_memory",
            [],
            ctypes.c_uint64
        )

        return func()

    def get_free_memory(self) -> int:
        """Get free memory in bytes"""
        self._check_initialized()

        func = self._lib.get_function(
            "omnisystem_get_free_memory",
            [],
            ctypes.c_uint64
        )

        return func()

    def get_process_count(self) -> int:
        """Get number of processes"""
        self._check_initialized()

        func = self._lib.get_function(
            "omnisystem_get_process_count",
            [],
            ctypes.c_uint32
        )

        return func()

    def create_process(self) -> int:
        """Create a new process, returns process ID"""
        self._check_initialized()

        func = self._lib.get_function(
            "omnisystem_create_process",
            [],
            ctypes.c_uint64
        )

        pid = func()
        if pid == 0:
            raise OmnisystemError("Failed to create process")

        return pid

    def get_health(self) -> str:
        """Get system health status"""
        self._check_initialized()

        func = self._lib.get_function(
            "omnisystem_get_health",
            [],
            ctypes.c_int
        )

        status = func()
        if status == 0:
            return "healthy"
        elif status == 1:
            return "degraded"
        else:
            return "critical"

    def echo_int(self, value: int) -> int:
        """Echo function - returns value * 2"""
        self._check_initialized()

        func = self._lib.get_function(
            "omnisystem_echo_int",
            [ctypes.c_int],
            ctypes.c_int
        )

        return func(value)

    def shutdown(self) -> None:
        """Gracefully shutdown Omnisystem"""
        self._check_initialized()

        func = self._lib.get_function(
            "omnisystem_shutdown",
            [],
            ctypes.c_int
        )

        result = func()
        if result == 0:
            self._initialized = False
            print("✓ Omnisystem shutdown complete")
        else:
            raise OmnisystemError(f"Shutdown failed (error code: {result})")

    def get_stats(self) -> Dict[str, Any]:
        """Get comprehensive system statistics"""
        self._check_initialized()

        return {
            "total_memory_mb": self.get_total_memory() / (1024 * 1024),
            "allocated_memory_mb": self.get_allocated_memory() / (1024 * 1024),
            "free_memory_mb": self.get_free_memory() / (1024 * 1024),
            "process_count": self.get_process_count(),
            "health": self.get_health(),
        }

    def _check_initialized(self) -> None:
        """Check if kernel is initialized"""
        if not self._initialized:
            raise OmnisystemNotInitializedError(
                "Kernel not initialized. Call initialize() first."
            )

    def is_initialized(self) -> bool:
        """Check if kernel is initialized"""
        return self._initialized


def main():
    """Example usage of Omnisystem from Python"""
    print("╔════════════════════════════════════════════════════════════╗")
    print("║     OMNISYSTEM PYTHON BINDINGS - POLYGLOT DEMO             ║")
    print("╚════════════════════════════════════════════════════════════╝\n")

    try:
        # Initialize
        omni = Omnisystem()
        omni.initialize()
        print()

        # Get stats
        print("📊 System Statistics:")
        stats = omni.get_stats()
        for key, value in stats.items():
            if isinstance(value, float):
                print(f"   {key}: {value:.2f}")
            else:
                print(f"   {key}: {value}")
        print()

        # Create processes
        print("🔧 Creating processes from Python:")
        for i in range(3):
            pid = omni.create_process()
            print(f"   ✓ Created process with PID: {pid}")
        print()

        # Test echo function
        print("🔍 Testing FFI communication:")
        test_value = 21
        result = omni.echo_int(test_value)
        print(f"   echo_int({test_value}) = {result}")
        assert result == test_value * 2, "Echo function returned unexpected value"
        print(f"   ✓ FFI working correctly\n")

        # Final stats
        print("📈 Final Statistics:")
        final_stats = omni.get_stats()
        print(f"   Process count: {final_stats['process_count']}")
        print(f"   Health: {final_stats['health']}")
        print()

        # Shutdown
        omni.shutdown()

        print("╔════════════════════════════════════════════════════════════╗")
        print("║              PYTHON BINDINGS DEMO COMPLETE                 ║")
        print("║         Python ↔ C FFI ↔ Rust Kernel WORKING             ║")
        print("╚════════════════════════════════════════════════════════════╝")

    except OmnisystemError as e:
        print(f"❌ Omnisystem error: {e}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"❌ Unexpected error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
