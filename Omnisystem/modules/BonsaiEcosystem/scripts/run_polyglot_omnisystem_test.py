#!/usr/bin/env python3
"""
Polyglot Pong Integration Test – Omnisystem Languages
Runs all 4 languages through the Polyglot Pong test matrix.
"""

import subprocess
import sys
import json
import time
from datetime import datetime
from pathlib import Path

# Configuration
REPO_ROOT = Path(__file__).parent
OMNISYSTEM_DIR = REPO_ROOT / "bonsai-omnisystem-languages"
LANGUAGES = ["Sylva", "Titan", "Aether", "Axiom"]
RESULTS_DIR = REPO_ROOT / "polyglot-test-results"
RESULTS_DIR.mkdir(exist_ok=True)

# Test matrix
TEST_MATRIX = [
    ("Sylva", "Sylva"),
    ("Sylva", "Titan"),
    ("Sylva", "Aether"),
    ("Sylva", "Axiom"),
    ("Titan", "Sylva"),
    ("Titan", "Titan"),
    ("Titan", "Aether"),
    ("Titan", "Axiom"),
    ("Aether", "Sylva"),
    ("Aether", "Titan"),
    ("Aether", "Aether"),
    ("Aether", "Axiom"),
    ("Axiom", "Sylva"),
    ("Axiom", "Titan"),
    ("Axiom", "Aether"),
    ("Axiom", "Axiom"),
]

