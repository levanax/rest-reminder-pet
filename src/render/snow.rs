//! Snowfall ported from SnowApp.ts.

use macroquad::prelude::*;
use std::collections::HashMap;
use std::f32::consts::PI;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Layer {
    Far,
    Mid,
    Near,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    Dot,
    Crystal,
}

struct Flake {
    x: f32,
    y: f32,
    size: f32,
    speed: f32,
    drift: f32,
    swing: f32,
    phase: f32,
    spin: f32,
    rot: f32,
    opacity: f32,
    layer: Layer,
    kind: Kind,
}

pub struct SnowScene {
    flakes: Vec<Flake>,
    running: bool,
    w: f32,
    h: f32,
    wind: f32,
    wind_target: f32,
    sprites: HashMap<(Kind, u32), Texture2D>,
}

impl SnowScene {
    pub fn new() -> Self {
        Self {
            flakes: Vec::new(),
            running: false,
            w: 0.0,
            h: 0.0,
            wind: 0.0,
            wind_target: 0.0,
            sprites: HashMap::new(),
        }
    }

    pub fn set_active(&mut self, active: bool, w: f32, h: f32) {
        if active {
            self.start(w, h);
        } else {
            self.stop();
        }
    }

    pub fn running(&self) -> bool {
        self.running
    }

    fn start(&mut self, w: f32, h: f32) {
        self.w = w.max(1.0);
        self.h = h.max(1.0);
        self.flakes.clear();
        self.densify();
        for i in 0..self.flakes.len() {
            self.flakes[i] = self.make_flake(false);
        }
        self.wind = 0.0;
        self.wind_target = (macroquad::rand::gen_range(0.0, 1.0) - 0.5) * 0.6;
        self.running = true;
    }

    fn stop(&mut self) {
        self.running = false;
        self.flakes.clear();
    }

    fn densify(&mut self) {
        let area = (self.w * self.h).max(1.0);
        let target = ((area / 14000.0).floor() as usize).clamp(120, 520);
        while self.flakes.len() < target {
            let f = self.make_flake(true);
            self.flakes.push(f);
        }
        self.flakes.truncate(target);
    }

    fn pick_layer() -> Layer {
        let r = macroquad::rand::gen_range(0.0, 1.0);
        if r < 0.45 {
            Layer::Far
        } else if r < 0.8 {
            Layer::Mid
        } else {
            Layer::Near
        }
    }

    fn make_flake(&self, from_top: bool) -> Flake {
        let layer = Self::pick_layer();
        let (mut size, speed, opacity, kind) = match layer {
            Layer::Far => (
                1.4 + macroquad::rand::gen_range(0.0, 2.2),
                0.35 + macroquad::rand::gen_range(0.0, 0.55),
                0.25 + macroquad::rand::gen_range(0.0, 0.25),
                Kind::Dot,
            ),
            Layer::Mid => (
                3.2 + macroquad::rand::gen_range(0.0, 3.5),
                0.7 + macroquad::rand::gen_range(0.0, 1.1),
                0.45 + macroquad::rand::gen_range(0.0, 0.3),
                if macroquad::rand::gen_range(0.0, 1.0) < 0.55 {
                    Kind::Crystal
                } else {
                    Kind::Dot
                },
            ),
            Layer::Near => (
                5.5 + macroquad::rand::gen_range(0.0, 5.5),
                1.2 + macroquad::rand::gen_range(0.0, 1.6),
                0.65 + macroquad::rand::gen_range(0.0, 0.3),
                if macroquad::rand::gen_range(0.0, 1.0) < 0.85 {
                    Kind::Crystal
                } else {
                    Kind::Dot
                },
            ),
        };
        size = (size * 2.0_f32).round() / 2.0;
        Flake {
            x: macroquad::rand::gen_range(0.0, self.w),
            y: if from_top {
                -30.0 - macroquad::rand::gen_range(0.0, self.h * 0.4)
            } else {
                macroquad::rand::gen_range(0.0, self.h)
            },
            size,
            speed,
            drift: 0.15 + macroquad::rand::gen_range(0.0, 0.7),
            swing: 0.6 + macroquad::rand::gen_range(0.0, 1.4),
            phase: macroquad::rand::gen_range(0.0, PI * 2.0),
            spin: (macroquad::rand::gen_range(0.0, 1.0) - 0.5) * 0.04,
            rot: macroquad::rand::gen_range(0.0, PI * 2.0),
            opacity,
            layer,
            kind,
        }
    }

