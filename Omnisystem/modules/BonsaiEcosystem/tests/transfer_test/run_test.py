#!/usr/bin/env python3
"""
TransferDaemon P2P Real Internet Test Runner
Orchestrates two real Sanctum vault nodes and runs comprehensive file transfer tests
"""

import os
import sys
import json
import subprocess
import time
import logging
import argparse
from datetime import datetime
from pathlib import Path
from typing import Optional, Dict, List

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='[%(levelname)s] %(asctime)s - %(message)s'
)
logger = logging.getLogger(__name__)


class TransferTestRunner:
    """Orchestrates TransferDaemon P2P real internet test"""

    def __init__(self, config_path: Optional[str] = None):
        self.config = self._load_config(config_path)
        self.test_start_time = None
        self.test_results = {}

    def _load_config(self, config_path: Optional[str]) -> Dict:
        """Load test configuration"""
        if config_path and os.path.exists(config_path):
            logger.info(f"Loading configuration from: {config_path}")
            with open(config_path, 'r') as f:
                return json.load(f)

        # Default configuration
        return {
            "node_a": {
                "id": os.getenv("FTDAEMON_NODE_A_ID", "node-a"),
                "address": os.getenv("FTDAEMON_NODE_A_ADDR", "127.0.0.1"),
                "port": int(os.getenv("FTDAEMON_NODE_A_PORT", "8114")),
                "vault_id": os.getenv("FTDAEMON_NODE_A_VAULT_ID", "vault-a"),
            },
            "node_b": {
                "id": os.getenv("FTDAEMON_NODE_B_ID", "node-b"),
                "address": os.getenv("FTDAEMON_NODE_B_ADDR", "127.0.0.1"),
                "port": int(os.getenv("FTDAEMON_NODE_B_PORT", "8115")),
                "vault_id": os.getenv("FTDAEMON_NODE_B_VAULT_ID", "vault-b"),
            },
            "file_size_mb": int(os.getenv("FTDAEMON_TEST_FILE_SIZE_MB", "50")),
            "file_count": int(os.getenv("FTDAEMON_TEST_FILE_COUNT", "1")),
            "timeout_seconds": int(os.getenv("FTDAEMON_TEST_TIMEOUT", "300")),
            "multi_path": os.getenv("FTDAEMON_MULTI_PATH", "true").lower() == "true",
            "encryption": os.getenv("FTDAEMON_ENCRYPTION", "true").lower() == "true",
            "verify_integrity": os.getenv("FTDAEMON_VERIFY_INTEGRITY", "true").lower() == "true",
        }

    def verify_node_connectivity(self, node_id: str, address: str, port: int) -> bool:
        """Verify node is reachable"""
        logger.info(f"Verifying connectivity to {node_id} ({address}:{port})...")

        try:
            # Try to connect to FTDaemon API
            import socket
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.settimeout(5)
            result = sock.connect_ex((address, port))
            sock.close()

            if result == 0:
                logger.info(f"✅ {node_id} is reachable")
                return True
            else:
                logger.error(f"❌ {node_id} is not reachable")
                return False
        except Exception as e:
            logger.error(f"Connection verification failed: {e}")
            return False

    def verify_prerequisites(self) -> bool:
        """Verify all prerequisites are met"""
        logger.info("Verifying prerequisites...")

        # Check if omni compiler is available
        try:
            result = subprocess.run(["omni", "--version"], capture_output=True, timeout=5)
            if result.returncode != 0:
                logger.error("Omni compiler not found or not working")
                return False
            logger.info(f"Omni compiler: {result.stdout.decode().strip()}")
        except Exception as e:
            logger.error(f"Failed to check Omni compiler: {e}")
            return False

        # Verify node connectivity
        if not self.verify_node_connectivity(
            self.config["node_a"]["id"],
            self.config["node_a"]["address"],
            self.config["node_a"]["port"]
        ):
            return False

        if not self.verify_node_connectivity(
            self.config["node_b"]["id"],
            self.config["node_b"]["address"],
            self.config["node_b"]["port"]
        ):
            return False

        logger.info("✅ All prerequisites verified")
        return True

    def compile_test_suite(self) -> bool:
        """Compile Omni-language test modules"""
        logger.info("Compiling test suite in Omni-languages...")

        # Compile main test orchestrator
        try:
            result = subprocess.run(
                ["omni", "build", "-m", "transfer_test::main", "--release"],
                cwd=os.path.dirname(__file__),
                capture_output=True,
                timeout=300
            )

            if result.returncode != 0:
                logger.error(f"Compilation failed: {result.stderr.decode()}")
                return False

            logger.info("✅ Test suite compiled successfully")
            return True
        except Exception as e:
            logger.error(f"Compilation error: {e}")
            return False

    def run_test(self) -> Dict:
        """Run the actual test"""
        logger.info("Starting TransferDaemon P2P real internet test...")
        self.test_start_time = time.time()

        # Set environment variables
        env = os.environ.copy()
        env.update({
            "FTDAEMON_NODE_A_ID": self.config["node_a"]["id"],
            "FTDAEMON_NODE_A_ADDR": self.config["node_a"]["address"],
            "FTDAEMON_NODE_A_PORT": str(self.config["node_a"]["port"]),
            "FTDAEMON_NODE_B_ID": self.config["node_b"]["id"],
            "FTDAEMON_NODE_B_ADDR": self.config["node_b"]["address"],
            "FTDAEMON_NODE_B_PORT": str(self.config["node_b"]["port"]),
            "FTDAEMON_TEST_FILE_SIZE_MB": str(self.config["file_size_mb"]),
            "FTDAEMON_MULTI_PATH": str(self.config["multi_path"]).lower(),
            "FTDAEMON_ENCRYPTION": str(self.config["encryption"]).lower(),
            "FTDAEMON_VERIFY_INTEGRITY": str(self.config["verify_integrity"]).lower(),
        })

        try:
            # Run the compiled test
            result = subprocess.run(
                ["omni", "run", "transfer_test"],
                env=env,
                capture_output=True,
                timeout=self.config["timeout_seconds"] + 30
            )

            test_duration = time.time() - self.test_start_time

            self.test_results = {
                "test_id": f"ftdaemon-p2p-{datetime.now().isoformat()}",
                "status": "passed" if result.returncode == 0 else "failed",
                "duration_seconds": test_duration,
                "exit_code": result.returncode,
                "stdout": result.stdout.decode(),
                "stderr": result.stderr.decode(),
            }

            return self.test_results

        except subprocess.TimeoutExpired:
            self.test_results = {
                "test_id": f"ftdaemon-p2p-{datetime.now().isoformat()}",
                "status": "timeout",
                "duration_seconds": time.time() - self.test_start_time,
                "error": "Test timed out",
            }
            return self.test_results
        except Exception as e:
            self.test_results = {
                "test_id": f"ftdaemon-p2p-{datetime.now().isoformat()}",
                "status": "error",
                "duration_seconds": time.time() - self.test_start_time,
                "error": str(e),
            }
            return self.test_results

    def generate_report(self, output_path: Optional[str] = None) -> str:
        """Generate test report"""
        logger.info("Generating test report...")

        report = {
            "test_name": "TransferDaemon Real Internet P2P Transfer Test",
            "timestamp": datetime.now().isoformat(),
            "configuration": self.config,
            "results": self.test_results,
        }

        # Save to file
        if output_path is None:
            output_path = f"/tmp/ftdaemon_test_report_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"

        os.makedirs(os.path.dirname(output_path), exist_ok=True)
        with open(output_path, 'w') as f:
            json.dump(report, f, indent=2)

        logger.info(f"Report saved to: {output_path}")

        # Print summary
        print("\n" + "=" * 80)
        print("TEST REPORT SUMMARY")
        print("=" * 80)
        print(f"Test Status: {self.test_results.get('status', 'unknown').upper()}")
        print(f"Duration: {self.test_results.get('duration_seconds', 0):.2f} seconds")
        print(f"File Size: {self.config['file_size_mb']} MB")
        print(f"Multi-path: {self.config['multi_path']}")
        print(f"Encryption: {self.config['encryption']}")
        print("=" * 80)
        print()

        return output_path

    def run(self) -> int:
        """Run complete test workflow"""
        print("╔════════════════════════════════════════════════════════════════════════════╗")
        print("║        TransferDaemon P2P Real Internet Transfer Test                     ║")
        print("║     Testing multi-path bonding, NAT traversal, and file integrity         ║")
        print("╚════════════════════════════════════════════════════════════════════════════╝")
        print()

        # Verify prerequisites
        if not self.verify_prerequisites():
            logger.error("Prerequisites verification failed")
            return 1

        # Compile test suite
        if not self.compile_test_suite():
            logger.error("Test suite compilation failed")
            return 1

        # Run test
        results = self.run_test()

        # Generate report
        self.generate_report()

        # Print test output
        print("\nTest Output:")
        print("-" * 80)
        print(results.get("stdout", ""))
        if results.get("stderr"):
            print("\nErrors:")
            print(results.get("stderr", ""))
        print("-" * 80)

        # Return exit code
        return 0 if results["status"] == "passed" else 1


def main():
    parser = argparse.ArgumentParser(
        description="TransferDaemon P2P Real Internet Test Runner"
    )
    parser.add_argument("--config", help="Path to configuration file")
    parser.add_argument("--output", help="Path for test report output")
    parser.add_argument("--file-size", type=int, help="Test file size in MB")
    parser.add_argument("--timeout", type=int, help="Test timeout in seconds")
    parser.add_argument("--no-encryption", action="store_true", help="Disable encryption")
    parser.add_argument("--no-multi-path", action="store_true", help="Disable multi-path")

    args = parser.parse_args()

    runner = TransferTestRunner(args.config)

    # Override config with command-line arguments
    if args.file_size:
        runner.config["file_size_mb"] = args.file_size
    if args.timeout:
        runner.config["timeout_seconds"] = args.timeout
    if args.no_encryption:
        runner.config["encryption"] = False
    if args.no_multi_path:
        runner.config["multi_path"] = False

    exit_code = runner.run()
    sys.exit(exit_code)


if __name__ == "__main__":
    main()
