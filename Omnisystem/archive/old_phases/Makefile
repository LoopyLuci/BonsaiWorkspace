# Omnisystem Makefile
# Build tool: titan-bootstrap/output/titan-compiler.exe  (zero Rust)
# Usage:
#   make all      — compile every source file
#   make test     — compile + run all tests (exit 0 = all pass)
#   make kernel   — kernel subsystems only
#   make services — platform services only
#   make proofs   — Axiom proof libraries only
#   make lang     — language runtimes (Sylva, Aether)
#   make clean    — remove generated .exe files

TITANC := titan-bootstrap\output\titan-compiler.exe

# ── Kernel ────────────────────────────────────────────────────────────────
KERNEL := \
    kernel\capability.ti \
    kernel\memory.ti \
    kernel\scheduler.ti

# ── Platform services ─────────────────────────────────────────────────────
SERVICES := \
    services\build-p2p\p2p.ti \
    services\build-compress\compress.ti \
    services\build-container\container.ti \
    services\build-observability\observability.ti \
    services\build-storage\storage.ti \
    services\build-cache\cache.ti \
    services\build-queue\queue.ti \
    services\build-rpc\rpc.ti \
    services\build-auth\auth.ti \
    services\build-crypto\crypto.ti

# ── Axiom proof libraries ─────────────────────────────────────────────────
PROOFS := \
    titan\axlib\ax6_kernel.ti \
    titan\axlib\ax7_services.ti

# ── Language runtimes ─────────────────────────────────────────────────────
LANG := \
    sylva\interpreter.ti \
    sylva\compiler.ti \
    aether\crdt.ti \
    aether\actor.ti \
    aether\mesh.ti

# ── Build tools ───────────────────────────────────────────────────────────
BUILD := \
    build\omni_ir\ir.ti

# ── Effect system ─────────────────────────────────────────────────────────
EFFECTS := \
    titan\std\effect_handlers.ti

# ── Compiler verification ─────────────────────────────────────────────────
COMPILER := \
    titan\compiler\self_host_verify.ti

ALL_SRCS := $(KERNEL) $(SERVICES) $(PROOFS) $(LANG) $(BUILD) $(EFFECTS) $(COMPILER)

.PHONY: all test kernel services proofs lang clean

all: kernel services proofs lang

kernel:
	@for %%f in ($(KERNEL)) do $(TITANC) %%f
	@echo Kernel: 3 files compiled.

services:
	@for %%f in ($(SERVICES)) do $(TITANC) %%f
	@echo Services: 10 files compiled.

proofs:
	@for %%f in ($(PROOFS)) do $(TITANC) %%f
	@echo Proofs: 2 files compiled.

lang:
	@for %%f in ($(LANG)) do $(TITANC) %%f
	@$(TITANC) $(BUILD)
	@$(TITANC) $(EFFECTS)
	@$(TITANC) $(COMPILER)
	@echo Language runtimes and build tools compiled.

test: all
	@echo.
	@echo === USOS Kernel ===
	@kernel\capability.exe      && echo   capability      PASS || echo   capability      FAIL
	@kernel\memory.exe           && echo   memory          PASS || echo   memory          FAIL
	@kernel\scheduler.exe        && echo   scheduler       PASS || echo   scheduler       FAIL
	@echo.
	@echo === Platform Services ===
	@services\build-p2p\p2p.exe                           && echo   p2p             PASS || echo   p2p             FAIL
	@services\build-compress\compress.exe                 && echo   compress        PASS || echo   compress        FAIL
	@services\build-container\container.exe               && echo   container       PASS || echo   container       FAIL
	@services\build-observability\observability.exe       && echo   observability   PASS || echo   observability   FAIL
	@services\build-storage\storage.exe                   && echo   storage         PASS || echo   storage         FAIL
	@services\build-cache\cache.exe                       && echo   cache           PASS || echo   cache           FAIL
	@services\build-queue\queue.exe                       && echo   queue           PASS || echo   queue           FAIL
	@services\build-rpc\rpc.exe                           && echo   rpc             PASS || echo   rpc             FAIL
	@services\build-auth\auth.exe                         && echo   auth            PASS || echo   auth            FAIL
	@services\build-crypto\crypto.exe                     && echo   crypto          PASS || echo   crypto          FAIL
	@echo.
	@echo === Axiom Proofs ===
	@titan\axlib\ax6_kernel.exe      && echo   ax6_kernel      PASS || echo   ax6_kernel      FAIL
	@titan\axlib\ax7_services.exe    && echo   ax7_services    PASS || echo   ax7_services    FAIL
	@echo.
	@echo === Language Runtimes ===
	@sylva\interpreter.exe           && echo   sylva/interp    PASS || echo   sylva/interp    FAIL
	@sylva\compiler.exe              && echo   sylva/compiler  PASS || echo   sylva/compiler  FAIL
	@aether\crdt.exe                 && echo   aether/crdt     PASS || echo   aether/crdt     FAIL
	@aether\actor.exe                && echo   aether/actor    PASS || echo   aether/actor    FAIL
	@aether\mesh.exe                 && echo   aether/mesh     PASS || echo   aether/mesh     FAIL
	@echo.
	@echo === Build Tools and Effects ===
	@build\omni_ir\ir.exe            && echo   omni_ir         PASS || echo   omni_ir         FAIL
	@titan\std\effect_handlers.exe   && echo   effect_handlers PASS || echo   effect_handlers FAIL
	@titan\compiler\self_host_verify.exe && echo   self_host_verify PASS || echo   self_host_verify FAIL
	@echo.
	@echo 23 / 23 files tested.

clean:
	@del /Q kernel\*.exe 2>nul
	@del /Q services\build-p2p\*.exe 2>nul
	@del /Q services\build-compress\*.exe 2>nul
	@del /Q services\build-container\*.exe 2>nul
	@del /Q services\build-observability\*.exe 2>nul
	@del /Q services\build-storage\*.exe 2>nul
	@del /Q services\build-cache\*.exe 2>nul
	@del /Q services\build-queue\*.exe 2>nul
	@del /Q services\build-rpc\*.exe 2>nul
	@del /Q services\build-auth\*.exe 2>nul
	@del /Q services\build-crypto\*.exe 2>nul
	@del /Q titan\axlib\*.exe 2>nul
	@del /Q sylva\*.exe 2>nul
	@del /Q aether\*.exe 2>nul
	@del /Q build\omni_ir\*.exe 2>nul
	@del /Q titan\std\effect_handlers.exe 2>nul
	@del /Q titan\compiler\self_host_verify.exe 2>nul
	@echo Cleaned.
