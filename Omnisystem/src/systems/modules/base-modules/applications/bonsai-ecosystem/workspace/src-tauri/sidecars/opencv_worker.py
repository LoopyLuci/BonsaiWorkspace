#!/usr/bin/env python3
"""
OpenCV 4.12 worker for BonsAI.

Reads a JSON operation descriptor from stdin, executes the requested
OpenCV operation, and writes a JSON result to stdout.  All image outputs
are base64-encoded PNG to keep the interface simple.

Install:  pip install opencv-python numpy
"""
import sys
import json
import base64
import time
import os

try:
    import cv2
    import numpy as np
except ImportError:
    print(json.dumps({"error": "cv2 not found. Run: pip install opencv-python"}))
    sys.exit(1)


def encode_image(img):
    """Encode an OpenCV image (numpy array) to base64 PNG."""
    ok, buf = cv2.imencode(".png", img)
    if not ok:
        return None
    return base64.b64encode(buf.tobytes()).decode("ascii")


def make_result(img, metadata=None, save_path=None, t0=None):
    h, w = img.shape[:2]
    ch = img.shape[2] if img.ndim == 3 else 1
    if save_path:
        cv2.imwrite(save_path, img)
    return {
        "image_b64":  encode_image(img),
        "saved_path": save_path,
        "width":      w,
        "height":     h,
        "channels":   ch,
        "metadata":   metadata or {},
        "elapsed_ms": int((time.time() - t0) * 1000) if t0 else 0,
    }


# ── Operations ────────────────────────────────────────────────────────────────

def op_convert_color(req, img, t0):
    space = req.get("color_space", "grayscale").lower()
    codes = {
        "grayscale": cv2.COLOR_BGR2GRAY,
        "gray":      cv2.COLOR_BGR2GRAY,
        "hsv":       cv2.COLOR_BGR2HSV,
        "lab":       cv2.COLOR_BGR2LAB,
        "yuv":       cv2.COLOR_BGR2YUV,
        "rgb":       cv2.COLOR_BGR2RGB,
        "bgr":       None,
    }
    code = codes.get(space)
    out = cv2.cvtColor(img, code) if code is not None else img.copy()
    return make_result(out, {"color_space": space}, req.get("save_path"), t0)


def op_resize_image(req, img, t0):
    w = int(req["width"])
    h = int(req["height"])
    interp_map = {
        "nearest": cv2.INTER_NEAREST,
        "linear":  cv2.INTER_LINEAR,
        "cubic":   cv2.INTER_CUBIC,
        "lanczos": cv2.INTER_LANCZOS4,
        "area":    cv2.INTER_AREA,
    }
    interp = interp_map.get(req.get("interpolation", "linear"), cv2.INTER_LINEAR)
    out = cv2.resize(img, (w, h), interpolation=interp)
    return make_result(out, {"original_width": img.shape[1], "original_height": img.shape[0]},
                       req.get("save_path"), t0)


def op_blur_image(req, img, t0):
    blur_type   = req.get("blur_type", "gaussian").lower()
    kernel_size = int(req.get("kernel_size", 5))
    if kernel_size % 2 == 0:
        kernel_size += 1
    if blur_type == "gaussian":
        out = cv2.GaussianBlur(img, (kernel_size, kernel_size), 0)
    elif blur_type == "median":
        out = cv2.medianBlur(img, kernel_size)
    elif blur_type == "bilateral":
        out = cv2.bilateralFilter(img, kernel_size, 75, 75)
    else:
        out = cv2.GaussianBlur(img, (kernel_size, kernel_size), 0)
    return make_result(out, {"blur_type": blur_type, "kernel_size": kernel_size},
                       req.get("save_path"), t0)


def op_detect_edges(req, img, t0):
    gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY) if img.ndim == 3 else img
    t1   = float(req.get("threshold1", 100))
    t2   = float(req.get("threshold2", 200))
    ap   = int(req.get("aperture_size", 3))
    edges = cv2.Canny(gray, t1, t2, apertureSize=ap)
    return make_result(edges, {"threshold1": t1, "threshold2": t2},
                       req.get("save_path"), t0)


def op_find_contours(req, img, t0):
    gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY) if img.ndim == 3 else img
    _, binary = cv2.threshold(gray, 127, 255, cv2.THRESH_BINARY)
    mode_map = {
        "external": cv2.RETR_EXTERNAL,
        "list":     cv2.RETR_LIST,
        "tree":     cv2.RETR_TREE,
    }
    mode = mode_map.get(req.get("mode", "external"), cv2.RETR_EXTERNAL)
    contours, _ = cv2.findContours(binary, mode, cv2.CHAIN_APPROX_SIMPLE)
    areas = [cv2.contourArea(c) for c in contours]
    out = img.copy() if req.get("draw_overlay", True) else img
    if req.get("draw_overlay", True) and img.ndim == 3:
        cv2.drawContours(out, contours, -1, (0, 255, 0), 2)
    meta = {
        "contour_count": len(contours),
        "areas":         sorted(areas, reverse=True)[:20],
        "total_area":    sum(areas),
    }
    return make_result(out, meta, req.get("save_path"), t0)


