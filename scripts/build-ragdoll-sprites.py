"""Build default CharacterPack frames from the bundled cat photo."""
from __future__ import annotations

import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageEnhance, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "characters" / "default-source" / "cat-source.jpg"
OUT = ROOT / "characters" / "default"
PREVIEW = ROOT / "characters" / "default-source" / "previews"
ICON_PNG = ROOT / "icons" / "icon.png"
SIZE = 128
VARIANT = "cat-photo"

# Relative crop boxes (left, top, right, bottom) on source image — face-focused
CROPS = {
    "face": (0.12, 0.02, 0.88, 0.72),
    "peek": (0.18, 0.00, 0.82, 0.42),
    "gaze": (0.14, 0.06, 0.86, 0.76),
    "torso": (0.10, 0.02, 0.90, 0.82),
    "tumble": (0.08, 0.00, 0.92, 0.78),
}


def soft_ellipse_cutout(im: Image.Image) -> Image.Image:
    """Keep photo colors; fade edges with an elliptical alpha mask."""
    im = im.convert("RGBA")
    w, h = im.size
    mask = Image.new("L", (w, h), 0)
    draw = ImageDraw.Draw(mask)
    inset_x, inset_y = int(w * 0.04), int(h * 0.04)
    draw.ellipse((inset_x, inset_y, w - inset_x, h - inset_y), fill=255)
    mask = mask.filter(ImageFilter.GaussianBlur(radius=max(4, w // 40)))
    r, g, b, _ = im.split()
    return Image.merge("RGBA", (r, g, b, mask))


def to_sprite(im: Image.Image) -> Image.Image:
    return im.resize((SIZE, SIZE), Image.Resampling.LANCZOS)


def shift(im: Image.Image, dx: int, dy: int) -> Image.Image:
    out = Image.new("RGBA", im.size, (0, 0, 0, 0))
    out.paste(im, (dx, dy), im)
    return out


def rotate(im: Image.Image, deg: float) -> Image.Image:
    return im.rotate(deg, resample=Image.Resampling.BICUBIC, expand=False)


def scale_about_center(im: Image.Image, sx: float, sy: float | None = None) -> Image.Image:
    if sy is None:
        sy = sx
    w, h = im.size
    nw, nh = max(1, int(w * sx)), max(1, int(h * sy))
    resized = im.resize((nw, nh), Image.Resampling.LANCZOS)
    out = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    out.paste(resized, ((w - nw) // 2, (h - nh) // 2), resized)
    return out


def bright(im: Image.Image, factor: float) -> Image.Image:
    rgb = ImageEnhance.Brightness(im.convert("RGB")).enhance(factor)
    r, g, b = rgb.split()
    return Image.merge("RGBA", (r, g, b, im.split()[-1]))


def crop_box(raw: Image.Image, key: str) -> Image.Image:
    w, h = raw.size
    l, t, r, b = CROPS[key]
    return raw.crop((int(w * l), int(h * t), int(w * r), int(h * b)))


def make_base(raw: Image.Image, crop_key: str) -> Image.Image:
    face = crop_box(raw, crop_key)
    face = ImageEnhance.Contrast(face).enhance(1.06)
    face = ImageEnhance.Color(face).enhance(1.04)
    return to_sprite(soft_ellipse_cutout(face))


def compose(
    base: Image.Image,
    *,
    dx: int = 0,
    dy: int = 0,
    rot: float = 0.0,
    sx: float = 1.0,
    sy: float | None = None,
    brightness: float = 1.0,
) -> Image.Image:
    im = base
    if sx != 1.0 or (sy is not None and sy != 1.0):
        im = scale_about_center(im, sx, sy)
    if dx or dy:
        im = shift(im, dx, dy)
    if rot:
        im = rotate(im, rot)
    if brightness != 1.0:
        im = bright(im, brightness)
    return im


def write_preview_gifs(frames: dict[str, list[str]], actions: dict) -> None:
    PREVIEW.mkdir(parents=True, exist_ok=True)
    for action, files in frames.items():
        if not files:
            continue
        fps = int(actions[action]["fps"])
        duration = max(40, int(1000 / fps))
        imgs: list[Image.Image] = []
        for name in files:
            im = Image.open(OUT / name).convert("RGBA")
            # solid bg so GIF transparency does not smear
            bg = Image.new("RGBA", im.size, (250, 246, 240, 255))
            bg.alpha_composite(im)
            imgs.append(bg.convert("P", palette=Image.Palette.ADAPTIVE, colors=256))
        out = PREVIEW / f"{action}.gif"
        imgs[0].save(
            out,
            save_all=True,
            append_images=imgs[1:],
            duration=duration,
            loop=0,
            disposal=2,
        )
        print(f"  preview gif: {out.name}")


def write_app_icon(raw: Image.Image) -> None:
    """Square face cutout → icons/icon.png (then prefer `npm run gen-icons`)."""
    # Face-centered crop for tray / installer clarity
    w, h = raw.size
    l, t, r, b = 0.16, 0.00, 0.84, 0.62
    face = raw.crop((int(w * l), int(h * t), int(w * r), int(h * b)))
    face = ImageEnhance.Contrast(face).enhance(1.08)
    face = soft_ellipse_cutout(face)
    canvas = Image.new("RGBA", (1024, 1024), (0, 0, 0, 0))
    sized = face.resize((960, 960), Image.Resampling.LANCZOS)
    canvas.paste(sized, ((1024 - 960) // 2, (1024 - 960) // 2), sized)
    ICON_PNG.parent.mkdir(parents=True, exist_ok=True)
    canvas.save(ICON_PNG, "PNG")
    # Windows sizes used by tauri.conf.json
    for path, px in [
        (ICON_PNG.parent / "32x32.png", 32),
        (ICON_PNG.parent / "64x64.png", 64),
        (ICON_PNG.parent / "128x128.png", 128),
        (ICON_PNG.parent / "128x128@2x.png", 256),
    ]:
        canvas.resize((px, px), Image.Resampling.LANCZOS).save(path, "PNG")
    ico_sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    ico_imgs = [canvas.resize(s, Image.Resampling.LANCZOS) for s in ico_sizes]
    ico_imgs[0].save(
        ICON_PNG.parent / "icon.ico",
        format="ICO",
        sizes=ico_sizes,
        append_images=ico_imgs[1:],
    )
    print(f"  app icon: {ICON_PNG.relative_to(ROOT)}")


def main() -> None:
    if not SRC.exists():
        raise SystemExit(f"missing source image: {SRC}")

    OUT.mkdir(parents=True, exist_ok=True)
    for p in OUT.glob("*.png"):
        p.unlink()

    raw = Image.open(SRC).convert("RGB")
    bases = {
        "face": make_base(raw, "face"),
        "peek": make_base(raw, "peek"),
        "gaze": make_base(raw, "gaze"),
        "torso": make_base(raw, "torso"),
        "tumble": make_base(raw, "tumble"),
    }

    frames: dict[str, list[str]] = {
        "crawl": [],
        "lookDown": [],
        "sneakPeek": [],
        "jumpUp": [],
        "happyClimb": [],
        "fall": [],
    }

    def save(name: str, im: Image.Image, action: str) -> None:
        im.save(OUT / name, "PNG")
        frames[action].append(name)

    # jumpUp: crouch squash → launch stretch → mid → apex → settle → soft
    jump_specs = [
        (0, 6, 0.0, 1.10, 0.88),
        (0, -2, 0.0, 0.92, 1.14),
        (0, -4, -2.0, 0.96, 1.08),
        (0, -2, 0.0, 1.02, 0.96),
        (0, 2, 1.0, 1.06, 0.94),
        (0, 0, 0.0, 1.0, 1.0),
    ]
    for i, (dx, dy, rot, sx, sy) in enumerate(jump_specs, 1):
        save(
            f"jump_{i:02d}.png",
            compose(bases["tumble"], dx=dx, dy=dy, rot=rot, sx=sx, sy=sy),
            "jumpUp",
        )

    # crawl: L上 → 中 → R上 → 中 → L下 → R下
    crawl_specs = [
        (-8, -6, -10.0),
        (0, -2, 0.0),
        (8, -6, 10.0),
        (0, 0, 0.0),
        (-7, 5, -8.0),
        (7, 5, 8.0),
    ]
    for i, (dx, dy, rot) in enumerate(crawl_specs, 1):
        save(
            f"crawl_{i:02d}.png",
            compose(bases["torso"], dx=dx, dy=dy, rot=rot),
            "crawl",
        )

    # lookDown: 下探 → 稳 → 微侧倾 → 稳 → 眨眼
    look_specs = [
        (0, 8, 0.0, 1.02, 1.0, 1.0),
        (0, 4, 0.0, 1.0, 1.0, 1.0),
        (6, 5, 6.0, 1.0, 1.0, 1.0),
        (0, 4, 0.0, 1.0, 1.0, 1.0),
        (0, 3, 0.0, 0.97, 0.97, 0.92),
    ]
    for i, (dx, dy, rot, sx, sy, br) in enumerate(look_specs, 1):
        save(
            f"look_{i:02d}.png",
            compose(bases["gaze"], dx=dx, dy=dy, rot=rot, sx=sx, sy=sy, brightness=br),
            "lookDown",
        )

    # sneakPeek: 只露耳 → 半脸 → 多一点 → 半脸犹豫
    sneak_specs = [
        (0, 12, 0.0, 0.92),
        (0, 8, -2.0, 0.96),
        (0, 4, 0.0, 1.0),
        (2, 7, 3.0, 0.97),
    ]
    for i, (dx, dy, rot, sc) in enumerate(sneak_specs, 1):
        save(
            f"sneak_{i:02d}.png",
            compose(bases["peek"], dx=dx, dy=dy, rot=rot, sx=sc, sy=sc),
            "sneakPeek",
        )

    # happyClimb: 上拉L → 中 → 上拉R → 中 → 伸展
    climb_specs = [
        (-8, -6, 9.0, 1.0, 1.04, 1.04),
        (0, -3, 0.0, 1.0, 1.0, 1.02),
        (8, -6, -9.0, 1.0, 1.04, 1.04),
        (0, -2, 0.0, 1.0, 1.0, 1.02),
        (0, -8, 0.0, 0.96, 1.08, 1.05),
    ]
    for i, (dx, dy, rot, sx, sy, br) in enumerate(climb_specs, 1):
        save(
            f"climb_{i:02d}.png",
            compose(bases["torso"], dx=dx, dy=dy, rot=rot, sx=sx, sy=sy, brightness=br),
            "happyClimb",
        )

    # fall: 失稳 → 加速转 → 大转 → 反向 → 更大转 → 拉长
    fall_specs = [
        (0, 2, 8.0, 1.0, 1.0),
        (2, 4, 14.0, 0.98, 1.03),
        (-2, 6, 20.0, 0.97, 1.05),
        (3, 8, -12.0, 0.98, 1.04),
        (-1, 10, 22.0, 0.96, 1.07),
        (0, 12, -18.0, 0.95, 1.08),
    ]
    for i, (dx, dy, rot, sx, sy) in enumerate(fall_specs, 1):
        save(
            f"fall_{i:02d}.png",
            compose(bases["tumble"], dx=dx, dy=dy, rot=rot, sx=sx, sy=sy),
            "fall",
        )

    actions = {
        "crawl": {"fps": 10, "frames": frames["crawl"]},
        "lookDown": {"fps": 6, "frames": frames["lookDown"]},
        "sneakPeek": {"fps": 6, "frames": frames["sneakPeek"]},
        "jumpUp": {"fps": 12, "frames": frames["jumpUp"]},
        "happyClimb": {"fps": 10, "frames": frames["happyClimb"]},
        "fall": {"fps": 12, "frames": frames["fall"]},
    }
    manifest = {
        "name": "default",
        "variant": VARIANT,
        "frameSize": {"w": SIZE, "h": SIZE},
        "actions": actions,
    }
    (OUT / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")

    (ROOT / "characters" / "default-source" / "ATTRIBUTION.md").write_text(
        """# Default character source

- Image: `cat-source.jpg` (user-provided photo for local Rest Reminder Pet)
- Processed into 128×128 elliptical cutout animation frames (pseudo-motion from a still)
- Preview GIFs: `previews/*.gif`
- App icon: derived face cutout written to `icons/`

If redistributing this project, replace with an image you have rights to use, or restore a public-domain source.
""",
        encoding="utf-8",
    )

    write_preview_gifs(frames, actions)
    write_app_icon(raw)

    print(f"Wrote cat CharacterPack to {OUT}")
    for action, files in frames.items():
        print(f"  {action}: {len(files)} frames")


if __name__ == "__main__":
    main()
