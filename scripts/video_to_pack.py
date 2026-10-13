#!/usr/bin/env python3
"""把万相 i2v 生成的猫动作视频抽帧、逐帧抠图（去背景），打包成 CharacterPack。

用法（单个动作）：
    python scripts/video_to_pack.py --video work/lookDown.mp4 --action lookDown \
        --fps 8 --size 256 --frames 12 --out src-tauri/characters/real-default

多次运行不同 --action 会累积到同一 --out 目录，逐步拼出完整形象包。

依赖（在专用 venv 里安装）：
    pip install rembg onnxruntime opencv-python-headless
首次抠图会从网上下载抠图模型（约几十 MB）。
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import cv2
import numpy as np
from PIL import Image
from rembg import new_session, remove

MODEL = "isnet-general-use"


def square_canvas(im: Image.Image, size: int) -> Image.Image:
    """等比缩放到 size×size 的透明画布（不裁切，留白居中）。"""
    im = im.convert("RGBA")
    w, h = im.size
    scale = min(size / w, size / h)
    nw, nh = max(1, round(w * scale)), max(1, round(h * scale))
    im = im.resize((nw, nh), Image.Resampling.LANCZOS)
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    canvas.paste(im, ((size - nw) // 2, (size - nh) // 2), im)
    return canvas


def extract_frames(video: Path, count: int) -> list[np.ndarray]:
    cap = cv2.VideoCapture(str(video))
    if not cap.isOpened():
        raise SystemExit(f"无法打开视频：{video}")
    total = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))
    if total <= 0:
        raise SystemExit(f"视频无帧或无法读取：{video}")
    count = max(1, min(count, total))
    # 均匀取 count 帧
    idxs = [round(i * (total - 1) / (count - 1)) for i in range(count)] if count > 1 else [0]
    frames: list[np.ndarray] = []
    for i in idxs:
        cap.set(cv2.CAP_PROP_POS_FRAMES, i)
        ok, frame = cap.read()
        if ok:
            frames.append(frame)
    cap.release()
    if not frames:
        raise SystemExit("未能从视频抽取到任何帧。")
    return frames


def matte(frame: np.ndarray, session) -> Image.Image:
    rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
    pil = Image.fromarray(rgb)
    out = remove(pil, session=session)  # RGBA，透明背景
    return out.convert("RGBA")


def write_preview(out: Path, action: str, names: list[str], size: int) -> None:
    """写一张循环 GIF 预览 + 一张拼图（contact sheet），方便快速目视效果。"""
    bg = Image.new("RGBA", (size, size), (250, 246, 240, 255))
    frames_p = []
    for name in names:
        im = Image.open(out / name).convert("RGBA")
        comp = bg.copy()
        comp.alpha_composite(im)
        frames_p.append(comp.convert("P", palette=Image.Palette.ADAPTIVE, colors=256))
    gif = out / f"preview_{action}.gif"
    frames_p[0].save(
        gif, save_all=True, append_images=frames_p[1:],
        duration=max(60, int(1000 / 8)), loop=0, disposal=2,
    )
    print(f"  预览 GIF: {gif.name}")

    cols = min(6, len(names))
    rows = (len(names) + cols - 1) // cols
    sheet = Image.new("RGBA", (size * cols, size * rows), (0, 0, 0, 0))
    for i, name in enumerate(names):
        im = Image.open(out / name).convert("RGBA")
        x, y = (i % cols) * size, (i // cols) * size
        sheet.paste(im, (x, y), im)
    sheet_path = out / f"contact_{action}.png"
    sheet.save(sheet_path, "PNG")
    print(f"  拼图: {sheet_path.name}")


def load_manifest(out: Path) -> dict:
    p = out / "manifest.json"
    if p.exists():
        return json.loads(p.read_text(encoding="utf-8"))
    return {"name": out.name, "frameSize": {"w": 0, "h": 0}, "actions": {}}


def save_manifest(out: Path, manifest: dict) -> None:
    (out / "manifest.json").write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )


def main() -> None:
    ap = argparse.ArgumentParser(description="视频抽帧 + 抠图 → CharacterPack")
    ap.add_argument("--video", required=True)
    ap.add_argument("--action", required=True, help="动作名，如 lookDown / crawl")
    ap.add_argument("--out", required=True, help="输出目录（多次运行可累积）")
    ap.add_argument("--fps", type=int, default=8)
    ap.add_argument("--size", type=int, default=256, help="帧尺寸（正方形边长）")
    ap.add_argument("--frames", type=int, default=12, help="抽取帧数")
    ap.add_argument("--model", default=MODEL)
    args = ap.parse_args()

    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)

    print(f"抽取帧（目标 {args.frames} 帧）...")
    frames = extract_frames(Path(args.video), args.frames)
    print(f"共 {len(frames)} 帧，加载抠图模型（{args.model}）...")

    session = new_session(args.model)
    names: list[str] = []
    for i, frame in enumerate(frames, 1):
        name = f"{args.action}_{i:02d}.png"
        rgba = square_canvas(matte(frame, session), args.size)
        rgba.save(out / name, "PNG")
        names.append(name)
        print(f"  {name}")

    manifest = load_manifest(out)
    if manifest["frameSize"]["w"] != args.size:
        manifest["frameSize"] = {"w": args.size, "h": args.size}
    manifest["actions"][args.action] = {"fps": args.fps, "frames": names}
    save_manifest(out, manifest)
    write_preview(out, args.action, names, args.size)
    print(f"已写入 {len(names)} 帧到 {out}，动作 {args.action}。")


if __name__ == "__main__":
    main()