def op_detect_faces(req, img, t0):
    gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY) if img.ndim == 3 else img
    # Use built-in Haar cascade (ships with opencv-python)
    cascade_path = cv2.data.haarcascades + "haarcascade_frontalface_default.xml"
    cascade = cv2.CascadeClassifier(cascade_path)
    scale  = float(req.get("scale_factor", 1.1))
    minN   = int(req.get("min_neighbors", 5))
    faces  = cascade.detectMultiScale(gray, scaleFactor=scale, minNeighbors=minN)
    out    = img.copy()
    face_list = []
    for (x, y, w, h) in faces if len(faces) > 0 else []:
        face_list.append({"x": int(x), "y": int(y), "w": int(w), "h": int(h)})
        if req.get("draw_overlay", True):
            cv2.rectangle(out, (x, y), (x+w, y+h), (255, 0, 0), 2)
    return make_result(out, {"faces": face_list, "face_count": len(face_list)},
                       req.get("save_path"), t0)


def op_apply_threshold(req, img, t0):
    gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY) if img.ndim == 3 else img
    method = req.get("method", "otsu").lower()
    thresh = int(req.get("threshold", 127))
    if method == "otsu":
        _, out = cv2.threshold(gray, 0, 255, cv2.THRESH_BINARY + cv2.THRESH_OTSU)
    elif method == "adaptive_mean":
        out = cv2.adaptiveThreshold(gray, 255, cv2.ADAPTIVE_THRESH_MEAN_C,
                                    cv2.THRESH_BINARY, 11, 2)
    elif method == "adaptive_gaussian":
        out = cv2.adaptiveThreshold(gray, 255, cv2.ADAPTIVE_THRESH_GAUSSIAN_C,
                                    cv2.THRESH_BINARY, 11, 2)
    elif method == "binary_inv":
        _, out = cv2.threshold(gray, thresh, 255, cv2.THRESH_BINARY_INV)
    else:
        _, out = cv2.threshold(gray, thresh, 255, cv2.THRESH_BINARY)
    return make_result(out, {"method": method}, req.get("save_path"), t0)


def op_apply_morphology(req, img, t0):
    op_map = {
        "erode":    cv2.MORPH_ERODE,
        "dilate":   cv2.MORPH_DILATE,
        "open":     cv2.MORPH_OPEN,
        "close":    cv2.MORPH_CLOSE,
        "gradient": cv2.MORPH_GRADIENT,
        "tophat":   cv2.MORPH_TOPHAT,
        "blackhat": cv2.MORPH_BLACKHAT,
    }
    operation   = req.get("operation", "dilate").lower()
    kernel_size = int(req.get("kernel_size", 5))
    if kernel_size % 2 == 0:
        kernel_size += 1
    iterations  = int(req.get("iterations", 1))
    morph_op    = op_map.get(operation, cv2.MORPH_DILATE)
    kernel      = cv2.getStructuringElement(cv2.MORPH_RECT, (kernel_size, kernel_size))
    if operation == "erode":
        out = cv2.erode(img, kernel, iterations=iterations)
    elif operation == "dilate":
        out = cv2.dilate(img, kernel, iterations=iterations)
    else:
        out = cv2.morphologyEx(img, morph_op, kernel, iterations=iterations)
    return make_result(out, {"operation": operation, "kernel_size": kernel_size},
                       req.get("save_path"), t0)


def op_analyze_histogram(req, img, t0):
    bins = int(req.get("bins", 256))
    if img.ndim == 3:
        channel_names = req.get("channels", ["b", "g", "r"])
        channel_ids   = {"b": 0, "g": 1, "r": 2}
        hist_img = np.zeros((256, bins * 3, 3), dtype=np.uint8)
        means, stds = [], []
        colors = [(255, 0, 0), (0, 255, 0), (0, 0, 255)]
        for ci, ch_name in enumerate(["b", "g", "r"]):
            ch = img[:, :, ci]
            hist = cv2.calcHist([img], [ci], None, [bins], [0, 256])
            cv2.normalize(hist, hist, 0, 255, cv2.NORM_MINMAX)
            means.append(float(np.mean(ch)))
            stds.append(float(np.std(ch)))
            for x in range(bins):
                h = int(hist[x, 0])
                cv2.line(hist_img,
                         (ci * bins + x, 255),
                         (ci * bins + x, 255 - h),
                         colors[ci], 1)
        meta = {"channels": ["b", "g", "r"], "mean": means, "std_dev": stds}
        return make_result(hist_img, meta, req.get("save_path"), t0)
    else:
        hist = cv2.calcHist([img], [0], None, [bins], [0, 256])
        cv2.normalize(hist, hist, 0, 255, cv2.NORM_MINMAX)
        hist_img = np.zeros((256, bins, 1), dtype=np.uint8)
        for x in range(bins):
            h = int(hist[x, 0])
            cv2.line(hist_img, (x, 255), (x, 255 - h), 255, 1)
        meta = {"channels": ["gray"], "mean": [float(np.mean(img))], "std_dev": [float(np.std(img))]}
        return make_result(hist_img, meta, req.get("save_path"), t0)


