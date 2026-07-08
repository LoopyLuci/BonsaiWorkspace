.PHONY: all run-sylva run-titan run-aether run-axiom test clean

all: run-sylva run-aether

run-sylva:
	python3 sylva/sylva.py sylva/pong.sv

run-titan:
	python3 titan/titan.py titan/pong.ti titan/out.wat
	@echo "✓ Titan compiled to WebAssembly (titan/out.wat)"
	@echo "  To run: wasmtime titan/out.wat"

run-aether:
	python3 aether/pong_runner.py

run-axiom:
	python3 axiom/axiom.py axiom/pong.ax

test:
	python3 sandbox/sandbox.py

clean:
	rm -f titan/out.wat extracted_pong.ti
	find . -name "__pycache__" -type d -exec rm -rf {} + 2>/dev/null || true
	find . -name "*.pyc" -delete

help:
	@echo "Omnisystem Languages – Build Targets"
	@echo ""
	@echo "  make run-sylva    - Run Sylva Pong (Python interpreter)"
	@echo "  make run-titan    - Compile Titan Pong to WebAssembly"
	@echo "  make run-aether   - Run Aether Pong (Actor-based)"
	@echo "  make run-axiom    - Verify Axiom Pong proofs"
	@echo "  make test         - Run all languages in sandbox"
	@echo "  make clean        - Remove generated files"
	@echo "  make help         - Show this help message"
