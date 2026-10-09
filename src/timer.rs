use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    BootIntro,
    Working,
    Paused,
    SneakPeek,
    Reminding,
    Observing,
    HappyExit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetEvent {
    pub phase: Phase,
    pub action: Option<String>,
    pub character_pack: String,
}

pub struct TimerState {
    pub phase: Phase,
    pub work_minutes: u32,
    pub observation_seconds: u32,
    pub character_pack: String,
    pub work_elapsed: Duration,
    pub segment_started: Option<Instant>,
    pub observation_started: Option<Instant>,
    pub last_activity: Option<Instant>,
    pub next_sneak_at: Instant,
}

fn random_range_secs(min: u64, max: u64) -> u64 {
    let span = max.saturating_sub(min).saturating_add(1);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(1);
    min + (nanos % span)
}

impl TimerState {
    pub fn new(work_minutes: u32, observation_seconds: u32, character_pack: String) -> Self {
        Self {
            phase: Phase::BootIntro,
            work_minutes,
            observation_seconds,
            character_pack,
            work_elapsed: Duration::ZERO,
            segment_started: None,
            observation_started: None,
            last_activity: None,
            next_sneak_at: Instant::now() + Duration::from_secs(3600),
        }
    }

    pub fn start_boot_intro(&mut self) {
        self.phase = Phase::BootIntro;
        self.segment_started = None;
    }

    pub fn end_boot_intro(&mut self) {
        if self.phase != Phase::BootIntro {
            return;
        }
        self.phase = Phase::Working;
        self.work_elapsed = Duration::ZERO;
        self.segment_started = Some(Instant::now());
        self.schedule_next_sneak();
    }

    pub fn schedule_next_sneak(&mut self) {
        self.next_sneak_at = Instant::now() + Duration::from_secs(random_range_secs(90, 240));
    }

    pub fn apply_config(&mut self, work_minutes: u32, observation_seconds: u32, pack: String) {
        self.work_minutes = work_minutes;
        self.observation_seconds = observation_seconds;
        self.character_pack = pack;
    }

    fn flush_work(&mut self) {
        if matches!(self.phase, Phase::Working | Phase::SneakPeek) {
            if let Some(start) = self.segment_started.take() {
                self.work_elapsed += start.elapsed();
            }
        }
    }

    pub fn pause(&mut self) {
        if matches!(self.phase, Phase::Working | Phase::SneakPeek) {
            self.flush_work();
            self.phase = Phase::Paused;
            self.schedule_next_sneak();
        }
    }

    pub fn resume(&mut self) {
        if self.phase == Phase::Paused {
            self.phase = Phase::Working;
            self.segment_started = Some(Instant::now());
        }
    }

    pub fn dismiss_to_working(&mut self, was_paused: bool) {
        self.phase = if was_paused {
            Phase::Paused
        } else {
            Phase::Working
        };
        self.observation_started = None;
        self.last_activity = None;
        self.work_elapsed = Duration::ZERO;
        self.segment_started = if was_paused {
            None
        } else {
            Some(Instant::now())
        };
        self.schedule_next_sneak();
    }

    pub fn start_sneak_peek(&mut self) {
        if self.phase != Phase::Working {
            return;
        }
        self.phase = Phase::SneakPeek;
    }

    pub fn end_sneak_peek(&mut self) {
        if self.phase == Phase::SneakPeek {
            self.phase = Phase::Working;
            self.schedule_next_sneak();
        }
    }

    pub fn start_reminding(&mut self) {
        self.flush_work();
        self.phase = Phase::Reminding;
        self.segment_started = None;
        self.observation_started = None;
        self.last_activity = None;
    }

    pub fn enter_observing(&mut self) {
        self.phase = Phase::Observing;
        let now = Instant::now();
        self.observation_started = Some(now);
        self.last_activity = None;
    }

    /// 观察期内有键鼠：重置「连续空闲」计时，必须重新凑满 observation_seconds。
    pub fn note_activity(&mut self) {
        if self.phase == Phase::Observing {
            self.observation_started = Some(Instant::now());
            self.last_activity = Some(Instant::now());
        }
    }

    pub fn start_happy_exit(&mut self) {
        self.phase = Phase::HappyExit;
    }

    pub fn work_deadline(&self) -> Duration {
        Duration::from_secs(self.work_minutes as u64 * 60)
    }

    pub fn current_work_elapsed(&self) -> Duration {
        let mut total = self.work_elapsed;
        if matches!(self.phase, Phase::Working | Phase::SneakPeek) {
            if let Some(start) = self.segment_started {
                total += start.elapsed();
            }
        }
        total
    }

    fn remaining_work(&self) -> Duration {
        self.work_deadline()
            .saturating_sub(self.current_work_elapsed())
    }

    pub fn tick(&mut self) -> Option<PhaseTransition> {
        match self.phase {
            Phase::Working => {
                if self.current_work_elapsed() >= self.work_deadline() {
                    return Some(PhaseTransition::EnterReminding);
                }
                if Instant::now() >= self.next_sneak_at
                    && self.remaining_work() > Duration::from_secs(45)
                {
                    return Some(PhaseTransition::EnterSneakPeek);
                }
            }
            Phase::SneakPeek => {
                if self.current_work_elapsed() >= self.work_deadline() {
                    return Some(PhaseTransition::EnterReminding);
                }
            }
            Phase::Observing => {
                if let Some(started) = self.observation_started {
                    if started.elapsed() >= Duration::from_secs(self.observation_seconds as u64) {
                        self.start_happy_exit();
                        return Some(PhaseTransition::EnterHappyExit);
                    }
                }
            }
            _ => {}
        }
        None
    }

    pub fn to_event(&self, action: Option<&str>) -> PetEvent {
        PetEvent {
            phase: self.phase,
            action: action.map(|s| s.to_string()),
            character_pack: self.character_pack.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PhaseTransition {
    EnterReminding,
    EnterSneakPeek,
    EnterHappyExit,
}
