#!/usr/bin/env python3
"""
Sulphur-2-base video generation worker for BonsAI.

Reads JSON from stdin, generates a video, writes JSON to stdout.
The video is saved to the requested path (MP4 via imageio/ffmpeg).

Install:  pip install diffusers transformers accelerate torch imageio imageio-ffmpeg
"""
import sys
import json
import time
import os

try:
    import torch
except ImportError as e:
    print(json.dumps({"error": f"Missing dependency: {e}. Run: pip install torch"}))
    sys.exit(1)


def generate_video(req):
    prompt     = req.get("prompt", "")
    model_path = req.get("model_path", "")
    save_path  = req.get("save_path", "output.mp4")
    frames     = int(req.get("frames", 24))
    fps        = int(req.get("fps", 8))

    if not os.path.exists(model_path) and not os.path.isdir(model_path):
        return {"error": f"Model not found: {model_path}"}

    t0 = time.time()

    try:
        from diffusers import DiffusionPipeline
    except ImportError:
        return {"error": "diffusers not installed. Run: pip install diffusers"}

    device = "cuda" if torch.cuda.is_available() else "cpu"
    dtype  = torch.float16 if device == "cuda" else torch.float32

    # Load LTX-Video or compatible pipeline bundled in the model directory
    pipe = DiffusionPipeline.from_pretrained(
        model_path,
        torch_dtype=dtype,
        use_safetensors=True,
    ).to(device)

    # Generate frames
    output = pipe(
        prompt=prompt,
        num_frames=frames,
        guidance_scale=7.5,
        num_inference_steps=25,
        output_type="pil",
    )
    video_frames = output.frames[0] if hasattr(output, "frames") else output.images

    # Save as MP4
    try:
        import imageio
        writer = imageio.get_writer(save_path, fps=fps, codec="libx264", quality=8)
        import numpy as np
        for frame in video_frames:
            writer.append_data(np.array(frame))
        writer.close()
    except ImportError:
        # Fallback: save individual PNG frames
        save_dir = save_path.replace(".mp4", "_frames")
        os.makedirs(save_dir, exist_ok=True)
        for i, frame in enumerate(video_frames):
            frame.save(os.path.join(save_dir, f"frame_{i:04d}.png"))
        save_path = save_dir

    elapsed = time.time() - t0
    actual_frames = len(video_frames)

    return {
        "video_path":  save_path,
        "frames":      actual_frames,
        "duration_s":  actual_frames / fps,
        "fps":         fps,
        "elapsed_ms":  int(elapsed * 1000),
    }


def main():
    try:
        raw = sys.stdin.read()
        req = json.loads(raw)
    except Exception as e:
        print(json.dumps({"error": f"JSON parse error: {e}"}))
        sys.exit(1)

    try:
        result = generate_video(req)
        print(json.dumps(result))
    except Exception as e:
        import traceback
        print(json.dumps({"error": str(e), "traceback": traceback.format_exc()}))
        sys.exit(1)


if __name__ == "__main__":
    main()
