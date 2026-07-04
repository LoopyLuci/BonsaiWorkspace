#!/usr/bin/env python3
"""
Qwen-Image-Edit worker for BonsAI.

Supports three operations:
  - edit_image       : edit an existing image with a text prompt
  - generate_image   : generate a new image from a text prompt
  - generate_multiview : generate multiple camera-angle views

Reads JSON from stdin, writes JSON to stdout.

Install:  pip install diffusers transformers accelerate torch pillow
"""
import sys
import json
import base64
import time
import os
import io

try:
    from PIL import Image
    import torch
except ImportError as e:
    print(json.dumps({"error": f"Missing dependency: {e}. Run: pip install diffusers transformers accelerate torch pillow"}))
    sys.exit(1)


def encode_image(img: Image.Image) -> str:
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return base64.b64encode(buf.getvalue()).decode("ascii")


def load_pipeline(model_path: str, op: str):
    try:
        from diffusers import AutoPipelineForImage2Image, AutoPipelineForText2Image
    except ImportError:
        raise RuntimeError("diffusers not installed. Run: pip install diffusers")

    device = "cuda" if torch.cuda.is_available() else "cpu"
    dtype  = torch.float16 if device == "cuda" else torch.float32

    if op in ("edit_image",):
        pipe = AutoPipelineForImage2Image.from_pretrained(
            model_path, torch_dtype=dtype, use_safetensors=True
        ).to(device)
    else:
        pipe = AutoPipelineForText2Image.from_pretrained(
            model_path, torch_dtype=dtype, use_safetensors=True
        ).to(device)
    return pipe


def op_edit_image(req):
    image_path = req["image_path"]
    prompt     = req["prompt"]
    model_path = req["model_path"]
    save_path  = req.get("save_path")

    if not os.path.exists(image_path):
        return {"error": f"Image not found: {image_path}"}

    t0   = time.time()
    pipe = load_pipeline(model_path, "edit_image")
    img  = Image.open(image_path).convert("RGB")

    result_img = pipe(prompt=prompt, image=img, strength=0.75,
                      guidance_scale=7.5, num_inference_steps=20).images[0]

    if save_path:
        result_img.save(save_path)

    return {
        "image_b64":  encode_image(result_img),
        "saved_path": save_path,
        "elapsed_ms": int((time.time() - t0) * 1000),
    }


def op_generate_image(req):
    prompt     = req["prompt"]
    model_path = req["model_path"]
    save_path  = req.get("save_path")

    t0   = time.time()
    pipe = load_pipeline(model_path, "generate_image")

    result_img = pipe(prompt=prompt, guidance_scale=7.5,
                      num_inference_steps=20).images[0]

    if save_path:
        result_img.save(save_path)

    return {
        "image_b64":  encode_image(result_img),
        "saved_path": save_path,
        "elapsed_ms": int((time.time() - t0) * 1000),
    }


ANGLE_PROMPTS = {
    "front":      "front view, facing camera",
    "back":       "back view, rear facing",
    "side_left":  "left side view, profile",
    "side_right": "right side view, profile",
    "top":        "top-down bird's eye view",
    "isometric":  "isometric 3/4 view, diagonal",
}


def op_generate_multiview(req):
    image_path = req["image_path"]
    angles     = req.get("angles", list(ANGLE_PROMPTS.keys()))
    model_path = req["model_path"]

    if not os.path.exists(image_path):
        return {"error": f"Image not found: {image_path}"}

    t0   = time.time()
    pipe = load_pipeline(model_path, "edit_image")
    src  = Image.open(image_path).convert("RGB")

    images_b64   = []
    saved_paths  = []

    for angle in angles:
        angle_prompt = ANGLE_PROMPTS.get(angle, f"{angle} view")
        result_img   = pipe(
            prompt=angle_prompt,
            image=src,
            strength=0.6,
            guidance_scale=7.5,
            num_inference_steps=15,
        ).images[0]
        images_b64.append(encode_image(result_img))
        saved_paths.append(None)

    return {
        "images_b64":  images_b64,
        "angles":      angles,
        "saved_paths": saved_paths,
        "elapsed_ms":  int((time.time() - t0) * 1000),
    }


OP_MAP = {
    "edit_image":        op_edit_image,
    "generate_image":    op_generate_image,
    "generate_multiview": op_generate_multiview,
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