    fn sprite_key(kind: Kind, size: f32) -> (Kind, u32) {
        (kind, (size * 10.0) as u32)
    }

    fn ensure_sprite(&mut self, kind: Kind, size: f32) -> Texture2D {
        let key = Self::sprite_key(kind, size);
        if let Some(t) = self.sprites.get(&key) {
            return t.clone();
        }
        let img = match kind {
            Kind::Dot => make_dot_image(size * 0.55, size * 0.9),
            Kind::Crystal => make_crystal_image(size),
        };
        let tex = Texture2D::from_rgba8(img.width() as u16, img.height() as u16, &img);
        tex.set_filter(FilterMode::Linear);
        self.sprites.insert(key, tex.clone());
        tex
    }

    pub fn update_and_draw(&mut self, dt: f32) {
        if !self.running {
            return;
        }
        let t = get_time() as f32;
        if macroquad::rand::gen_range(0.0, 1.0) < 0.01 {
            self.wind_target = (macroquad::rand::gen_range(0.0, 1.0) - 0.5) * 1.2;
        }
        self.wind += (self.wind_target - self.wind) * (dt * 0.8).min(1.0);

        let order = [Layer::Far, Layer::Mid, Layer::Near];
        for layer in order {
            for i in 0..self.flakes.len() {
                if self.flakes[i].layer != layer {
                    continue;
                }
                {
                    let f = &mut self.flakes[i];
                    f.phase += dt * f.swing;
                    f.rot += f.spin;
                    f.y += f.speed * (1.0 + (t * 0.7 + f.phase).sin() * 0.08);
                    f.x += self.wind * (0.4 + f.size * 0.08)
                        + (t * 0.9 + f.phase).sin() * f.drift;
                    if f.y - f.size > self.h + 20.0 {
                        f.y = -20.0 - macroquad::rand::gen_range(0.0, 60.0);
                        f.x = macroquad::rand::gen_range(0.0, self.w);
                    }
                    if f.x < -40.0 {
                        f.x = self.w + 40.0;
                    }
                    if f.x > self.w + 40.0 {
                        f.x = -40.0;
                    }
                }
                let (kind, size, x, y, rot, opacity) = {
                    let f = &self.flakes[i];
                    (f.kind, f.size, f.x, f.y, f.rot, f.opacity)
                };
                let tex = self.ensure_sprite(kind, size);
                let tw = tex.width();
                let th = tex.height();
                let params = DrawTextureParams {
                    dest_size: Some(vec2(tw, th)),
                    rotation: if kind == Kind::Crystal { rot } else { 0.0 },
                    pivot: Some(vec2(x, y)),
                    ..Default::default()
                };
                draw_texture_ex(
                    &tex,
                    x - tw * 0.5,
                    y - th * 0.5,
                    Color::new(1.0, 1.0, 1.0, opacity),
                    params,
                );
            }
        }
    }
}

fn make_dot_image(radius: f32, glow: f32) -> image::RgbaImage {
    let pad = (radius * 3.0 + glow * 2.0).ceil() as i32;
    let dim = (pad * 2) as u32;
    let mut img = image::RgbaImage::new(dim, dim);
    let cx = pad as f32;
    let cy = pad as f32;
    let max_r = radius + glow;
    for y in 0..dim {
        for x in 0..dim {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d = (dx * dx + dy * dy).sqrt();
            if d > max_r {
                continue;
            }
            let t = (d / max_r).clamp(0.0, 1.0);
            // Approximate radial stops from SnowApp
            let (a, r, g, b) = if t < 0.35 {
                let u = t / 0.35;
                (
                    lerp(0.95, 0.75, u),
                    lerp(255.0, 235.0, u),
                    lerp(255.0, 245.0, u),
                    lerp(255.0, 255.0, u),
                )
            } else if t < 0.7 {
                let u = (t - 0.35) / 0.35;
                (
                    lerp(0.75, 0.25, u),
                    lerp(235.0, 200.0, u),
                    lerp(245.0, 220.0, u),
                    lerp(255.0, 245.0, u),
                )
            } else {
                let u = (t - 0.7) / 0.3;
                (
                    lerp(0.25, 0.0, u),
                    200.0,
                    220.0,
                    245.0,
                )
            };
            img.put_pixel(
                x,
                y,
                image::Rgba([r as u8, g as u8, b as u8, (a * 255.0) as u8]),
            );
        }
    }
    img
}

