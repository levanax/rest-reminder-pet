"""Build default CharacterPack frames from a public-domain Ragdoll photo."""
from __future__ import annotations

import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageEnhance, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "src-tauri" / "characters" / "default-source" / "ragdoll-source.jpg"
OUT = ROOT / "src-tauri" / "characters" / "default"
SIZE = 128


def soft_ellipse_cutout(im: Image.Image) -> Image.Image:
    """Keep photo colors; fade edges with an elliptical alpha mask."""
    im = im.convert("RGBA")
    w, h = im.size
    mask = Image.new("L", (w, h), 0)
    draw = ImageDraw.Draw(mask)
    # Inset ellipse so whiskers/ears aren't hard-cropped
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


def scale_about_center(im: Image.Image, factor: float) -> Image.Image:
    w, h = im.size
    nw, nh = max(1, int(w * factor)), max(1, int(h * factor))
    resized = im.resize((nw, nh), Image.Resampling.LANCZOS)
    out = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    out.paste(resized, ((w - nw) // 2, (h - nh) // 2), resized)
    return out


def bright(im: Image.Image, factor: float) -> Image.Image:
    rgb = ImageEnhance.Brightness(im.convert("RGB")).enhance(factor)
    r, g, b = rgb.split()
    return Image.merge("RGBA", (r, g, b, im.split()[-1]))


def main() -> None:
    if not SRC.exists():
        raise SystemExit(f"missing source image: {SRC}")

    OUT.mkdir(parents=True, exist_ok=True)
    for p in OUT.glob("*.png"):
        p.unlink()

    raw = Image.open(SRC).convert("RGB")
    w, h = raw.size
    # Tight crop on head + chest (photo is portrait, cat in lower-center)
    face = raw.crop((int(w * 0.08), int(h * 0.18), int(w * 0.92), int(h * 0.72)))
    # Mild enhance for screen readability
    face = ImageEnhance.Contrast(face).enhance(1.06)
    face = ImageEnhance.Color(face).enhance(1.04)
    base = to_sprite(soft_ellipse_cutout(face))

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

    for i, (dx, dy, rot) in enumerate([(-2, 2, -3), (0, 0, 0), (2, 3, 3)], 1):
        save(f"crawl_{i:02d}.png", rotate(shift(base, dx, dy), rot), "crawl")

    for i, dy in enumerate([4, 8], 1):
        save(f"look_{i:02d}.png", shift(scale_about_center(base, 1.02), 0, dy), "lookDown")

    for i, dy in enumerate([6, 10], 1):
        save(f"sneak_{i:02d}.png", shift(scale_about_center(base, 0.96), 0, dy), "sneakPeek")

    for i, (sy, dy) in enumerate([(1.06, 3), (1.0, 0), (0.94, -2)], 1):
        save(f"jump_{i:02d}.png", shift(scale_about_center(base, sy), 0, dy), "jumpUp")

    for i, (dx, dy, rot) in enumerate([(-3, -2, 4), (3, -4, -4)], 1):
        save(f"climb_{i:02d}.png", rotate(shift(bright(base, 1.04), dx, dy), rot), "happyClimb")

    for i, (rot, dy) in enumerate([(12, 2), (-8, 6), (18, 10)], 1):
        save(f"fall_{i:02d}.png", rotate(shift(base, 0, dy), rot), "fall")

    manifest = {
        "name": "default",
        "variant": "ragdoll-photo",
        "frameSize": {"w": SIZE, "h": SIZE},
        "actions": {
            "crawl": {"fps": 8, "frames": frames["crawl"]},
            "lookDown": {"fps": 4, "frames": frames["lookDown"]},
            "sneakPeek": {"fps": 4, "frames": frames["sneakPeek"]},
            "jumpUp": {"fps": 10, "frames": frames["jumpUp"]},
            "happyClimb": {"fps": 8, "frames": frames["happyClimb"]},
            "fall": {"fps": 10, "frames": frames["fall"]},
        },
    }
    (OUT / "manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")

    (ROOT / "src-tauri" / "characters" / "default-source" / "ATTRIBUTION.md").write_text(
        """# Default character source

- Image: [Ragdoll Blue Colourpoint.jpg](https://commons.wikimedia.org/wiki/File:Ragdoll_Blue_Colourpoint.jpg)
- Subject: \"Mork\"; Ragdoll - Blue Colorpoint
- Author: CX23882-19 (English Wikipedia)
- License: Public domain (released by the author)

Processed into 128×128 elliptical cutout animation frames for Rest Reminder Pet.
""",
        encoding="utf-8",
    )
    print(f"Wrote ragdoll CharacterPack to {OUT}")


if __name__ == "__main__":
    main()
