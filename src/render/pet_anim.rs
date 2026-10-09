//! Pet pose timelines ported from PetApp.ts.

use crate::overlay_win::RectI;
use crate::render::assets::LoadedPack;
use crate::timer::{Phase, PetEvent};
use macroquad::prelude::*;
use std::f32::consts::PI;

#[derive(Clone, Copy)]
pub struct Pose {
    pub y: f32,
    pub x: f32,
    pub rot: f32,
    pub sx: f32,
    pub sy: f32,
    pub opacity: f32,
    pub frame_t: f32,
}

enum AnimKind {
    Timeline {
        action: String,
        duration_ms: f32,
        start: f64,
        frames: Vec<String>,
        kind: TimelineKind,
    },
    LookLoop {
        frames: Vec<String>,
        next_blink_at: f64,
        blink_until: f64,
    },
}

#[derive(Clone, Copy)]
enum TimelineKind {
    JumpUp,
    Crawl,
    LookDownOnce,
    HappyClimb { from_y: f32 },
    SneakPeek,
}

pub struct PetAnimator {
    anim: Option<AnimKind>,
    current_action: Option<String>,
    pose: Pose,
    frame_file: Option<String>,
    pending_done: Option<String>,
    /// Last pet Y (monitor-local) for happyClimb start.
    last_y: f32,
}

impl PetAnimator {
    pub fn new() -> Self {
        Self {
            anim: None,
            current_action: None,
            pose: Pose {
                y: -160.0,
                x: 0.0,
                rot: 0.0,
                sx: 1.0,
                sy: 1.0,
                opacity: 0.0,
                frame_t: 0.0,
            },
            frame_file: None,
            pending_done: None,
            last_y: 0.0,
        }
    }

    pub fn take_done(&mut self) -> Option<String> {
        self.pending_done.take()
    }

    pub fn clear(&mut self) {
        self.anim = None;
        self.current_action = None;
        self.pose.opacity = 0.0;
        self.frame_file = None;
    }

    pub fn on_event(&mut self, ev: &PetEvent, pack: &LoadedPack, monitor_h: f32) {
        let Some(action) = ev.action.as_deref() else {
            self.clear();
            return;
        };

        if ev.phase == Phase::Observing && action == "lookDown" {
            if self.current_action.as_deref() == Some("lookDown") {
                if matches!(self.anim, Some(AnimKind::LookLoop { .. })) {
                    return;
                }
            }
            self.start_look_loop(pack);
            return;
        }

        if self.current_action.as_deref() == Some(action) && ev.phase == Phase::Observing {
            return;
        }

        match action {
            "jumpUp" => self.start_jump_up(pack, monitor_h),
            "sneakPeek" => self.start_sneak_peek(pack),
            "crawl" => self.start_crawl(pack),
            "lookDown" => self.start_look_down_once(pack, monitor_h),
            "happyClimb" => self.start_happy_climb(pack),
            other => {
                self.pending_done = Some(other.to_string());
            }
        }
    }

    fn frames_for(pack: &LoadedPack, action: &str) -> Vec<String> {
        pack.action(action)
            .map(|a| a.frames.clone())
            .unwrap_or_default()
    }

    fn start_jump_up(&mut self, pack: &LoadedPack, monitor_h: f32) {
        let def = pack
            .action("jumpUp")
            .or_else(|| pack.action("fall"))
            .or_else(|| pack.action("happyClimb"));
        let Some(def) = def.filter(|d| !d.frames.is_empty()) else {
            self.pending_done = Some("jumpUp".into());
            return;
        };
        self.current_action = Some("jumpUp".into());
        self.frame_file = None;
        self.anim = Some(AnimKind::Timeline {
            action: "jumpUp".into(),
            duration_ms: 1300.0,
            start: get_time(),
            frames: def.frames.clone(),
            kind: TimelineKind::JumpUp,
        });
        let _ = monitor_h;
    }

    fn start_crawl(&mut self, pack: &LoadedPack) {
        let frames = Self::frames_for(pack, "crawl");
        if frames.is_empty() {
            self.pending_done = Some("crawl".into());
            return;
        }
        self.current_action = Some("crawl".into());
        self.frame_file = None;
        self.anim = Some(AnimKind::Timeline {
            action: "crawl".into(),
            duration_ms: 1400.0,
            start: get_time(),
            frames,
            kind: TimelineKind::Crawl,
        });
    }