fn make_crystal_image(size: f32) -> image::RgbaImage {
    let pad = (size * 2.4).ceil() as i32;
    let dim = (pad * 2).max(4) as u32;
    let mut img = image::RgbaImage::new(dim, dim);
    let cx = pad as f32;
    let cy = pad as f32;

    // Soft bloom
    let bloom_r = size * 1.35;
    for y in 0..dim {
        for x in 0..dim {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d = (dx * dx + dy * dy).sqrt();
            if d < bloom_r {
                let t = d / bloom_r;
                let a = if t < 0.55 {
                    lerp(0.55, 0.18, t / 0.55)
                } else {
                    lerp(0.18, 0.0, (t - 0.55) / 0.45)
                };
                let px = img.get_pixel(x, y).0;
                let na = (a * 255.0) as u8;
                if na > px[3] {
                    img.put_pixel(x, y, image::Rgba([240, 248, 255, na]));
                }
            }
        }
    }

    let line_w = (size * 0.11).max(0.8);
    for i in 0..6 {
        let angle = (PI / 3.0) * i as f32 - PI / 2.0;
        draw_line_aa(
            &mut img,
            cx,
            cy,
            cx + angle.cos() * size,
            cy + angle.sin() * size,
            line_w,
            [255, 255, 255, 235],
        );
        for &(bf, bl) in &[(0.42, 0.28), (0.72, 0.22)] {
            let bx = cx + angle.cos() * size * bf;
            let by = cy + angle.sin() * size * bf;
            let perp = angle + PI / 2.0;
            let len = size * bl;
            let tip_x = bx + angle.cos() * size * 0.18;
            let tip_y = by + angle.sin() * size * 0.18;
            draw_line_aa(
                &mut img,
                bx,
                by,
                tip_x - perp.cos() * len,
                tip_y - perp.sin() * len,
                line_w * 0.85,
                [255, 255, 255, 220],
            );
            draw_line_aa(
                &mut img,
                bx,
                by,
                tip_x + perp.cos() * len,
                tip_y + perp.sin() * len,
                line_w * 0.85,
                [255, 255, 255, 220],
            );
        }
    }
    // Center hub
    let hub = (size * 0.14).max(1.2);
    fill_circle(&mut img, cx, cy, hub, [245, 250, 255, 230]);
    img
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn draw_line_aa(img: &mut image::RgbaImage, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, rgba: [u8; 4]) {
    let steps = ((x1 - x0).hypot(y1 - y0) * 2.0).ceil().max(1.0) as i32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let x = lerp(x0, x1, t);
        let y = lerp(y0, y1, t);
        fill_circle(img, x, y, width * 0.5, rgba);
    }
}

fn fill_circle(img: &mut image::RgbaImage, cx: f32, cy: f32, r: f32, rgba: [u8; 4]) {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let r2 = r * r;
    let min_x = (cx - r - 1.0).floor().max(0.0) as i32;
    let max_x = (cx + r + 1.0).ceil().min(w as f32 - 1.0) as i32;
    let min_y = (cy - r - 1.0).floor().max(0.0) as i32;
    let max_y = (cy + r + 1.0).ceil().min(h as f32 - 1.0) as i32;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            if dx * dx + dy * dy <= r2 {
                let prev = img.get_pixel(x as u32, y as u32).0;
                if rgba[3] >= prev[3] {
                    img.put_pixel(x as u32, y as u32, image::Rgba(rgba));
                }
            }
        }
    }
}
