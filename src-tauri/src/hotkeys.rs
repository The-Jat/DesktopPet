//! System-wide keyboard shortcuts (quick add, clipboard to note).

use pet_core::settings::Settings;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

/// Replace all shortcuts with the ones in `settings`. Returns an error naming
/// any shortcut that could not be registered (bad format or taken by
/// another app); the others still work.
pub fn apply(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let mut errors = Vec::new();

    let quick = settings.hotkey_quick_add.as_str();
    if !quick.is_empty() {
        let r = gs.on_shortcut(quick, |app, _, event| {
            if event.state == ShortcutState::Pressed {
                crate::show_quick_add(app);
            }
        });
        if let Err(e) = r {
            errors.push(format!("Quick add shortcut \"{quick}\": {e}"));
        }
    }

    let clip = settings.hotkey_clipboard_note.as_str();
    if !clip.is_empty() {
        if clip.eq_ignore_ascii_case(quick) {
            errors.push(format!("\"{clip}\" is used for both shortcuts"));
        } else {
            let r = gs.on_shortcut(clip, |app, _, event| {
                if event.state == ShortcutState::Pressed {
                    crate::commands::save_clipboard_note(app);
                }
            });
            if let Err(e) = r {
                errors.push(format!("Clipboard shortcut \"{clip}\": {e}"));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}