    fn start_look_down_once(&mut self, pack: &LoadedPack, _monitor_h: f32) {
        let frames = Self::frames_for(pack, "lookDown");
        if frames.is_empty() {
            self.pending_done = Some("lookDown".into());
            return;
        }
        self.current_action = Some("lookDown".into());
        self.frame_file = None;
        self.anim = Some(AnimKind::Timeline {
            action: "lookDown".into(),
            duration_ms: 1000.0,
            start: get_time(),
            frames,
            kind: TimelineKind::LookDownOnce,
        });
    }

    fn start_happy_climb(&mut self, pack: &LoadedPack) {
        let frames = Self::frames_for(pack, "happyClimb");
        if frames.is_empty() {
            self.pending_done = Some("happyClimb".into());
            return;
        }
        let from_y = if self.pose.opacity > 0.01 {
            self.pose.y
        } else {
            self.last_y
        };
        self.current_action = Some("happyClimb".into());
        self.frame_file = None;
        self.anim = Some(AnimKind::Timeline {
            action: "happyClimb".into(),
            duration_ms: 1200.0,
            start: get_time(),
            frames,
            kind: TimelineKind::HappyClimb { from_y },
        });
    }

    fn start_sneak_peek(&mut self, pack: &LoadedPack) {
        let def = pack
            .action("sneakPeek")
            .or_else(|| pack.action("lookDown"));
        let Some(def) = def.filter(|d| !d.frames.is_empty()) else {
            self.pending_done = Some("sneakPeek".into());
            return;
        };
        self.current_action = Some("sneakPeek".into());
        self.frame_file = None;
        self.anim = Some(AnimKind::Timeline {
            action: "sneakPeek".into(),
            duration_ms: 2800.0,
            start: get_time(),
            frames: def.frames.clone(),
            kind: TimelineKind::SneakPeek,
        });
    }

    fn start_look_loop(&mut self, pack: &LoadedPack) {
        let frames = Self::frames_for(pack, "lookDown");
        if frames.is_empty() {
            return;
        }
        self.current_action = Some("lookDown".into());
        self.frame_file = None;
        let now = get_time() * 1000.0;
        self.anim = Some(AnimKind::LookLoop {
            frames,
            next_blink_at: now + 2800.0 + f64::from(macroquad::rand::gen_range(0.0_f32, 1400.0)),
            blink_until: 0.0,
        });
    }

    pub fn update(&mut self, monitor_h: f32) {
        let now_s = get_time();
        let now_ms = now_s * 1000.0;

        match &mut self.anim {
            Some(AnimKind::Timeline {
                action,
                duration_ms,
                start,
                frames,
                kind,
            }) => {
                let t = (((now_s - *start) * 1000.0 / f64::from(*duration_ms)) as f32).clamp(0.0, 1.0);
                let pose = sample_timeline(*kind, t, monitor_h, now_s);
                let frames_clone = frames.clone();
                let done_action = action.clone();
                self.pose = pose;
                self.last_y = pose.y;
                self.apply_frame(&frames_clone, pose.frame_t);
                if t >= 1.0 {
                    self.pose.opacity = 0.0;
                    self.anim = None;
                    self.pending_done = Some(done_action);
                }
            }
            Some(AnimKind::LookLoop {
                frames,
                next_blink_at,
                blink_until,
            }) => {
                if now_ms >= *next_blink_at {
                    *blink_until = now_ms + 120.0;
                    *next_blink_at = now_ms + 2800.0 + f64::from(macroquad::rand::gen_range(0.0_f32, 1400.0));
                }
                let breath = (now_ms / 900.0).sin() as f32;
                let scan = (now_ms / 1800.0).sin() as f32;
                let blinking = now_ms < *blink_until;
                let frame_t = if blinking {
                    0.92_f32
                } else {
                    0.15 + (0.5 + 0.5 * (now_ms / 2200.0).sin() as f32) * 0.55
                };
                let y = y_from_pct(6.0, monitor_h);
                self.pose = Pose {
                    y,
                    x: scan * 4.0,
                    rot: scan * 3.0,
                    sx: 1.0 - breath * 0.012,
                    sy: 1.0 + breath * 0.025,
                    opacity: 1.0,
                    frame_t,
                };
                self.last_y = y;
                let frames = frames.clone();
                self.apply_frame(&frames, frame_t);
            }
            None => {}
        }
    }

