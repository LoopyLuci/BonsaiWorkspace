#!/usr/bin/env python3
"""Bonsai Stable Diffusion generator — AMD DirectML backend.

Supports:
  - HuggingFace model directories (safetensors layout)
  - Single .safetensors or .ckpt files (from_single_file)
  - GGUF quantized models (from_single_file, diffusers >= 0.31)

Called by image_generation.rs with arguments:
    --model <path_or_hf_id>   path to model dir, .safetensors, .ckpt, or .gguf
    --prompt <text>
    --output <path.png>
    --width  <int>
    --height <int>
    --steps  <int>
    --guidance <float>
    [--negative_prompt <text>]
"""

import argparse
import sys
import os


def parse_args():
    p = argparse.ArgumentParser()
    p.add_argument("--model",           required=True)
    p.add_argument("--prompt",          required=True)
    p.add_argument("--output",          required=True)
    p.add_argument("--width",           type=int,   default=512)
    p.add_argument("--height",          type=int,   default=512)
    p.add_argument("--steps",           type=int,   default=20)
    p.add_argument("--guidance",        type=float, default=7.5)
    p.add_argument("--negative_prompt", default="")
    return p.parse_args()


def load_pipeline(model_path: str, device):
    import torch
    from diffusers import (
        StableDiffusionPipeline,
        StableDiffusionXLPipeline,
        DPMSolverMultistepScheduler,
    )

    ext    = os.path.splitext(model_path)[1].lower()
    is_file = os.path.isfile(model_path)
    is_xl   = "xl" in os.path.basename(model_path).lower()

    print(f"[sd] Loading: {model_path}", file=sys.stderr)
    fmt = "GGUF" if ext == ".gguf" else "safetensors" if ext == ".safetensors" else "ckpt" if ext == ".ckpt" else "directory/HF"
    print(f"[sd] Format : {fmt}  XL: {is_xl}", file=sys.stderr)

    pipe_class = StableDiffusionXLPipeline if is_xl else StableDiffusionPipeline

    if is_file:
        pipe = pipe_class.from_single_file(
            model_path,
            torch_dtype=torch.float16,
            use_safetensors=(ext == ".safetensors"),
        )
    else:
        pipe = pipe_class.from_pretrained(
            model_path,
            torch_dtype=torch.float16,
            safety_checker=None,
            requires_safety_checker=False,
        )

    pipe.scheduler = DPMSolverMultistepScheduler.from_config(pipe.scheduler.config)
    return pipe.to(device), is_xl


def main():
    args = parse_args()

    try:
        import torch_directml
        import torch
    except ImportError as e:
        print(f"ERROR: {e} — run install-sd.ps1 to set up the SD environment.", file=sys.stderr)
        sys.exit(1)

    device = torch_directml.device()
    print(f"[sd] Device: {device}", file=sys.stderr)

    pipe, is_xl = load_pipeline(args.model, device)

    w, h = args.width, args.height
    if is_xl and w <= 512 and h <= 512:
        w, h = 1024, 1024
        print(f"[sd] SDXL detected — upgrading resolution to {w}x{h}", file=sys.stderr)

    print(f"[sd] Generating ({w}x{h}, {args.steps} steps)…", file=sys.stderr)

    kwargs = dict(
        prompt=args.prompt,
        width=w, height=h,
        num_inference_steps=args.steps,
        guidance_scale=args.guidance,
        num_images_per_prompt=1,
    )
    if args.negative_prompt:
        kwargs["negative_prompt"] = args.negative_prompt

    image = pipe(**kwargs).images[0]
    os.makedirs(os.path.dirname(os.path.abspath(args.output)), exist_ok=True)
    image.save(args.output, "PNG")
    print(f"[sd] Saved: {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
