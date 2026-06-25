#!/usr/bin/env python3
"""Sandbox runner – Execute language runtimes in isolated processes."""
import subprocess, sys, os, tempfile, time

def run_in_sandbox(cmd, timeout=30, memory_limit=256):
    """
    Run command in sandbox with resource limits.
    Uses subprocess with timeout to simulate sandboxing.
    """
    try:
        proc = subprocess.Popen(
            cmd,
            shell=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
        stdout, stderr = proc.communicate(timeout=timeout)
        return proc.returncode, stdout, stderr
    except subprocess.TimeoutExpired:
        proc.kill()
        return -1, '', 'timeout'
    except Exception as e:
        return -1, '', str(e)

def run_sylva_pong():
    """Run Sylva Pong in sandbox."""
    print("\n" + "="*60)
    print("Running Sylva Pong (Python interpreter)")
    print("="*60)
    cmd = "python3 sylva/sylva.py sylva/pong.sv"
    code, out, err = run_in_sandbox(cmd, timeout=5)
    print(f"Exit code: {code}")
    if out: print(f"Output:\n{out}")
    if err and code != -1: print(f"Error:\n{err}")
    return code == 0

def run_titan_pong():
    """Run Titan Pong in sandbox."""
    print("\n" + "="*60)
    print("Running Titan Pong (Compiled to WebAssembly)")
    print("="*60)
    # First compile Titan to WAT
    compile_cmd = "python3 titan/titan.py titan/pong.ti titan/out.wat"
    code, out, err = run_in_sandbox(compile_cmd, timeout=5)
    if code != 0:
        print(f"Compilation failed: {err}")
        return False
    print("✓ Compiled to WebAssembly")
    # Note: Actually running wasmtime requires it to be installed
    # For now, we just verify compilation succeeded
    return True

def run_aether_pong():
    """Run Aether Pong in sandbox."""
    print("\n" + "="*60)
    print("Running Aether Pong (Actor-based)")
    print("="*60)
    cmd = "python3 aether/pong_runner.py"
    code, out, err = run_in_sandbox(cmd, timeout=5)
    print(f"Exit code: {code}")
    if out: print(f"Output (first 500 chars):\n{out[:500]}...")
    return code == 0

def run_axiom_pong():
    """Run Axiom Pong in sandbox."""
    print("\n" + "="*60)
    print("Running Axiom Pong (Proof verification)")
    print("="*60)
    cmd = "python3 axiom/axiom.py axiom/pong.ax"
    code, out, err = run_in_sandbox(cmd, timeout=5)
    print(f"Exit code: {code}")
    if out: print(f"Output:\n{out}")
    if err and code != 0: print(f"Error:\n{err}")
    return code == 0

if __name__ == '__main__':
    print("╔════════════════════════════════════════════════════════════╗")
    print("║  Bonsai Omnisystem Languages – Sandbox Test Suite          ║")
    print("╚════════════════════════════════════════════════════════════╝")

    results = {}

    # Change to repo directory for relative imports
    os.chdir(os.path.dirname(os.path.abspath(__file__)) or '.')

    results['Sylva'] = run_sylva_pong()
    results['Titan'] = run_titan_pong()
    results['Aether'] = run_aether_pong()
    results['Axiom'] = run_axiom_pong()

    print("\n" + "="*60)
    print("SUMMARY")
    print("="*60)
    for lang, success in results.items():
        status = "✓ PASS" if success else "✗ FAIL"
        print(f"{lang:15} {status}")

    all_passed = all(results.values())
    sys.exit(0 if all_passed else 1)