    fn apply_frame(&mut self, frames: &[String], frame_t: f32) {
        if frames.is_empty() {
            return;
        }
        let idx = ((frame_t * frames.len() as f32 * 0.999).floor() as usize).min(frames.len() - 1);
        self.frame_file = Some(frames[idx].clone());
    }

    pub fn draw(&self, pack: &LoadedPack, monitor: RectI, desktop: RectI) {
        if self.pose.opacity <= 0.01 {
            return;
        }
        let Some(file) = self.frame_file.as_deref() else {
            return;
        };
        let Some(tex) = pack.frame_tex(file) else {
            return;
        };

        let size = 128.0;
        let origin_x = (monitor.x - desktop.x) as f32 + monitor.w as f32 * 0.5 + self.pose.x;
        let origin_y = (monitor.y - desktop.y) as f32 + self.pose.y;

        let params = DrawTextureParams {
            dest_size: Some(vec2(size * self.pose.sx, size * self.pose.sy)),
            rotation: self.pose.rot * PI / 180.0,
            pivot: Some(vec2(origin_x, origin_y + size * 0.4)),
            ..Default::default()
        };

        draw_texture_ex(
            tex,
            origin_x - size * self.pose.sx * 0.5,
            origin_y,
            Color::new(1.0, 1.0, 1.0, self.pose.opacity),
            params,
        );
    }

    pub fn visible(&self) -> bool {
        self.pose.opacity > 0.01 || self.anim.is_some()
    }
}

