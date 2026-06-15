#!/usr/bin/env python3
import os
import re
from pathlib import Path
from collections import defaultdict

workspace_root = Path(".")
crates_dir = workspace_root / "crates"

systems = defaultdict(list)
total_loc = 0
crate_count = 0

# Scan all crates
for crate_path in sorted(crates_dir.iterdir()):
    if not crate_path.is_dir():
        continue
    
    cargo_toml = crate_path / "Cargo.toml"
    if not cargo_toml.exists():
        continue
    
    crate_count += 1
    crate_name = crate_path.name
    
    # Read Cargo.toml
    with open(cargo_toml) as f:
        content = f.read()
        
    # Extract description
    desc_match = re.search(r'description\s*=\s*"([^"]*)"', content)
    description = desc_match.group(1) if desc_match else "No description"
    
    # Count LOC
    src_dir = crate_path / "src"
    loc = 0
    if src_dir.exists():
        for rs_file in src_dir.rglob("*.rs"):
            try:
                with open(rs_file) as f:
                    loc += len(f.readlines())
            except:
                pass
    
    total_loc += loc
    
    # Categorize by prefix
    if crate_name.startswith("omnisystem"):
        category = "CORE: Omnisystem"
    elif crate_name.startswith("buddy"):
        category = "SYSTEMS: Buddy"
    elif crate_name.startswith("omnibot"):
        category = "SYSTEMS: OmniBot"
    elif crate_name.startswith("usee"):
        category = "SYSTEMS: USEE"
    elif crate_name.startswith("transfer"):
        category = "SYSTEMS: TransferDaemon"
    elif crate_name.startswith("remote"):
        category = "SYSTEMS: Remote Access"
    elif crate_name.startswith("bonsai"):
        category = "CORE: Bonsai"
    elif crate_name.startswith("bmn"):
        category = "CORE: BMN"
    elif crate_name.startswith("app"):
        category = "CORE: App Management"
    elif crate_name.startswith("api"):
        category = "CORE: API"
    elif crate_name.startswith("ahf"):
        category = "SYSTEMS: AHF"
    elif crate_name.startswith("aion"):
        category = "SYSTEMS: Aion"
    elif crate_name.startswith("launcher"):
        category = "SYSTEMS: Launcher"
    elif crate_name.startswith("backend"):
        category = "CORE: Backend"
    elif crate_name.startswith("ui"):
        category = "CORE: UI"
    elif crate_name.startswith("p2p"):
        category = "CORE: P2P"
    elif crate_name.startswith("network"):
        category = "SYSTEMS: Network"
    elif crate_name.startswith("iot"):
        category = "SYSTEMS: IoT"
    elif crate_name.startswith("aether"):
        category = "SYSTEMS: Aether"
    else:
        category = "OTHER: Utilities"
    
    systems[category].append({
        "name": crate_name,
        "description": description,
        "loc": loc
    })

# Print summary
print("=" * 100)
print("OMNISYSTEM COMPREHENSIVE CODEBASE ANALYSIS")
print("=" * 100)
print(f"\nTotal Crates: {crate_count}")
print(f"Total Lines of Code: {total_loc:,}")
print()

# Print by category
for category in sorted(systems.keys()):
    crates = systems[category]
    cat_loc = sum(c['loc'] for c in crates)
    print(f"\n{category}")
    print(f"  Crates: {len(crates)} | LOC: {cat_loc:,}")
    for crate in sorted(crates, key=lambda x: x['name']):
        print(f"    • {crate['name']:<40} ({crate['loc']:>6,} LOC) - {crate['description'][:60]}")

