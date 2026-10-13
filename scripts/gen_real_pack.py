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
from pathlib import Path

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
}


def run_action(action: str, cfg: dict, img_uri: str | None, api_key: str, endpoint: str,
               model: str, resolution: str, work_dir: Path, out: Path, size: int) -> None:
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
    manifest = video_to_pack.load_manifest(out)
    manifest["frameSize"] = {"w": size, "h": size}
    manifest["actions"][action] = {"fps": cfg["fps"], "frames": names}
    video_to_pack.save_manifest(out, manifest)
    video_to_pack.write_preview(out, action, names, size)
    print(f"  {action} 完成：{len(names)} 帧")


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
