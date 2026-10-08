use crate::timer::Phase;
use rdev::{listen, EventType};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};

/// Start a background listener. Only Activity during Observing is recorded.
pub fn start_input_hook(app: AppHandle, timer: Arc<Mutex<crate::timer::TimerState>>) {
    thread::spawn(move || {
        let timer_for_cb = Arc::clone(&timer);
        let app_for_cb = app.clone();
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
                let _ = app_for_cb.emit("activity-detected", ());
            }
        });
        if let Err(err) = result {
            eprintln!("input hook error: {err:?}");
        }
    });
}
