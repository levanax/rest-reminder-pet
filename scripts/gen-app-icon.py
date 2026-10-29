"""Generate GitHub-Desktop-style black rounded-square + white sitting cat silhouette."""
from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "src-tauri" / "icons" / "icon.png"
SIZE = 1024
RADIUS = 220
BG = (13, 17, 23, 255)
FG = (255, 255, 255, 255)


def rounded_rect_mask(size: int, radius: int) -> Image.Image:
    m = Image.new("L", (size, size), 0)
    d = ImageDraw.Draw(m)
    d.rounded_rectangle((0, 0, size - 1, size - 1), radius=radius, fill=255)
    return m


def draw_sitting_cat(draw: ImageDraw.ImageDraw) -> None:
    """Continuous side-sitting cat facing left (original mark, not Octocat)."""
    # Single outline path in icon coordinates (1024 canvas)
    cat = [
        (300, 390),  # neck/chest
        (265, 300),
        (250, 220),  # forehead
        (235, 155),  # left ear tip
        (295, 210),
        (340, 195),
        (385, 145),  # right ear tip
        (400, 220),
        (430, 280),  # muzzle
        (445, 350),
        (470, 400),  # shoulder
        (560, 370),
        (640, 390),  # back
        (700, 430),
        (735, 500),  # haunch top
        (755, 580),
        (730, 680),
        (680, 740),  # hind
        (600, 770),
        (520, 780),  # belly / paws
        (430, 775),
        (360, 750),
        (320, 700),
        (300, 620),
        (290, 520),
        (295, 450),
    ]
    draw.polygon(cat, fill=FG)

    # Tail as attached loop (overlaps haunch so no gap)
    tail = [
        (700, 470),
        (780, 430),
        (845, 470),
        (860, 560),
        (820, 640),
        (750, 680),
        (720, 640),
        (760, 580),
        (755, 520),
        (720, 490),
    ]
    draw.polygon(tail, fill=FG)


def main() -> None:
    layer = Image.new("RGBA", (SIZE, SIZE), BG)
    mask = rounded_rect_mask(SIZE, RADIUS)
    base = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    base.paste(layer, (0, 0), mask)
    draw = ImageDraw.Draw(base)
    draw_sitting_cat(draw)
    OUT.parent.mkdir(parents=True, exist_ok=True)
    base.save(OUT, "PNG")
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