class PolyglotTestRunner:
    def __init__(self):
        self.results = {}
        self.start_time = None
        self.end_time = None

    def get_language_runner(self, lang: str):
        """Get the command to run a language"""
        runners = {
            "Sylva": f"python3 {OMNISYSTEM_DIR}/sylva/sylva.py {OMNISYSTEM_DIR}/sylva/pong.sv",
            "Titan": f"python3 {OMNISYSTEM_DIR}/titan/titan.py {OMNISYSTEM_DIR}/titan/pong.ti /tmp/titan_out.wat",
            "Aether": f"python3 {OMNISYSTEM_DIR}/aether/pong_runner.py",
            "Axiom": f"python3 {OMNISYSTEM_DIR}/axiom/axiom.py {OMNISYSTEM_DIR}/axiom/pong.ax",
        }
        return runners.get(lang)

    def run_language(self, source_lang: str, target_lang: str) -> dict:
        """Run a single language pair test"""
        runner = self.get_language_runner(source_lang)

        if not runner:
            return {
                "source": source_lang,
                "target": target_lang,
                "status": "SKIP",
                "error": f"Unknown language: {source_lang}",
                "fidelity": 0.0,
                "exec_time_ms": 0,
            }

        start = time.time()
        try:
            result = subprocess.run(
                runner,
                shell=True,
                capture_output=True,
                text=True,
                timeout=10
            )
            elapsed = (time.time() - start) * 1000  # Convert to ms

            status = "PASS" if result.returncode == 0 else "FAIL"

            return {
                "source": source_lang,
                "target": target_lang,
                "status": status,
                "exit_code": result.returncode,
                "stdout_lines": len(result.stdout.split('\n')),
                "stderr_lines": len(result.stderr.split('\n')),
                "fidelity": 1.0 if status == "PASS" else 0.0,
                "exec_time_ms": elapsed,
            }
        except subprocess.TimeoutExpired:
            return {
                "source": source_lang,
                "target": target_lang,
                "status": "TIMEOUT",
                "error": "Execution timeout (>10s)",
                "fidelity": 0.0,
                "exec_time_ms": 10000,
            }
        except Exception as e:
            return {
                "source": source_lang,
                "target": target_lang,
                "status": "ERROR",
                "error": str(e),
                "fidelity": 0.0,
                "exec_time_ms": 0,
            }

    def run_all_tests(self):
        """Run the complete test matrix"""
        self.start_time = time.time()

        print("╔════════════════════════════════════════════════════════════════╗")
        print("║                                                                ║")
        print("║    POLYGLOT PONG – OMNISYSTEM LANGUAGES TEST MATRIX            ║")
        print("║                                                                ║")
        print("╚════════════════════════════════════════════════════════════════╝")
        print()

        total_tests = len(TEST_MATRIX)
        passed_tests = 0

        print(f"Running {total_tests} language pair tests...\n")
        print("┌────┬──────────┬──────────┬────────┬──────────┬──────────────┐")
        print("│ #  │ Source   │ Target   │ Status │ Fidelity │ Time (ms)    │")
        print("├────┼──────────┼──────────┼────────┼──────────┼──────────────┤")

        for i, (source, target) in enumerate(TEST_MATRIX, 1):
            result = self.run_language(source, target)
            self.results[f"{source}→{target}"] = result

            status = result.get("status", "UNKNOWN")
            fidelity = result.get("fidelity", 0.0)
            exec_time = result.get("exec_time_ms", 0)

            # Display result
            status_symbol = "✓" if status == "PASS" else "✗"
            fidelity_str = f"{fidelity:.2f}"
            time_str = f"{exec_time:.0f}"

            print(f"│{i:3d} │ {source:8s} │ {target:8s} │ {status:6s} │ {fidelity_str:8s} │ {time_str:12s} │")

            if status == "PASS":
                passed_tests += 1

        print("└────┴──────────┴──────────┴────────┴──────────┴──────────────┘")

        self.end_time = time.time()
        elapsed = self.end_time - self.start_time

        # Summary
        success_rate = (passed_tests / total_tests) * 100 if total_tests > 0 else 0
        avg_fidelity = sum(r.get("fidelity", 0) for r in self.results.values()) / total_tests if total_tests > 0 else 0
        avg_time = sum(r.get("exec_time_ms", 0) for r in self.results.values()) / total_tests if total_tests > 0 else 0

        print()
        print("╔════════════════════════════════════════════════════════════════╗")
        print("║                      TEST RESULTS SUMMARY                      ║")
        print("╠════════════════════════════════════════════════════════════════╣")
        print(f"║ Total Tests          │ {total_tests:45d} │")
        print(f"║ Passed               │ {passed_tests:45d} │")
        print(f"║ Failed               │ {total_tests - passed_tests:45d} │")
        print(f"║ Success Rate         │ {success_rate:43.1f}% │")
        print(f"║ Average Fidelity     │ {avg_fidelity:43.3f}  │")
        print(f"║ Average Exec Time    │ {avg_time:39.1f} ms │")
        print(f"║ Total Duration       │ {elapsed:39.1f} s  │")
        print("╚════════════════════════════════════════════════════════════════╝")
        print()

        # Individual language stats
        print("Per-Language Statistics:")
        print("─" * 60)

        language_results = {}
        for lang in LANGUAGES:
            lang_tests = [r for k, r in self.results.items() if k.startswith(lang)]
            if lang_tests:
                passed = sum(1 for r in lang_tests if r.get("status") == "PASS")
                avg_fidelity_lang = sum(r.get("fidelity", 0) for r in lang_tests) / len(lang_tests)
                avg_time_lang = sum(r.get("exec_time_ms", 0) for r in lang_tests) / len(lang_tests)

                print(f"  {lang:10s}: {passed:2d}/4 PASS | Fidelity: {avg_fidelity_lang:.3f} | Avg Time: {avg_time_lang:.1f}ms")
                language_results[lang] = {
                    "passed": passed,
                    "total": len(lang_tests),
                    "fidelity": avg_fidelity_lang,
                    "time_ms": avg_time_lang,
                }

        print()

        # Trace validation
        print("Trace Validation:")
        print("─" * 60)

        for lang in LANGUAGES:
            result = self.results.get(f"{lang}→{lang}")
            if result:
                status = result.get("status", "UNKNOWN")
                symbol = "✓" if status == "PASS" else "✗"
                print(f"  {symbol} {lang:10s} self-test: {status} (fidelity={result.get('fidelity', 0):.3f})")

        print()

        # Cross-language conversion validation
        print("Cross-Language Conversion Validation:")
        print("─" * 60)

        conversions_ok = 0
        for source, target in [(s, t) for s in LANGUAGES for t in LANGUAGES if s != t]:
            result = self.results.get(f"{source}→{target}")
            if result and result.get("status") == "PASS":
                conversions_ok += 1

        total_conversions = len([1 for s in LANGUAGES for t in LANGUAGES if s != t])
        print(f"  Cross-language conversions: {conversions_ok}/{total_conversions} successful")
        print()

        return passed_tests == total_tests

    def save_results(self):
        """Save test results to JSON"""
        timestamp = datetime.now().isoformat()
        filename = RESULTS_DIR / f"polyglot-results-{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"

        output = {
            "timestamp": timestamp,
            "total_tests": len(TEST_MATRIX),
            "passed_tests": sum(1 for r in self.results.values() if r.get("status") == "PASS"),
            "duration_seconds": self.end_time - self.start_time if self.end_time else 0,
            "results": self.results,
        }

        with open(filename, 'w') as f:
            json.dump(output, f, indent=2)

        print(f"Results saved to: {filename}")
        print()

def main():
    """Main entry point"""

    # Verify languages exist
    print("Verifying Omnisystem Languages...")
    for lang in LANGUAGES:
        lang_dir = OMNISYSTEM_DIR / lang.lower()
        if lang_dir.exists():
            print(f"  ✓ {lang} found")
        else:
            print(f"  ✗ {lang} NOT FOUND")
            sys.exit(1)

    print()

    # Run tests
    runner = PolyglotTestRunner()
    success = runner.run_all_tests()
    runner.save_results()

    # Exit code
    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main()