def op_draw_annotations(req, img, t0):
    out = img.copy()
    for ann in req.get("annotations", []):
        atype     = ann.get("type", "rect")
        x, y      = int(ann.get("x", 0)), int(ann.get("y", 0))
        color     = tuple(ann.get("color", [0, 255, 0]))
        thickness = int(ann.get("thickness", 2))
        if atype == "rect":
            w, h = int(ann.get("w", 50)), int(ann.get("h", 50))
            cv2.rectangle(out, (x, y), (x+w, y+h), color, thickness)
            if ann.get("label"):
                cv2.putText(out, ann["label"], (x, y - 5),
                            cv2.FONT_HERSHEY_SIMPLEX, 0.6, color, 2)
        elif atype == "circle":
            r = int(ann.get("r", 20))
            cv2.circle(out, (x, y), r, color, thickness)
        elif atype == "text":
            label = ann.get("label", "")
            scale = float(ann.get("font_scale", 0.8))
            cv2.putText(out, label, (x, y), cv2.FONT_HERSHEY_SIMPLEX,
                        scale, color, thickness)
    return make_result(out, {"annotation_count": len(req.get("annotations", []))},
                       req.get("save_path"), t0)


def op_warp_perspective(req, img, t0):
    src_pts = np.float32(req["src_points"])
    ow = int(req["output_width"])
    oh = int(req["output_height"])
    dst_pts = np.float32([[0, 0], [ow, 0], [ow, oh], [0, oh]])
    M = cv2.getPerspectiveTransform(src_pts, dst_pts)
    out = cv2.warpPerspective(img, M, (ow, oh))
    return make_result(out, {"transform_matrix": M.tolist()}, req.get("save_path"), t0)


def op_pipeline(req, img, t0):
    steps = req.get("steps", [])
    current = img.copy()
    step_results = []
    for step in steps:
        step_op = step.get("op", "")
        step_req = {**req, **step, "image_path": None, "save_path": None}
        handler = OP_MAP.get(step_op)
        if handler is None:
            step_results.append({"op": step_op, "error": f"Unknown op: {step_op}"})
            continue
        result = handler(step_req, current, time.time())
        step_results.append({"op": step_op, "width": result["width"], "height": result["height"]})
        # Decode the base64 PNG back to numpy for the next step
        raw = base64.b64decode(result["image_b64"])
        arr = np.frombuffer(raw, dtype=np.uint8)
        current = cv2.imdecode(arr, cv2.IMREAD_UNCHANGED)
    final_save = req.get("final_save_path")
    if final_save:
        cv2.imwrite(final_save, current)
    result = make_result(current, {"steps": step_results, "step_count": len(steps)},
                         final_save, t0)
    return result


# ── Dispatch table ────────────────────────────────────────────────────────────

OP_MAP = {
    "convert_color":    op_convert_color,
    "resize_image":     op_resize_image,
    "blur_image":       op_blur_image,
    "detect_edges":     op_detect_edges,
    "find_contours":    op_find_contours,
    "detect_faces":     op_detect_faces,
    "apply_threshold":  op_apply_threshold,
    "apply_morphology": op_apply_morphology,
    "analyze_histogram": op_analyze_histogram,
    "draw_annotations": op_draw_annotations,
    "warp_perspective": op_warp_perspective,
    "pipeline":         op_pipeline,
}


# ── Entry point ───────────────────────────────────────────────────────────────

def main():
    try:
        raw = sys.stdin.read()
        req = json.loads(raw)
    except Exception as e:
        print(json.dumps({"error": f"JSON parse error: {e}"}))
        sys.exit(1)

    t0 = time.time()
    op = req.get("op", "")

    # Resource guard: refuse to process images larger than 64 MP
    image_path = req.get("image_path")
    img = None
    if image_path:
        img = cv2.imread(image_path, cv2.IMREAD_UNCHANGED)
        if img is None:
            print(json.dumps({"error": f"Cannot read image: {image_path}"}))
            sys.exit(1)
        pixels = img.shape[0] * img.shape[1]
        if pixels > 64_000_000:
            print(json.dumps({"error": f"Image too large ({pixels:,} px). Max 64 MP."}))
            sys.exit(1)
        # Normalise to BGR if needed
        if img.ndim == 2:
            pass  # grayscale OK
        elif img.shape[2] == 4:
            img = cv2.cvtColor(img, cv2.COLOR_BGRA2BGR)

    handler = OP_MAP.get(op)
    if handler is None:
        print(json.dumps({"error": f"Unknown operation: {op}. Available: {list(OP_MAP)}"}))
        sys.exit(1)

    try:
        result = handler(req, img, t0)
        print(json.dumps(result))
    except Exception as e:
        import traceback
        print(json.dumps({"error": str(e), "traceback": traceback.format_exc()}))
        sys.exit(1)


if __name__ == "__main__":
    main()
