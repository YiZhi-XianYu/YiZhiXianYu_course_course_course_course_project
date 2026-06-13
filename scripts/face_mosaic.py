import argparse
import os
import sys
from pathlib import Path

try:
    import cv2
except ImportError:
    print(
        "OpenCV is not installed. Install it with: python -m pip install opencv-python",
        file=sys.stderr,
    )
    sys.exit(10)


def parse_args():
    parser = argparse.ArgumentParser(description="Apply face mosaic to a video.")
    parser.add_argument("--input", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--scale", type=float, default=1.25)
    parser.add_argument("--block-size", type=int, default=18)
    parser.add_argument("--min-size", type=int, default=40)
    parser.add_argument("--verbose", action="store_true")
    return parser.parse_args()


def expand_rect(x, y, w, h, scale, max_w, max_h):
    cx = x + w / 2.0
    cy = y + h / 2.0
    side_w = w * scale
    side_h = h * scale
    x0 = max(0, int(round(cx - side_w / 2.0)))
    y0 = max(0, int(round(cy - side_h / 2.0)))
    x1 = min(max_w, int(round(cx + side_w / 2.0)))
    y1 = min(max_h, int(round(cy + side_h / 2.0)))
    return x0, y0, max(1, x1 - x0), max(1, y1 - y0)


def apply_mosaic(frame, rect, block_size):
    x, y, w, h = rect
    roi = frame[y : y + h, x : x + w]
    if roi.size == 0:
        return

    small_w = max(1, w // block_size)
    small_h = max(1, h // block_size)
    small = cv2.resize(roi, (small_w, small_h), interpolation=cv2.INTER_LINEAR)
    mosaic = cv2.resize(small, (w, h), interpolation=cv2.INTER_NEAREST)
    frame[y : y + h, x : x + w] = mosaic


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


def main():
    args = parse_args()
    input_path = Path(args.input)
    output_path = Path(args.output)

    if not input_path.is_file():
        print(f"input video not found: {input_path}", file=sys.stderr)
        return 2

    cascade_path = find_cascade_path()
    if cascade_path is None:
        print("failed to find haarcascade_frontalface_default.xml", file=sys.stderr)
        return 3

    detector = cv2.CascadeClassifier(str(cascade_path))
    if detector.empty():
        print(f"failed to load face detector: {cascade_path}", file=sys.stderr)
        return 3

    cap = cv2.VideoCapture(str(input_path))
    if not cap.isOpened():
        print(f"failed to open input video: {input_path}", file=sys.stderr)
        return 4

    fps = cap.get(cv2.CAP_PROP_FPS) or 25.0
    width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))
    total = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))

    output_path.parent.mkdir(parents=True, exist_ok=True)
    fourcc = cv2.VideoWriter_fourcc(*"mp4v")
    writer = cv2.VideoWriter(str(output_path), fourcc, fps, (width, height))
    if not writer.isOpened():
        print(f"failed to open output video: {output_path}", file=sys.stderr)
        return 5

    frame_index = 0
    detected_faces = 0
    while True:
        ok, frame = cap.read()
        if not ok:
            break

        gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)
        gray = cv2.equalizeHist(gray)
        faces = detector.detectMultiScale(
            gray,
            scaleFactor=1.1,
            minNeighbors=5,
            minSize=(args.min_size, args.min_size),
        )

        for face in faces:
            rect = expand_rect(*face, args.scale, width, height)
            apply_mosaic(frame, rect, args.block_size)
        detected_faces += len(faces)

        writer.write(frame)
        frame_index += 1
        if args.verbose and frame_index % 100 == 0:
            print(f"[mosaic] frame {frame_index}/{total}, faces={len(faces)}", flush=True)

    cap.release()
    writer.release()

    if args.verbose:
        print(f"[mosaic] processed={frame_index}, detected_faces={detected_faces}", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
