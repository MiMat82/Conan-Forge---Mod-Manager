use crate::model::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;

#[derive(Default)]
pub struct Startup {
    finished: AtomicBool,
}

impl Startup {
    fn finish_once(&self, reveal: impl FnOnce() -> Result<()>) -> Result<()> {
        if self.finished.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        if let Err(error) = reveal() {
            self.finished.store(false, Ordering::SeqCst);
            return Err(error);
        }
        Ok(())
    }
}

pub fn finish(app: &tauri::AppHandle) -> Result<()> {
    app.state::<Startup>().finish_once(|| {
        let main = app
            .get_webview_window("main")
            .ok_or("Main window is unavailable")?;
        main.show().map_err(|e| e.to_string())?;
        let _ = main.set_focus();
        if let Some(splash) = app.get_webview_window("splash") {
            splash.close().map_err(|e| e.to_string())?;
        }
        Ok(())
    })
}

#[tauri::command]
pub fn show_intro(window: tauri::WebviewWindow, state: tauri::State<'_, Startup>) -> Result<bool> {
    if window.label() != "splash" {
        return Err("Only the startup window can show the intro.".into());
    }
    if state.finished.load(Ordering::SeqCst) {
        return Ok(false);
    }
    window.show().map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn finish_intro(window: tauri::WebviewWindow, app: tauri::AppHandle) -> Result<()> {
    if window.label() != "splash" {
        return Err("Only the startup window can finish the intro.".into());
    }
    finish(&app)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::AtomicUsize};

    #[test]
    fn audio_completion_skip_and_fallback_reveal_only_once() {
        let startup = Arc::new(Startup::default());
        let count = Arc::new(AtomicUsize::new(0));
        let threads: Vec<_> = (0..3)
            .map(|_| {
                let startup = Arc::clone(&startup);
                let count = Arc::clone(&count);
                std::thread::spawn(move || {
                    startup
                        .finish_once(|| {
                            count.fetch_add(1, Ordering::SeqCst);
                            Ok(())
                        })
                        .unwrap()
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn failed_reveal_allows_the_fallback_to_retry() {
        let startup = Startup::default();
        assert!(
            startup
                .finish_once(|| Err("window unavailable".into()))
                .is_err()
        );
        assert!(!startup.finished.load(Ordering::SeqCst));
        startup.finish_once(|| Ok(())).unwrap();
        assert!(startup.finished.load(Ordering::SeqCst));
    }
}
