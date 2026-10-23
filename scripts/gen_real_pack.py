#!/usr/bin/env python3
"""一键生成「照片级真实猫」CharacterPack：万相生视频 → 抽帧抠图 → 打包。

文生视频（无图，AI 自己生成一只逼真猫，推荐）：
    .venv-pipeline\\Scripts\\python scripts/gen_real_pack.py ^
        --out src-tauri/characters/real-cat --actions lookDown

图生视频（用猫照片做参考）：
    .venv-pipeline\\Scripts\\python scripts/gen_real_pack.py ^
        --img cat.jpg --out src-tauri/characters/real-cat --actions lookDown

必须用 .venv-pipeline 里的 python（因为要 rembg/cv2）。
Key 来源优先级：--api-key > DASHSCOPE_API_KEY > config.json 的 wanxiang.apiKey。
"""
from __future__ import annotations

import argparse
import sys
import time
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent))
import video_to_pack
import wanx_i2v

# 文生视频模式下，AI 每次生成前统一加这段外观描述，保证 6 个动作长得是同一只猫。
# 外观：中华田园猫·狸花猫；背景：纯白（便于抠图，最终只保留猫）。
CAT = (
    "A photorealistic Chinese Li Hua cat (Chinese rural tabby cat), with a brown-gray coat "
    "covered in dark black tabby stripes, yellow-green eyes, lean muscular body, "
    "short smooth fur, pointed ears and natural whiskers, full body clearly visible, "
    "isolated on a plain solid white background with no other objects."
)

# 每个动作：动作提示词（描述连贯动作）+ 打包参数。
ACTIONS: dict[str, dict] = {
    "lookDown": {
        "prompt": (
            "This cat peeks its head down from the top edge of the frame, "
            "leaning forward and looking downward curiously, ears perked, "
            "then holds still and blinks once. Natural realistic fur, "
            "soft indoor lighting, photorealistic."
        ),
        "fps": 8,
        "frames": 12,
        "duration": 5,
    },
    "crawl": {
        "prompt": (
            "This cat climbs downward along a vertical surface from the top of the frame, "
            "paws gripping and stepping down one by one, tail balancing, side view. "
            "Natural realistic cat climbing motion, photorealistic."
        ),
        "fps": 10,
        "frames": 14,
        "duration": 5,
    },
    "sneakPeek": {
        "prompt": (
            "This cat peeks down from the top edge of the frame, glances around briefly, "
            "then pulls its head back up and out of frame. "
            "Natural realistic fur, photorealistic."
        ),
        "fps": 8,
        "frames": 12,
        "duration": 5,
    },
    "jumpUp": {
        "prompt": (
            "This cat crouches low, then leaps straight upward with a powerful jump, "
            "body stretching and tail trailing, rising out of the bottom of the frame. "
            "Natural realistic cat jumping motion, photorealistic."
        ),
        "fps": 12,
        "frames": 12,
        "duration": 5,
    },
    "happyClimb": {
        "prompt": (
            "This cat happily climbs upward, front paws reaching and pulling upward one after another, "
            "hind legs pushing, cheerful energetic expression. "
            "Natural realistic cat climbing motion, photorealistic."
        ),
        "fps": 10,
        "frames": 12,
        "duration": 5,
    },
    "fall": {
        "prompt": (
            "This cat loses its grip and falls downward through the air, "
            "paws splayed, body tumbling slightly, tail flailing. "
            "Natural realistic cat falling motion, photorealistic."
        ),
        "fps": 12,
        "frames": 12,
        "duration": 5,
    },
    # walk：万相文生图定格（wan2.6-t2i）。用保守站立/微抬爪相位，避免视频鬼腿。
    "walk": {
        "mode": "t2i_stills",
        "fps": 3,
        "negative_prompt": (
            "extra legs, five legs, six legs, ghost limbs, duplicated paws, "
            "motion blur, motion trails, deformed legs, merged legs, "
            "blurry paws, three front legs, running, jumping, flying"
        ),
        "phases": [
            (
                "strict orthographic side profile facing right, standing still, "
                "near front paw raised one centimeter only, other three paws flat on ground, "
                "exactly four legs, museum specimen photo, sharp focus, full body"
            ),
            (
                "strict orthographic side profile facing right, standing still, "
                "weight shifted slightly forward, all four paws on ground, "
                "exactly four legs, museum specimen photo, sharp focus, full body"
            ),
        ],
    },
}


def _pack_action_pngs(action: str, names: list[str], out: Path, size: int, fps: int) -> None:
    for old in out.glob(f"{action}_*.png"):
        if old.name not in names:
            old.unlink(missing_ok=True)
    manifest = video_to_pack.load_manifest(out)
    manifest["frameSize"] = {"w": size, "h": size}
    manifest["actions"][action] = {"fps": fps, "frames": names}
    video_to_pack.save_manifest(out, manifest)
    video_to_pack.write_preview(out, action, names, size)
    print(f"  {action} 完成：{len(names)} 帧")


