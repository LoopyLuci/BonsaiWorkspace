#!/usr/bin/env python3
"""
TRELLIS.2-4B 3D asset generation worker for BonsAI.

Supports:
  - image_to_3d : convert an image to a 3D mesh (GLB/OBJ)
  - text_to_3d  : generate a 3D mesh from a text prompt

Reads JSON from stdin, writes JSON to stdout.

Install:  pip install torch torchvision transformers accelerate trimesh pillow
          # TRELLIS also needs: pip install open3d  (for mesh export)
"""
import sys
import json
import time
import os
import tempfile

try:
    import torch
    from PIL import Image
except ImportError as e:
    print(json.dumps({"error": f"Missing dependency: {e}. Run: pip install torch pillow trimesh"}))
    sys.exit(1)


def get_mesh_stats(mesh_path: str):
    try:
        import trimesh
        mesh = trimesh.load(mesh_path)
        if hasattr(mesh, "vertices") and hasattr(mesh, "faces"):
            return int(len(mesh.vertices)), int(len(mesh.faces))
        # Scene with multiple meshes
        if hasattr(mesh, "geometry"):
            verts = sum(len(g.vertices) for g in mesh.geometry.values())
            faces = sum(len(g.faces) for g in mesh.geometry.values())
            return int(verts), int(faces)
    except Exception:
        pass
    return 0, 0


def load_trellis(model_path: str):
    try:
        from transformers import AutoModelForCausalLM, AutoProcessor
    except ImportError:
        raise RuntimeError("transformers not installed")

    device = "cuda" if torch.cuda.is_available() else "cpu"
    dtype  = torch.float16 if device == "cuda" else torch.float32

    processor = AutoProcessor.from_pretrained(model_path, trust_remote_code=True)
    model     = AutoModelForCausalLM.from_pretrained(
        model_path,
        torch_dtype=dtype,
        trust_remote_code=True,
    ).to(device)
    return model, processor, device


def op_image_to_3d(req):
    image_path = req["image_path"]
    model_path = req["model_path"]
    save_path  = req.get("save_path", "output.glb")
    fmt        = req.get("format", "glb").lower()

    if not os.path.exists(image_path):
        return {"error": f"Image not found: {image_path}"}

    t0 = time.time()
    model, processor, device = load_trellis(model_path)

    img    = Image.open(image_path).convert("RGB")
    inputs = processor(images=img, return_tensors="pt").to(device)

    with torch.no_grad():
        outputs = model.generate_3d(**inputs, num_inference_steps=50)

    # Export mesh
    os.makedirs(os.path.dirname(os.path.abspath(save_path)), exist_ok=True)
    outputs.export(save_path)

    verts, faces = get_mesh_stats(save_path)
    return {
        "mesh_path":    save_path,
        "format":       fmt,
        "vertex_count": verts,
        "face_count":   faces,
        "elapsed_ms":   int((time.time() - t0) * 1000),
    }


def op_text_to_3d(req):
    prompt     = req["prompt"]
    model_path = req["model_path"]
    save_path  = req.get("save_path", "output.glb")
    fmt        = req.get("format", "glb").lower()

    t0 = time.time()
    model, processor, device = load_trellis(model_path)

    inputs = processor(text=prompt, return_tensors="pt").to(device)

    with torch.no_grad():
        outputs = model.generate_3d(**inputs, num_inference_steps=50)

    os.makedirs(os.path.dirname(os.path.abspath(save_path)), exist_ok=True)
    outputs.export(save_path)

    verts, faces = get_mesh_stats(save_path)
    return {
        "mesh_path":    save_path,
        "format":       fmt,
        "vertex_count": verts,
        "face_count":   faces,
        "elapsed_ms":   int((time.time() - t0) * 1000),
    }


OP_MAP = {
    "image_to_3d": op_image_to_3d,
    "text_to_3d":  op_text_to_3d,
}


def main():
    try:
        raw = sys.stdin.read()
        req = json.loads(raw)
    except Exception as e:
        print(json.dumps({"error": f"JSON parse error: {e}"}))
        sys.exit(1)

    op = req.get("op", "")
    handler = OP_MAP.get(op)
    if handler is None:
        print(json.dumps({"error": f"Unknown op: {op}. Available: {list(OP_MAP)}"}))
        sys.exit(1)

    try:
        result = handler(req)
        print(json.dumps(result))
    except Exception as e:
        import traceback
        print(json.dumps({"error": str(e), "traceback": traceback.format_exc()}))
        sys.exit(1)


if __name__ == "__main__":
    main()
