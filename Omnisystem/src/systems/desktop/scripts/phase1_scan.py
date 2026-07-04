#!/usr/bin/env python3
"""Phase 1: Model Scanner - Scan D:\Models\general and build inventory"""

import os
import json
from pathlib import Path
from datetime import datetime

def main():
    model_dir = r"D:\Models\general"
    output_file = r"D:\Models\extracted_knowledge\model_inventory.json"
    extensions = ['.gguf', '.safetensors', '.bin', '.pt', '.pth', '.onnx', '.bkp']

    models = []

    print("🧠 PHASE 1: Model Scanning & Inventory")
    print("=" * 60)
    print(f"Scanning directory: {model_dir}")

    if not os.path.exists(model_dir):
        print(f"\n⚠️  Directory does not exist: {model_dir}")
        print("   Creating empty inventory for demonstration...")
    else:
        print("Looking for model files...\n")

        for root, dirs, files in os.walk(model_dir):
            for file in files:
                if any(file.lower().endswith(ext) for ext in extensions):
                    path = os.path.join(root, file)
                    try:
                        size = os.path.getsize(path)

                        # Infer parameters from filename
                        fname = file.lower()
                        param_count = None
                        if '1b' in fname or '1.1b' in fname:
                            param_count = 1_100_000_000
                        elif '3b' in fname:
                            param_count = 3_000_000_000
                        elif '7b' in fname:
                            param_count = 7_000_000_000
                        elif '13b' in fname:
                            param_count = 13_000_000_000
                        elif '34b' in fname:
                            param_count = 34_000_000_000
                        elif '70b' in fname:
                            param_count = 70_000_000_000
                        elif '180b' in fname:
                            param_count = 180_000_000_000

                        # Detect quantization
                        quant = None
                        if 'q4_0' in fname:
                            quant = 'Q4_0'
                        elif 'q5_0' in fname:
                            quant = 'Q5_0'
                        elif 'q8_0' in fname:
                            quant = 'Q8_0'
                        elif 'f16' in fname or 'fp16' in fname:
                            quant = 'fp16'
                        elif 'f32' in fname or 'fp32' in fname:
                            quant = 'fp32'

                        # Format detection
                        fmt = 'unknown'
                        if file.lower().endswith('.gguf'):
                            fmt = 'gguf'
                        elif file.lower().endswith('.safetensors'):
                            fmt = 'safetensors'
                        elif file.lower().endswith(('.bin', '.pt', '.pth')):
                            fmt = 'pytorch'
                        elif file.lower().endswith('.onnx'):
                            fmt = 'onnx'
                        elif file.lower().endswith('.bkp'):
                            fmt = 'bonsai_package'

                        models.append({
                            'id': f'model_{len(models)+1:03d}',
                            'filename': file,
                            'path': path,
                            'format': fmt,
                            'size_bytes': size,
                            'parameter_count': param_count,
                            'quantization': quant,
                            'context_length': 2048,
                            'architecture': None,
                            'discovered_at': datetime.now().isoformat()
                        })
                    except Exception as e:
                        print(f"  ⚠️  Error processing {file}: {e}")

    # Sort by size ascending (smallest first)
    models.sort(key=lambda m: m['size_bytes'])

    # Write inventory
    os.makedirs(os.path.dirname(output_file), exist_ok=True)
    with open(output_file, 'w') as f:
        json.dump(models, f, indent=2)

    # Summary
    print("\n" + "=" * 60)
    if models:
        total_size = sum(m['size_bytes'] for m in models)
        print(f"✅ Found {len(models)} model(s)")
        print(f"   Total size: {total_size / 1e9:.2f} GB\n")
        print("📋 Model Inventory (sorted by size, smallest first):")
        for i, m in enumerate(models):
            size_gb = m['size_bytes'] / 1e9
            params = f"{m['parameter_count']/1e9:.1f}B" if m['parameter_count'] else "unknown"
            print(f"   {i+1:3}. {m['filename']:50} | {size_gb:8.2f} GB | {params:>6} | {m['format']:12} | {m['quantization'] or 'unquant':5}")
    else:
        print("ℹ️  No models found in directory")
        print("   The pipeline is ready to extract from new models as they're added\n")

    print(f"\n📁 Inventory saved to: {output_file}")
    return 0

if __name__ == "__main__":
    exit(main())
