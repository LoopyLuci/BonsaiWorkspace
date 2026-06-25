#!/usr/bin/env python3
"""
PixAI tagger worker for BonsAI.

Reads a JSON request from stdin, runs PixAI ONNX inference, writes JSON to stdout.

Install:  pip install onnxruntime numpy pillow huggingface_hub
          # or: pip install dghs-imgutils onnxruntime
"""
import sys
import json
import time
import os

try:
    import numpy as np
    from PIL import Image
    import onnxruntime as ort
except ImportError as e:
    print(json.dumps({"error": f"Missing dependency: {e}. Run: pip install onnxruntime numpy pillow"}))
    sys.exit(1)

# Danbooru tag category IDs
CATEGORY_NAMES = {
    0: "general",
    1: "artist",
    3: "copyright",
    4: "character",
    5: "meta",
    9: "rating",
}

LABEL_FILE = os.path.join(os.path.dirname(__file__), "pixai_labels.txt")


def load_labels(model_dir):
    """Load tag labels from alongside the ONNX model, or fall back to bundled list."""
    candidates = [
        os.path.join(os.path.dirname(model_dir), "selected_tags.csv"),
        os.path.join(os.path.dirname(model_dir), "labels.txt"),
        LABEL_FILE,
    ]
    for path in candidates:
        if os.path.exists(path):
            tags, categories = [], []
            with open(path, "r", encoding="utf-8") as f:
                for i, line in enumerate(f):
                    line = line.strip()
                    if not line or line.startswith("#"):
                        continue
                    if "," in line:
                        parts = line.split(",")
                        tags.append(parts[0].strip())
                        try:
                            categories.append(int(parts[1].strip()) if len(parts) > 1 else 0)
                        except ValueError:
                            categories.append(0)
                    else:
                        tags.append(line)
                        categories.append(0)
            return tags, categories
    return None, None


def preprocess(image_path, target_size=448):
    img = Image.open(image_path).convert("RGB")
    img = img.resize((target_size, target_size), Image.BICUBIC)
    arr = np.array(img, dtype=np.float32) / 255.0
    # ImageNet normalisation
    mean = np.array([0.485, 0.456, 0.406], dtype=np.float32)
    std  = np.array([0.229, 0.224, 0.225], dtype=np.float32)
    arr = (arr - mean) / std
    # CHW → NCHW
    arr = arr.transpose(2, 0, 1)[np.newaxis, :]
    return arr


def run_tagger(req):
    image_path  = req["image_path"]
    model_path  = req["model_path"]
    confidence  = float(req.get("confidence", 0.35))
    max_tags    = int(req.get("max_tags", 50))

    if not os.path.exists(image_path):
        return {"error": f"Image not found: {image_path}"}
    if not os.path.exists(model_path):
        return {"error": f"Model not found: {model_path}"}

    t0 = time.time()

    tags, categories = load_labels(model_path)
    if tags is None:
        return {"error": "Could not load tag labels. Place selected_tags.csv alongside the ONNX model."}

    sess = ort.InferenceSession(model_path, providers=["CPUExecutionProvider"])
    input_name = sess.get_inputs()[0].name

    inp = preprocess(image_path)
    outputs = sess.run(None, {input_name: inp})
    scores = outputs[0][0]  # shape: (num_tags,)

    entries = []
    for i, (score, tag) in enumerate(zip(scores, tags)):
        if score >= confidence:
            cat_id = categories[i] if i < len(categories) else 0
            entries.append({
                "tag":        tag,
                "confidence": float(score),
                "category":   CATEGORY_NAMES.get(cat_id, "general"),
            })

    entries.sort(key=lambda e: e["confidence"], reverse=True)
    entries = entries[:max_tags]

    return {
        "tags":     entries,
        "elapsed_ms": int((time.time() - t0) * 1000),
    }


def main():
    try:
        raw = sys.stdin.read()
        req = json.loads(raw)
    except Exception as e:
        print(json.dumps({"error": f"JSON parse error: {e}"}))
        sys.exit(1)

    result = run_tagger(req)
    print(json.dumps(result))


if __name__ == "__main__":
    main()
