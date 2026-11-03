use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Working,
    Paused,
    Reminding,
    Observing,
    HappyExit,
    FallExit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetEvent {
    pub phase: Phase,
    pub action: Option<String>,
    pub character_pack: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldReason {
    Lock,
    Sleep,
}

pub struct TimerState {
    pub phase: Phase,
    pub work_minutes: u32,
    pub observation_seconds: u32,
    pub character_pack: String,
    pub work_elapsed: Duration,
    pub segment_started: Option<Instant>,
    pub observation_started: Option<Instant>,
    pub observation_banked: Duration,
    pub last_activity: Option<Instant>,
    /// 当前 Reminder 尚未记账（成功或未完成）。
    pub reminder_active: bool,
    pub delay_accumulated: Duration,
    pub delay_running_since: Option<Instant>,
    /// SessionLock 解除后是否应继续累计 WorkSession（手动暂停优先）。
    pub resume_work_on_unlock: bool,
    pub hold_lock: bool,
    pub hold_sleep: bool,
}

impl TimerState {
    pub fn new(work_minutes: u32, observation_seconds: u32, character_pack: String) -> Self {
        Self {
            phase: Phase::Working,
            work_minutes,
            observation_seconds,
            character_pack,
            work_elapsed: Duration::ZERO,
            segment_started: Some(Instant::now()),
            observation_started: None,
            observation_banked: Duration::ZERO,
            last_activity: None,
            reminder_active: false,
            delay_accumulated: Duration::ZERO,
            delay_running_since: None,
            resume_work_on_unlock: false,
            hold_lock: false,
            hold_sleep: false,
        }
    }

    pub fn session_held(&self) -> bool {
        self.hold_lock || self.hold_sleep
    }

    fn in_reminder(&self) -> bool {
        matches!(
            self.phase,
            Phase::Reminding | Phase::Observing | Phase::HappyExit | Phase::FallExit
        )
    }

    fn clear_delay_clocks(&mut self) {
        self.delay_accumulated = Duration::ZERO;
        self.delay_running_since = None;
    }

    fn freeze_delay(&mut self) {
        if let Some(start) = self.delay_running_since.take() {
            self.delay_accumulated += start.elapsed();
        }
    }

    fn freeze_observation(&mut self) {
        if let Some(start) = self.observation_started.take() {
            self.observation_banked += start.elapsed();
        }
    }

    fn current_observation(&self) -> Duration {
        let mut total = self.observation_banked;
        if let Some(start) = self.observation_started {
            total += start.elapsed();
        }
        total
    }

    pub fn current_rest_delay(&self) -> Duration {
        let mut total = self.delay_accumulated;
        if let Some(start) = self.delay_running_since {
            total += start.elapsed();
        }
        total
    }

    /// 进入或离开 SessionLock。若保持状态发生变化，返回现在是否处于暂停。
    pub fn set_hold(&mut self, reason: HoldReason, active: bool) -> Option<bool> {
        let was = self.session_held();
        match reason {
            HoldReason::Lock => self.hold_lock = active,
            HoldReason::Sleep => self.hold_sleep = active,
        }
        let now_held = self.session_held();
        if !was && now_held {
            self.freeze_for_hold();
            return Some(true);
        }
        if was && !now_held {
            self.thaw_after_hold();
            return Some(false);
        }
        None
    }

    fn freeze_for_hold(&mut self) {
        if self.phase == Phase::Working {
            self.flush_work();
            self.segment_started = None;
            self.resume_work_on_unlock = true;
        } else if self.phase == Phase::Paused {
            self.resume_work_on_unlock = false;
        }
        self.freeze_delay();
        self.freeze_observation();
    }

    fn thaw_after_hold(&mut self) {
        let now = Instant::now();
        if self.resume_work_on_unlock && self.phase == Phase::Working {
            self.segment_started = Some(now);
            self.resume_work_on_unlock = false;
        }
        if self.phase == Phase::Reminding && self.reminder_active {
            self.delay_running_since = Some(now);
        }
        if self.phase == Phase::Observing {
            self.observation_started = Some(now);
        }
    }

    /// HappyExit 记账本次 RestDelay。已记账则返回 None。
    pub fn claim_success_delay(&mut self) -> Option<u64> {
        if !self.reminder_active || self.phase != Phase::HappyExit {
            return None;
        }
        self.reminder_active = false;
        let secs = self.current_rest_delay().as_secs();
        self.clear_delay_clocks();
        Some(secs)
    }

    /// 「知道了」等未完成路径。已记账则返回 None；否则返回本次已累计拖延秒数。
    pub fn claim_incomplete(&mut self) -> Option<u64> {
        if !self.reminder_active || !self.in_reminder() {
            return None;
        }
        self.reminder_active = false;
        let secs = self.current_rest_delay().as_secs();
        self.clear_delay_clocks();
        Some(secs)
    }

    pub fn apply_config(&mut self, work_minutes: u32, observation_seconds: u32, pack: String) {
        self.work_minutes = work_minutes;
        self.observation_seconds = observation_seconds;
        self.character_pack = pack;
    }

    fn flush_work(&mut self) {
        if self.phase == Phase::Working {
            if let Some(start) = self.segment_started.take() {
                self.work_elapsed += start.elapsed();
            }
        }
    }

    pub fn pause(&mut self) {
        if self.phase == Phase::Working {
            self.flush_work();
            self.phase = Phase::Paused;
            self.resume_work_on_unlock = false;
        }
    }

    pub fn resume(&mut self) {
        if self.phase == Phase::Paused {
            self.phase = Phase::Working;
            if self.session_held() {
                self.segment_started = None;
                self.resume_work_on_unlock = true;
            } else {
                self.segment_started = Some(Instant::now());
                self.resume_work_on_unlock = false;
            }
        }
    }

    pub fn dismiss_to_working(&mut self, was_paused: bool) {
        self.phase = if was_paused {
            Phase::Paused
        } else {
            Phase::Working
        };
        self.observation_started = None;
        self.observation_banked = Duration::ZERO;
        self.last_activity = None;
        self.reminder_active = false;
        self.clear_delay_clocks();
        self.work_elapsed = Duration::ZERO;
        if was_paused || self.session_held() {
            self.segment_started = None;
            self.resume_work_on_unlock = !was_paused && self.session_held();
        } else {
            self.segment_started = Some(Instant::now());
            self.resume_work_on_unlock = false;
        }
    }

    pub fn start_reminding(&mut self) {
        self.flush_work();
        self.phase = Phase::Reminding;
        self.segment_started = None;
        self.resume_work_on_unlock = false;
        self.observation_started = None;
        self.observation_banked = Duration::ZERO;
        self.last_activity = None;
        self.reminder_active = true;
        self.delay_accumulated = Duration::ZERO;
        self.delay_running_since = if self.session_held() {
            None
        } else {
            Some(Instant::now())
        };
    }

    pub fn enter_observing(&mut self) {
        self.phase = Phase::Observing;
        self.freeze_delay();
        self.observation_banked = Duration::ZERO;
        self.observation_started = if self.session_held() {
            None
        } else {
            Some(Instant::now())
        };
        self.last_activity = None;
    }

    /// 观察期内有键鼠：已过去的休息作废并计入拖延，连续空闲重新计算。锁屏时忽略。
    pub fn note_activity(&mut self) {
        if self.phase != Phase::Observing || self.session_held() {
            return;
        }
        self.delay_accumulated += self.current_observation();
        self.observation_banked = Duration::ZERO;
        self.observation_started = Some(Instant::now());
        self.last_activity = Some(Instant::now());
    }

    pub fn start_happy_exit(&mut self) {
        self.phase = Phase::HappyExit;
    }

    pub fn work_deadline(&self) -> Duration {
        Duration::from_secs(self.work_minutes as u64 * 60)
    }

    pub fn current_work_elapsed(&self) -> Duration {
        let mut total = self.work_elapsed;
        if self.phase == Phase::Working {
            if let Some(start) = self.segment_started {
                total += start.elapsed();
            }
        }
        total
    }

    pub fn tick(&mut self) -> Option<PhaseTransition> {
        if self.session_held() {
            return None;
        }
        match self.phase {
            Phase::Working => {
                if self.current_work_elapsed() >= self.work_deadline() {
                    return Some(PhaseTransition::EnterReminding);
                }
            }
            // 无桌宠回调后 Reminding 不应长时间停留；卡住则推进观察期
            Phase::Reminding => {
                if self.delay_running_since
                    .map(|t| t.elapsed() >= Duration::from_secs(2))
                    .unwrap_or(true)
                {
                    return Some(PhaseTransition::EnterObserving);
                }
            }
            Phase::Observing => {
                if self.current_observation()
                    >= Duration::from_secs(self.observation_seconds as u64)
                {
                    self.freeze_observation();
                    self.start_happy_exit();
                    return Some(PhaseTransition::EnterHappyExit);
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
    EnterObserving,
    EnterHappyExit,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn working_deadline_triggers_reminding() {
        let mut t = TimerState::new(1, 30, "default".into());
        t.work_elapsed = Duration::from_secs(60);
        t.segment_started = None;
        t.phase = Phase::Working;
        assert!(matches!(t.tick(), Some(PhaseTransition::EnterReminding)));
    }

    #[test]
    fn reminding_advances_to_observing_after_grace() {
        let mut t = TimerState::new(20, 30, "default".into());
        t.start_reminding();
        assert!(t.tick().is_none());
        thread::sleep(Duration::from_millis(2100));
        assert!(matches!(t.tick(), Some(PhaseTransition::EnterObserving)));
    }

    #[test]
    fn observing_idle_triggers_happy_exit() {
        let mut t = TimerState::new(20, 1, "default".into());
        t.start_reminding();
        t.enter_observing();
        thread::sleep(Duration::from_millis(1100));
        assert!(matches!(t.tick(), Some(PhaseTransition::EnterHappyExit)));
    }
}
