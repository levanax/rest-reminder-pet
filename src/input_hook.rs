use crate::timer::Phase;
use rdev::{listen, EventType};
use std::sync::{Arc, Mutex};
use std::thread;

/// Start a background listener. Only Activity during Observing is recorded.
pub fn start_input_hook(timer: Arc<Mutex<crate::timer::TimerState>>) {
    thread::spawn(move || {
        let timer_for_cb = Arc::clone(&timer);
        let result = listen(move |event| {
            let is_activity = matches!(
                event.event_type,
                EventType::KeyPress(_)
                    | EventType::ButtonPress(_)
                    | EventType::MouseMove { .. }
                    | EventType::Wheel { .. }
            );
            if !is_activity {
                return;
            }
            let Ok(mut state) = timer_for_cb.lock() else {
                return;
            };
            if state.phase == Phase::Observing {
                state.note_activity();
            }
        });
        if let Err(err) = result {
            eprintln!("input hook error: {err:?}");
        }
    });
}
