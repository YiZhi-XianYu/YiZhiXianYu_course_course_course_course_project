import argparse
import os
import sys
from pathlib import Path

try:
    import cv2
    import numpy as np
except ImportError:
    print(
        "OpenCV and NumPy are required. Install them in cv_env first.",
        file=sys.stderr,
    )
    sys.exit(10)


def parse_args():
    parser = argparse.ArgumentParser(description="Place a transparent sticker above the main face.")
    parser.add_argument("--input", required=True)
    parser.add_argument("--sticker", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--scale", type=float, default=1.6)
    parser.add_argument("--y-offset", type=float, default=0.08)
    parser.add_argument("--smooth", type=float, default=0.72)
    parser.add_argument("--min-size", type=int, default=70)
    parser.add_argument("--lost-frames", type=int, default=12)
    parser.add_argument("--verbose", action="store_true")
    return parser.parse_args()


def find_cascade_path():
    if hasattr(cv2, "data") and hasattr(cv2.data, "haarcascades"):
        candidate = Path(cv2.data.haarcascades) / "haarcascade_frontalface_default.xml"
        if candidate.is_file():
            return candidate

    prefixes = []
    if sys.prefix:
        prefixes.append(Path(sys.prefix))
    conda_prefix = os.environ.get("CONDA_PREFIX")
    if conda_prefix:
        prefixes.append(Path(conda_prefix))

    for prefix in prefixes:
        candidates = [
            prefix / "Library" / "etc" / "haarcascades" / "haarcascade_frontalface_default.xml",
            prefix / "Lib" / "site-packages" / "cv2" / "data" / "haarcascade_frontalface_default.xml",
        ]
        for candidate in candidates:
            if candidate.is_file():
                return candidate

    return None


def choose_main_face(faces):
    if len(faces) == 0:
        return None
    return max(faces, key=lambda rect: rect[2] * rect[3])


def target_rect_from_face(face, sticker_ratio, scale, y_offset):
    x, y, w, h = face
    target_w = max(1.0, w * scale)
    target_h = max(1.0, target_w / sticker_ratio)
    center_x = x + w / 2.0
    bottom_y = y - h * y_offset
    left = center_x - target_w / 2.0
    top = bottom_y - target_h
    return np.array([left, top, target_w, target_h], dtype=np.float32)


def blend_rect(prev, current, smooth):
    if prev is None:
        return current
    smooth = max(0.0, min(0.98, smooth))
    return prev * smooth + current * (1.0 - smooth)


def overlay_rgba(frame, sticker_rgba, rect):
    x, y, w, h = rect
    x = int(round(x))
    y = int(round(y))
    w = max(1, int(round(w)))
    h = max(1, int(round(h)))

    frame_h, frame_w = frame.shape[:2]
    if x >= frame_w or y >= frame_h or x + w <= 0 or y + h <= 0:
        return

    resized = cv2.resize(sticker_rgba, (w, h), interpolation=cv2.INTER_AREA)
    sx0 = max(0, -x)
    sy0 = max(0, -y)
    sx1 = min(w, frame_w - x)
    sy1 = min(h, frame_h - y)
    if sx0 >= sx1 or sy0 >= sy1:
        return

    dx0 = max(0, x)
    dy0 = max(0, y)
    dx1 = dx0 + (sx1 - sx0)
    dy1 = dy0 + (sy1 - sy0)

    patch = resized[sy0:sy1, sx0:sx1]
    rgb = patch[:, :, :3].astype(np.float32)
    alpha = patch[:, :, 3:4].astype(np.float32) / 255.0
    base = frame[dy0:dy1, dx0:dx1].astype(np.float32)
    blended = rgb * alpha + base * (1.0 - alpha)
    frame[dy0:dy1, dx0:dx1] = blended.astype(np.uint8)


def main():
    args = parse_args()
    input_path = Path(args.input)
    sticker_path = Path(args.sticker)
    output_path = Path(args.output)

    if not input_path.is_file():
        print(f"input video not found: {input_path}", file=sys.stderr)
        return 2
    if not sticker_path.is_file():
        print(f"sticker image not found: {sticker_path}", file=sys.stderr)
        return 3

    sticker = cv2.imread(str(sticker_path), cv2.IMREAD_UNCHANGED)
    if sticker is None:
        print(f"failed to read sticker image: {sticker_path}", file=sys.stderr)
        return 4
    if sticker.shape[2] == 3:
        alpha = np.full(sticker.shape[:2] + (1,), 255, dtype=np.uint8)
        sticker = np.concatenate([sticker, alpha], axis=2)
    sticker_ratio = sticker.shape[1] / sticker.shape[0]

    cascade_path = find_cascade_path()
    if cascade_path is None:
        print("failed to find haarcascade_frontalface_default.xml", file=sys.stderr)
        return 5
    detector = cv2.CascadeClassifier(str(cascade_path))
    if detector.empty():
        print(f"failed to load face detector: {cascade_path}", file=sys.stderr)
        return 6

    cap = cv2.VideoCapture(str(input_path))
    if not cap.isOpened():
        print(f"failed to open input video: {input_path}", file=sys.stderr)
        return 7

    fps = cap.get(cv2.CAP_PROP_FPS) or 25.0
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))
    total = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))

    output_path.parent.mkdir(parents=True, exist_ok=True)
    fourcc = cv2.VideoWriter_fourcc(*"mp4v")
    writer = cv2.VideoWriter(str(output_path), fourcc, fps, (width, height))
    if not writer.isOpened():
        print(f"failed to open output video: {output_path}", file=sys.stderr)
        return 8

    smoothed = None
    lost = args.lost_frames + 1
    frame_index = 0
    attached_frames = 0

    while True:
        ok, frame = cap.read()
        if not ok:
            break

        gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)
        gray = cv2.equalizeHist(gray)
        faces = detector.detectMultiScale(
            gray,
            scaleFactor=1.08,
            minNeighbors=5,
            minSize=(args.min_size, args.min_size),
        )
        face = choose_main_face(faces)

        if face is not None:
            current = target_rect_from_face(face, sticker_ratio, args.scale, args.y_offset)
            smoothed = blend_rect(smoothed, current, args.smooth)
            lost = 0
        else:
            lost += 1

        if smoothed is not None and lost <= args.lost_frames:
            overlay_rgba(frame, sticker, smoothed)
            attached_frames += 1

        writer.write(frame)
        frame_index += 1
        if args.verbose and frame_index % 100 == 0:
            print(
                f"[companion] frame {frame_index}/{total}, faces={len(faces)}, lost={lost}",
                flush=True,
            )

    cap.release()
    writer.release()

    if args.verbose:
        print(
            f"[companion] processed={frame_index}, attached_frames={attached_frames}",
            flush=True,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