fn y_from_pct(pct: f32, h: f32) -> f32 {
    ((h * pct) / 100.0).clamp(0.0, h - 128.0)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

fn ease_out_quad(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

fn ease_in_quad(t: f32) -> f32 {
    t * t
}

fn ease_in_out_quad(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

fn smoothstep(t: f32) -> f32 {
    let x = t.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn paused_progress(t: f32, pauses: &[f32], pause_width: f32) -> f32 {
    let samples = 64usize;
    let mut acc = 0.0f32;
    let mut weights = Vec::with_capacity(samples + 1);
    for i in 0..=samples {
        let u = i as f32 / samples as f32;
        let mut w = 1.0f32;
        for &p in pauses {
            let d = (u - p).abs();
            if d < pause_width {
                w *= d / pause_width;
            }
        }
        let w = w.max(0.05);
        weights.push(w);
        if i > 0 {
            acc += w;
        }
    }
    let target = t * acc;
    let mut sum = 0.0f32;
    for i in 1..=samples {
        sum += weights[i];
        if sum >= target {
            let prev = sum - weights[i];
            let local = (target - prev) / weights[i];
            return ((i as f32 - 1.0 + local) / samples as f32).clamp(0.0, 1.0);
        }
    }
    1.0
}

fn sample_timeline(kind: TimelineKind, t: f32, monitor_h: f32, now_s: f64) -> Pose {
    match kind {
        TimelineKind::JumpUp => {
            let from = monitor_h - 20.0;
            let to = -170.0;
            let crouch_y = from + 8.0;
            let (mut y, mut sx, mut sy, mut opacity, mut frame_t) = (0.0, 1.0, 1.0, 1.0, t);
            if t < 0.15 {
                let u = t / 0.15;
                y = lerp(from, crouch_y, u);
                sx = lerp(1.0, 1.1, u);
                sy = lerp(1.0, 0.88, u);
                frame_t = u * 0.16;
            } else if t < 0.55 {
                let u = (t - 0.15) / 0.4;
                let e = ease_out_cubic(u);
                y = lerp(crouch_y, lerp(from, to, 0.72), e);
                sx = lerp(1.1, 0.92, u);
                sy = lerp(0.88, 1.12, u);
                frame_t = 0.16 + u * 0.4;
            } else if t < 0.85 {
                let u = (t - 0.55) / 0.3;
                y = lerp(lerp(from, to, 0.72), to, ease_out_cubic(u));
                sx = lerp(0.92, 1.02, u);
                sy = lerp(1.12, 0.97, u);
                frame_t = 0.56 + u * 0.28;
            } else {
                let u = (t - 0.85) / 0.15;
                y = to;
                sx = lerp(1.02, 1.0, u);
                sy = lerp(0.97, 1.04, (u * PI).sin());
                opacity = lerp(1.0, 0.0, u);
                frame_t = 0.84 + u * 0.16;
            }
            let rot = if t < 0.15 {
                0.0
            } else if t < 0.55 {
                lerp(0.0, -4.0, (t - 0.15) / 0.4)
            } else {
                lerp(-4.0, 0.0, (t - 0.55) / 0.45)
            };
            Pose {
                y,
                x: 0.0,
                rot,
                sx,
                sy,
                opacity,
                frame_t: frame_t.min(1.0),
            }
        }
        TimelineKind::Crawl => {
            let from = -150.0;
            let to = 8.0;
            let p = smoothstep(paused_progress(t, &[0.28, 0.52, 0.76], 0.07));
            let sway = (p * PI * 3.0).sin();
            Pose {
                y: lerp(from, to, p),
                x: sway * 10.0,
                rot: sway * 8.0,
                sx: 1.0,
                sy: 1.0 + (p * PI * 3.0).sin().abs() * 0.03,
                opacity: 1.0,
                frame_t: p,
            }
        }
        TimelineKind::LookDownOnce => {
            let y = y_from_pct(6.0, monitor_h);
            let sway = (t * PI * 2.0).sin() * (1.0 - t);
            Pose {
                y,
                x: sway * 3.0,
                rot: sway * 2.0,
                sx: 1.0,
                sy: 1.0,
                opacity: 1.0,
                frame_t: t.min(0.8),
            }
        }
        TimelineKind::HappyClimb { from_y } => {
            let to = -170.0;
            let p = ease_in_out_quad(paused_progress(t, &[0.35, 0.65], 0.07));
            let sway = (p * PI * 2.5).sin();
            let fade = if t > 0.75 { (t - 0.75) / 0.25 } else { 0.0 };
            Pose {
                y: lerp(from_y, to, p),
                x: sway * 9.0,
                rot: sway * 9.0,
                sx: 1.0,
                sy: 1.0 + sway.abs() * 0.04,
                opacity: 1.0 - fade,
                frame_t: p,
            }
        }
        TimelineKind::SneakPeek => {
            let from = -110.0;
            let peek = -28.0;
            let hide = -130.0;
            let total = 2800.0;
            let out_end = 900.0 / total;
            let hold_end = (900.0 + 1200.0) / total;
            let (y, mut frame_t) = if t < out_end {
                let u = t / out_end;
                (lerp(from, peek, ease_out_quad(u)), (u * 0.75).min(1.0))
            } else if t < hold_end {
                let u = (t - out_end) / (hold_end - out_end);
                (peek, 0.5 + (u * PI).sin() * 0.35)
            } else {
                let u = (t - hold_end) / (1.0 - hold_end);
                (lerp(peek, hide, ease_in_quad(u)), (0.5 - u * 0.5).max(0.0))
            };
            let breath_t = now_s as f32 * 1000.0 / 900.0;
            let breath = breath_t.sin();
            let micro = t >= out_end && t < hold_end;
            let opacity = if t >= 0.97 {
                lerp(1.0, 0.0, (t - 0.97) / 0.03)
            } else {
                1.0
            };
            frame_t = frame_t.clamp(0.0, 1.0);
            Pose {
                y,
                x: if micro {
                    (breath_t * 0.7).sin() * 3.0
                } else {
                    0.0
                },
                rot: if micro {
                    (breath_t * 0.55).sin() * 2.5
                } else {
                    0.0
                },
                sx: 1.0 - if micro { breath * 0.012 } else { 0.0 },
                sy: 1.0 + if micro { breath * 0.025 } else { 0.0 },
                opacity,
                frame_t,
            }
        }
    }
}
