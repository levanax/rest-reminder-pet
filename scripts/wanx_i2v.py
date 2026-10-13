#!/usr/bin/env python3
"""用阿里云百炼·万相生成真实猫动作视频，支持两种模式：

文生视频（无图，AI 自己生成一只猫）：
    python scripts/wanx_i2v.py --prompt "一只逼真的猫从屏幕顶部探头往下看" --out work/lookDown.mp4

图生视频（用照片做参考）：
    python scripts/wanx_i2v.py --img cat.jpg --prompt "这只猫探头往下看" --out work/lookDown.mp4

Key 来源优先级：--api-key > 环境变量 DASHSCOPE_API_KEY > config.json 的 wanxiang.apiKey。
图生视频的本地图片会转 base64 data URI 直传（单张 < 20MB，无需公网 URL）。
"""
from __future__ import annotations

import argparse
import base64
import json
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

DEFAULT_MODEL = "wan2.6-i2v"   # 图生视频（需要 --img）
DEFAULT_T2V_MODEL = "wan2.6-t2v"  # 文生视频（无图，AI 自己生成）
DEFAULT_ENDPOINT = "https://dashscope.aliyuncs.com/api/v1/services/aigc/video-generation/video-synthesis"
TASK_URL = "https://dashscope.aliyuncs.com/api/v1/tasks/{task_id}"

MAX_IMAGE_BYTES = 20 * 1024 * 1024  # 万相 i2v 单图上限（base64 前）


def load_api_key(cli_key: str | None) -> str:
    if cli_key and cli_key.strip():
        return cli_key.strip()
    env = os.environ.get("DASHSCOPE_API_KEY", "").strip()
    if env:
        return env
    # 从 AppData 的 config.json 读取（与 Rust 端一致）
    cfg_path = (
        Path(os.environ.get("APPDATA", str(Path.home())))
        / "rest-reminder-pet"
        / "config.json"
    )
    try:
        cfg = json.loads(cfg_path.read_text(encoding="utf-8"))
        key = cfg.get("wanxiang", {}).get("apiKey")
        if key and key.strip():
            return key.strip()
    except Exception:
        pass
    raise SystemExit(
        "未找到 API Key。请用 --api-key 传入、设置环境变量 DASHSCOPE_API_KEY，"
        "或在设置页面填写 wanxiang.apiKey。"
    )


def image_to_data_uri(path: Path) -> str:
    data = path.read_bytes()
    if len(data) > MAX_IMAGE_BYTES:
        raise SystemExit(f"图片过大（{len(data)} 字节），万相 i2v 单图上限 {MAX_IMAGE_BYTES} 字节。")
    mime = "image/jpeg"
    if path.suffix.lower() in (".png",):
        mime = "image/png"
    elif path.suffix.lower() in (".webp",):
        mime = "image/webp"
    return f"data:{mime};base64," + base64.b64encode(data).decode("ascii")


def http_json(method: str, url: str, api_key: str, body: dict | None) -> dict:
    req = urllib.request.Request(url, method=method)
    req.add_header("Authorization", f"Bearer {api_key}")
    req.add_header("Content-Type", "application/json")
    if method == "POST":
        req.add_header("X-DashScope-Async", "enable")
    data = None
    if body is not None:
        data = json.dumps(body).encode("utf-8")
    try:
        with urllib.request.urlopen(req, data=data, timeout=60) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except urllib.error.HTTPError as e:
        detail = e.read().decode("utf-8", "replace")
        raise SystemExit(f"HTTP {e.code}: {detail}")


def submit(api_key: str, endpoint: str, model: str, img_url: str | None, prompt: str,
           resolution: str, duration: int) -> str:
    inp: dict = {"prompt": prompt}
    if img_url:
        inp["img_url"] = img_url
    body = {
        "model": model,
        "input": inp,
        "parameters": {
            "resolution": resolution,
            "duration": duration,
            "prompt_extend": True,
            "watermark": False,
        },
    }
    resp = http_json("POST", endpoint, api_key, body)
    task_id = resp.get("output", {}).get("task_id")
    if not task_id:
        raise SystemExit(f"提交失败，响应：{json.dumps(resp, ensure_ascii=False)}")
    return task_id


def poll(api_key: str, task_id: str, timeout_s: int = 900) -> str:
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        resp = http_json("GET", TASK_URL.format(task_id=task_id), api_key, None)
        status = resp.get("output", {}).get("task_status", "")
        if status == "SUCCEEDED":
            url = resp.get("output", {}).get("video_url")
            if not url:
                raise SystemExit(f"任务成功但无 video_url：{json.dumps(resp, ensure_ascii=False)}")
            return url
        if status in ("FAILED", "CANCELED", "UNKNOWN"):
            raise SystemExit(f"任务失败（{status}）：{json.dumps(resp, ensure_ascii=False)}")
        time.sleep(3)
    raise SystemExit("任务超时")


def download(url: str, out: Path) -> None:
    out.parent.mkdir(parents=True, exist_ok=True)
    req = urllib.request.Request(url)
    with urllib.request.urlopen(req, timeout=120) as resp, out.open("wb") as f:
        while True:
            chunk = resp.read(1 << 16)
            if not chunk:
                break
            f.write(chunk)


def main() -> None:
    ap = argparse.ArgumentParser(description="万相生视频：文生视频（无图）或图生视频（有图）")
    ap.add_argument("--img", help="本地图片路径（图生视频；省略则文生视频）")
    ap.add_argument("--img-url", help="或直接提供公网图片 URL")
    ap.add_argument("--prompt", required=True, help="动作描述")
    ap.add_argument("--out", required=True, help="输出视频路径（.mp4）")
    ap.add_argument("--model", default=None, help="默认：有图用 wan2.6-i2v，无图用 wan2.6-t2v")
    ap.add_argument("--resolution", default="720P", choices=["480P", "720P", "1080P"])
    ap.add_argument("--duration", type=int, default=5)
    ap.add_argument("--endpoint", default=DEFAULT_ENDPOINT)
    ap.add_argument("--api-key", default=None)
    ap.add_argument("--timeout", type=int, default=900)
    args = ap.parse_args()

    if args.img and args.img_url:
        raise SystemExit("--img 与 --img-url 只能二选一。")

    api_key = load_api_key(args.api_key)

    if args.img:
        img_url: str | None = image_to_data_uri(Path(args.img))
    elif args.img_url:
        img_url = args.img_url
    else:
        img_url = None

    model = args.model or (DEFAULT_MODEL if img_url else DEFAULT_T2V_MODEL)
    mode = "i2v" if img_url else "t2v"
    print(f"提交 {mode} 任务（model={model}, resolution={args.resolution}, duration={args.duration}s）...")
    task_id = submit(api_key, args.endpoint, model, img_url, args.prompt,
                     args.resolution, args.duration)
    print(f"task_id={task_id}，等待生成...")
    video_url = poll(api_key, task_id, timeout_s=args.timeout)
    print(f"生成完成，下载到 {args.out} ...")
    download(video_url, Path(args.out))
    print(f"OK: {args.out}")


if __name__ == "__main__":
    main()
