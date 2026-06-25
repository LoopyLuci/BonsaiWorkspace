#!/usr/bin/env python3
"""
BonsAI Model Selector
Unified interface for discovering, listing, and loading all available models.
"""

import json
import os
from pathlib import Path
from typing import List, Dict, Optional, Union
from dataclasses import dataclass, asdict
import sys


@dataclass
class ModelInfo:
    """Model metadata and location information."""
    name: str
    version: str
    model_type: str
    location: str
    size_mb: Optional[int] = None
    framework: Optional[str] = None
    status: str = "ready"
    capabilities: List[str] = None

    def __post_init__(self):
        if self.capabilities is None:
            self.capabilities = []


class ModelSelector:
    """
    Central interface for accessing all BonsAI, Octopus, and Poe models.

    Provides:
    - Model discovery and listing
    - Model metadata retrieval
    - Hardware-aware allocation
    - BUEB integration
    """

    def __init__(self, models_dir: str = "."):
        """Initialize model selector."""
        self.models_dir = Path(models_dir)
        self.manifest_path = self.models_dir / "MODEL_MANIFEST.json"
        self.manifest = self._load_manifest()

    def _load_manifest(self) -> Dict:
        """Load model manifest from JSON."""
        if self.manifest_path.exists():
            with open(self.manifest_path, 'r') as f:
                return json.load(f)
        return {}

    def list_all_models(self) -> List[ModelInfo]:
        """List all available models."""
        models = []

        if not self.manifest:
            return models

        # Main models
        for model_id, model_data in self.manifest.get("models", {}).items():
            models.append(ModelInfo(
                name=model_data.get("name", model_id),
                version=model_data.get("version", "unknown"),
                model_type=model_data.get("type", "unknown"),
                location=model_data.get("location", ""),
                size_mb=model_data.get("size_mb"),
                framework=model_data.get("framework"),
                status=model_data.get("status", "ready"),
                capabilities=model_data.get("capabilities", [])
            ))

        # Quantized models
        quantized = self.manifest.get("quantized_models", {})
        if quantized:
            for model_name in quantized.get("models", []):
                models.append(ModelInfo(
                    name=f"Quantized: {model_name}",
                    version="1.0",
                    model_type="gguf-quantized",
                    location=f"{quantized.get('location', 'models/quantized/')}{model_name}",
                    size_mb=2,  # Average size
                    framework="gguf",
                    status="ready",
                    capabilities=["inference", "cpu-optimized"]
                ))

        return models

    def get_model(self, model_id: str) -> Optional[ModelInfo]:
        """Get specific model by ID."""
        for model in self.list_all_models():
            if model_id.lower() in model.name.lower():
                return model
        return None

    def list_octopus_models(self) -> List[ModelInfo]:
        """List all Octopus AI models."""
        return [m for m in self.list_all_models() if "octopus" in m.name.lower()]

    def list_poe_models(self) -> List[ModelInfo]:
        """List all Poe AI models."""
        return [m for m in self.list_all_models() if "poe" in m.name.lower()]

    def list_bonsai_models(self) -> List[ModelInfo]:
        """List all BonsAI system models."""
        return [m for m in self.list_all_models() if "bonsai" in m.name.lower()]

    def get_model_path(self, model_id: str) -> Optional[Path]:
        """Get full path to model files."""
        model = self.get_model(model_id)
        if model:
            return self.models_dir / model.location
        return None

    def get_model_config(self, model_id: str) -> Optional[Dict]:
        """Get model configuration."""
        if not self.manifest:
            return None

        for model_key, model_data in self.manifest.get("models", {}).items():
            if model_id.lower() in model_key.lower():
                return model_data

        return None

    def print_summary(self):
        """Print summary of all available models."""
        print("\n" + "="*70)
        print("BonsAI Model Selector - Available Models")
        print("="*70 + "\n")

        models = self.list_all_models()

        if not models:
            print("No models found in manifest.")
            return

        # Group by type
        by_type = {}
        for model in models:
            model_type = model.model_type
            if model_type not in by_type:
                by_type[model_type] = []
            by_type[model_type].append(model)

        for model_type, type_models in sorted(by_type.items()):
            print(f"\n{model_type.upper()}")
            print("-" * 70)
            for model in type_models:
                size_str = f" ({model.size_mb}MB)" if model.size_mb else ""
                print(f"  • {model.name} v{model.version}{size_str}")
                print(f"    Status: {model.status}")
                print(f"    Location: {model.location}")
                if model.capabilities:
                    print(f"    Capabilities: {', '.join(model.capabilities)}")
                print()

    def get_model_for_task(self, task_type: str) -> Optional[ModelInfo]:
        """Get recommended model for a specific task."""
        task_mapping = {
            "server-management": "octopus-ai",
            "qa": "octopus-ai",
            "system-admin": "octopus-ai",
            "expression": "poe-ai",
            "personality": "poe-ai",
            "registry": "bonsai-foundation",
            "inference": "octopus-ai",
        }

        model_id = task_mapping.get(task_type.lower())
        if model_id:
            return self.get_model(model_id)

        return None


def main():
    """Command-line interface for model selector."""
    import argparse

    parser = argparse.ArgumentParser(
        description="BonsAI Model Selector - Manage and discover models"
    )
    parser.add_argument(
        "command",
        nargs="?",
        choices=["list", "info", "path", "summary"],
        default="list",
        help="Command to execute"
    )
    parser.add_argument(
        "--model",
        help="Model ID or name to query"
    )
    parser.add_argument(
        "--type",
        choices=["octopus", "poe", "bonsai", "quantized"],
        help="Filter by model type"
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output in JSON format"
    )

    args = parser.parse_args()

    # Initialize selector
    models_dir = Path(__file__).parent
    selector = ModelSelector(models_dir)

    if args.command == "list" or args.command == "summary":
        models = selector.list_all_models()

        if args.type == "octopus":
            models = selector.list_octopus_models()
        elif args.type == "poe":
            models = selector.list_poe_models()
        elif args.type == "bonsai":
            models = selector.list_bonsai_models()

        if args.json:
            print(json.dumps([asdict(m) for m in models], indent=2))
        else:
            selector.print_summary()

    elif args.command == "info":
        if not args.model:
            print("Error: --model required for 'info' command")
            sys.exit(1)

        config = selector.get_model_config(args.model)
        if config:
            print(json.dumps(config, indent=2))
        else:
            print(f"Model '{args.model}' not found")
            sys.exit(1)

    elif args.command == "path":
        if not args.model:
            print("Error: --model required for 'path' command")
            sys.exit(1)

        path = selector.get_model_path(args.model)
        if path:
            print(str(path))
        else:
            print(f"Model '{args.model}' not found")
            sys.exit(1)


if __name__ == "__main__":
    main()