def run_walk_stills(cfg: dict, api_key: str, work_dir: Path, out: Path, size: int) -> None:
    """万相文生图定格步态帧（避免视频鬼腿）。"""
    work_dir.mkdir(parents=True, exist_ok=True)
    out.mkdir(parents=True, exist_ok=True)
    phases: list[str] = list(cfg["phases"])
    session = video_to_pack.new_session(video_to_pack.MODEL)
    names: list[str] = []
    print(f"\n=== 万相文生图生成 walk 定格帧（{len(phases)} 张，model={wanx_i2v.DEFAULT_T2I_MODEL}）===")
    for i, phase in enumerate(phases, 1):
        prompt = (
            f"{CAT} {phase}. Full body visible, head and back not cropped, "
            "tail gently curved, calm upright walk, photorealistic."
        )
        raw = work_dir / f"walk_still_{i:02d}.png"
        print(f"  [{i}/{len(phases)}] 提交文生图...")
        task_id = wanx_i2v.submit_t2i(
            api_key,
            wanx_i2v.DEFAULT_T2I_ENDPOINT,
            wanx_i2v.DEFAULT_T2I_MODEL,
            prompt,
            negative_prompt=cfg.get("negative_prompt"),
        )
        url = wanx_i2v.poll_t2i(api_key, task_id)
        wanx_i2v.download(url, raw)
        name = f"walk_{i:02d}.png"
        pil = Image.open(raw).convert("RGB")
        bgr = cv2.cvtColor(np.array(pil), cv2.COLOR_RGB2BGR)
        video_to_pack.square_canvas(video_to_pack.matte(bgr, session), size).save(out / name, "PNG")
        names.append(name)
        print(f"    -> {name}")
        # 避开限流
        if i < len(phases):
            time.sleep(2)
    _pack_action_pngs("walk", names, out, size, int(cfg["fps"]))


def run_action(action: str, cfg: dict, img_uri: str | None, api_key: str, endpoint: str,
               model: str, resolution: str, work_dir: Path, out: Path, size: int) -> None:
    if cfg.get("mode") == "t2i_stills":
        run_walk_stills(cfg, api_key, work_dir, out, size)
        return

    work_dir.mkdir(parents=True, exist_ok=True)
    out.mkdir(parents=True, exist_ok=True)
    video = work_dir / f"{action}.mp4"
    # 文生视频（无图）时统一加猫外观描述，保证各动作是同一只猫
    prompt = (CAT + " " + cfg["prompt"]) if img_uri is None else cfg["prompt"]
    print(f"\n=== 生成动作 {action} 视频 ===")
    task_id = wanx_i2v.submit(api_key, endpoint, model, img_uri, prompt,
                              resolution, cfg["duration"])
    print(f"  task_id={task_id}，等待生成...")
    video_url = wanx_i2v.poll(api_key, task_id)
    wanx_i2v.download(video_url, video)
    print(f"  视频已下载：{video}")

    print(f"=== 抽帧 + 抠图 + 打包 {action} ===")
    frames = video_to_pack.extract_frames(video, cfg["frames"])
    session = video_to_pack.new_session(video_to_pack.MODEL)
    names: list[str] = []
    for i, frame in enumerate(frames, 1):
        name = f"{action}_{i:02d}.png"
        video_to_pack.square_canvas(video_to_pack.matte(frame, session), size).save(out / name, "PNG")
        names.append(name)
    _pack_action_pngs(action, names, out, size, int(cfg["fps"]))


def main() -> None:
    ap = argparse.ArgumentParser(description="一键生成照片级真实猫 CharacterPack")
    ap.add_argument("--img", default=None, help="猫照片（本地路径；省略则文生视频，AI 自己生成）")
    ap.add_argument("--out", required=True, help="输出形象包目录")
    ap.add_argument("--actions", default=",".join(ACTIONS), help="逗号分隔的动作名，默认全部")
    ap.add_argument("--size", type=int, default=512)
    ap.add_argument("--model", default=None, help="默认：无图 wan2.6-t2v，有图 wan2.6-i2v")
    ap.add_argument("--resolution", default="1080P", choices=["480P", "720P", "1080P"])
    ap.add_argument("--endpoint", default=wanx_i2v.DEFAULT_ENDPOINT)
    ap.add_argument("--api-key", default=None)
    args = ap.parse_args()

    api_key = wanx_i2v.load_api_key(args.api_key)
    img_uri: str | None = wanx_i2v.image_to_data_uri(Path(args.img)) if args.img else None
    model = args.model or (wanx_i2v.DEFAULT_MODEL if img_uri else wanx_i2v.DEFAULT_T2V_MODEL)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    work_dir = out / "_work"
    work_dir.mkdir(parents=True, exist_ok=True)

    wanted = [a.strip() for a in args.actions.split(",") if a.strip()]
    for action in wanted:
        if action not in ACTIONS:
            print(f"跳过未知动作：{action}")
            continue
        run_action(action, ACTIONS[action], img_uri, api_key, args.endpoint,
                   model, args.resolution, work_dir, out, args.size)

    print(f"\n全部完成，形象包位于：{out}")


if __name__ == "__main__":
    main()
