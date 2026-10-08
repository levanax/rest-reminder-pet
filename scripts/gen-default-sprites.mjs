import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const outDir = join(__dirname, "..", "src-tauri", "characters", "default");
mkdirSync(outDir, { recursive: true });

const W = 128;
const H = 128;

function crc32(buf) {
  let c = ~0;
  for (let i = 0; i < buf.length; i++) {
    c ^= buf[i];
    for (let k = 0; k < 8; k++) c = c & 1 ? (c >>> 1) ^ 0xedb88320 : c >>> 1;
  }
  return ~c >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const typeBuf = Buffer.from(type);
  const crcBuf = Buffer.alloc(4);
  crcBuf.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])));
  return Buffer.concat([len, typeBuf, data, crcBuf]);
}

function pngRGBA(pixels) {
  const raw = Buffer.alloc((W * 4 + 1) * H);
  for (let y = 0; y < H; y++) {
    raw[y * (W * 4 + 1)] = 0;
    pixels.copy(raw, y * (W * 4 + 1) + 1, y * W * 4, (y + 1) * W * 4);
  }
  const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(W, 0);
  ihdr.writeUInt32BE(H, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  return Buffer.concat([
    sig,
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

function setPixel(px, x, y, r, g, b, a = 255) {
  if (x < 0 || y < 0 || x >= W || y >= H) return;
  const i = (y * W + x) * 4;
  px[i] = r;
  px[i + 1] = g;
  px[i + 2] = b;
  px[i + 3] = a;
}

function fillCircle(px, cx, cy, rad, r, g, b, a = 255) {
  for (let y = -rad; y <= rad; y++) {
    for (let x = -rad; x <= rad; x++) {
      if (x * x + y * y <= rad * rad) setPixel(px, cx + x, cy + y, r, g, b, a);
    }
  }
}

function drawCat(px, offsetY, pose) {
  const bodyR = 70,
    bodyG = 120,
    bodyB = 160;
  const earR = 55,
    earG = 95,
    earB = 130;
  const eye = [40, 40, 40];
  const cx = 64;
  const cy = 70 + offsetY;

  // ears
  fillCircle(px, cx - 22, cy - 28, 12, earR, earG, earB);
  fillCircle(px, cx + 22, cy - 28, 12, earR, earG, earB);
  fillCircle(px, cx - 22, cy - 28, 6, 240, 180, 190);
  fillCircle(px, cx + 22, cy - 28, 6, 240, 180, 190);

  // head / body
  fillCircle(px, cx, cy, 28, bodyR, bodyG, bodyB);
  fillCircle(px, cx, cy + 30, 22, bodyR, bodyG, bodyB);

  // eyes look
  const eyeY = pose === "look" || pose === "sneak" ? cy + 4 : cy - 4;
  fillCircle(px, cx - 10, eyeY, 4, ...eye);
  fillCircle(px, cx + 10, eyeY, 4, ...eye);
  if (pose === "happy") {
    fillCircle(px, cx - 10, eyeY, 4, bodyR, bodyG, bodyB);
    fillCircle(px, cx + 10, eyeY, 4, bodyR, bodyG, bodyB);
    for (let x = -6; x <= 6; x++) {
      setPixel(px, cx - 10 + x, eyeY + Math.abs(x) - 2, ...eye);
      setPixel(px, cx + 10 + x, eyeY + Math.abs(x) - 2, ...eye);
    }
  }

  // nose
  fillCircle(px, cx, cy + 6, 3, 240, 140, 150);

  // paws / limbs by pose
  if (pose === "crawl" || pose === "climb") {
    fillCircle(px, cx - 18, cy + 18 + (offsetY % 3), 7, earR, earG, earB);
    fillCircle(px, cx + 18, cy + 20 - (offsetY % 3), 7, earR, earG, earB);
  }
  if (pose === "fall") {
    fillCircle(px, cx - 24, cy - 6, 6, earR, earG, earB);
    fillCircle(px, cx + 24, cy - 4, 6, earR, earG, earB);
    fillCircle(px, cx - 10, cy + 40, 6, earR, earG, earB);
    fillCircle(px, cx + 10, cy + 42, 6, earR, earG, earB);
  }
}

function makeFrame(pose, frameIndex) {
  const px = Buffer.alloc(W * H * 4);
  const bob = pose === "fall" ? frameIndex * 2 : (frameIndex - 1) * 2;
  const mapped =
    pose === "lookDown" || pose === "sneakPeek"
      ? pose === "sneakPeek"
        ? "sneak"
        : "look"
      : pose === "happyClimb" || pose === "jumpUp"
        ? pose === "jumpUp"
          ? "fall"
          : "happy"
        : pose === "fall"
          ? "fall"
          : pose === "crawl"
            ? "crawl"
            : "climb";
  drawCat(px, bob, mapped);
  return pngRGBA(px);
}

const actions = {
  crawl: ["crawl_01.png", "crawl_02.png", "crawl_03.png"],
  lookDown: ["look_01.png", "look_02.png"],
  sneakPeek: ["sneak_01.png", "sneak_02.png"],
  jumpUp: ["jump_01.png", "jump_02.png", "jump_03.png"],
  happyClimb: ["climb_01.png", "climb_02.png"],
  fall: ["fall_01.png", "fall_02.png", "fall_03.png"],
};

for (const [action, files] of Object.entries(actions)) {
  files.forEach((file, i) => {
    writeFileSync(join(outDir, file), makeFrame(action, i));
  });
}

const manifest = {
  name: "default",
  frameSize: { w: 128, h: 128 },
  actions: {
    crawl: { fps: 8, frames: actions.crawl },
    lookDown: { fps: 4, frames: actions.lookDown },
    sneakPeek: { fps: 4, frames: actions.sneakPeek },
    jumpUp: { fps: 10, frames: actions.jumpUp },
    happyClimb: { fps: 8, frames: actions.happyClimb },
    fall: { fps: 10, frames: actions.fall },
  },
};

writeFileSync(join(outDir, "manifest.json"), JSON.stringify(manifest, null, 2));
console.log("Wrote default CharacterPack to", outDir);
