//! Commands invoked from the pet window and the dashboard.

use crate::state::{AppState, BubbleView, FrameView};
use crate::{platform, Shared};
use chrono::{Local, NaiveDateTime};
use pet_core::mood::{Feeling, Mood};
use pet_core::pack::{LoadedPack, PackInfo};
use pet_core::reminder::Reminder;
use pet_core::settings::Settings;
use pet_core::store::{Note, ReminderDraft};
use serde::Serialize;
use std::sync::MutexGuard;
use tauri::{AppHandle, Emitter, Manager, State};

type CmdResult<T = ()> = Result<T, String>;

fn now() -> NaiveDateTime {
    Local::now().naive_local()
}

fn lock<'a>(state: &'a State<'_, Shared>) -> MutexGuard<'a, AppState> {
    // A panic elsewhere must not brick the UI; the data is still usable.
    state.lock().unwrap_or_else(|e| e.into_inner())
}

/// Tell the dashboard to refresh after data changed anywhere.
fn data_changed(app: &AppHandle) {
    let _ = app.emit_to("dashboard", "data-changed", ());
}

// ---- pet window ----

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetInit {
    pack: LoadedPack,
    frame: Option<FrameView>,
    bubble: Option<BubbleView>,
}

#[tauri::command]
pub fn pet_init(state: State<'_, Shared>) -> CmdResult<PetInit> {
    let s = lock(&state);
    // Hand over the current frame and bubble: the window may have missed
    // their events while it was loading.
    Ok(PetInit { pack: s.loaded_pack()?, frame: s.last_frame.clone(), bubble: s.bubble_view() })
}

#[tauri::command]
pub fn pet_pointer_down(state: State<'_, Shared>) {
    lock(&state).pointer_down(platform::cursor_pos());
}

#[tauri::command]
pub fn pet_pointer_up(state: State<'_, Shared>) {
    lock(&state).pointer_up();
}

#[tauri::command]
pub fn reminder_done(app: AppHandle, state: State<'_, Shared>, id: u64) -> CmdResult {
    lock(&state).reminder_done(id, now())?;
    data_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn reminder_snooze(app: AppHandle, state: State<'_, Shared>, id: u64, minutes: Option<u32>) -> CmdResult {
    lock(&state).reminder_snooze(id, now(), minutes)?;
    data_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn open_dashboard(app: AppHandle) -> CmdResult {
    crate::show_dashboard(&app).map_err(|e| e.to_string())
}

// ---- dashboard ----

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    reminders: Vec<Reminder>,
    notes: Vec<Note>,
    settings: Settings,
    packs: Vec<PackInfo>,
    pack_errors: Vec<String>,
    data_path: String,
    user_pets_dir: String,
}

#[tauri::command]
pub fn get_overview(app: AppHandle, state: State<'_, Shared>) -> CmdResult<Overview> {
    let s = lock(&state);
    let mut pack_errors = s.pack_errors.clone();
    let packs = s.packs.iter().filter_map(|p| p.info().map_err(|e| pack_errors.push(e.to_string())).ok()).collect();
    let user_pets_dir = app.path().app_data_dir().map(|d| d.join("pets").display().to_string()).unwrap_or_default();
    Ok(Overview {
        reminders: s.data.reminders.clone(),
        notes: s.data.notes.clone(),
        settings: s.data.settings.clone(),
        packs,
        pack_errors,
        data_path: s.data_path.display().to_string(),
        user_pets_dir,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    mood: Mood,
    feeling: Feeling,
    anim: Option<pet_core::behavior::Anim>,
    due_count: usize,
}

#[tauri::command]
pub fn get_status(state: State<'_, Shared>) -> Status {
    let s = lock(&state);
    Status {
        mood: s.mood,
        feeling: s.feeling(),
        anim: s.last_frame.as_ref().map(|f| f.anim),
        due_count: s.data.due_reminders(now()).len(),
    }
}

#[tauri::command]
pub fn add_reminder(app: AppHandle, state: State<'_, Shared>, draft: ReminderDraft) -> CmdResult<u64> {
    let mut s = lock(&state);
    let id = s.data.add_reminder(draft).map_err(|e| e.to_string())?;
    s.save()?;
    drop(s);
    data_changed(&app);
    Ok(id)
}

#[tauri::command]
pub fn update_reminder(app: AppHandle, state: State<'_, Shared>, id: u64, draft: ReminderDraft) -> CmdResult {
    let mut s = lock(&state);
    s.data.update_reminder(id, draft).map_err(|e| e.to_string())?;
    s.clear_reminder_bubble(id);
    s.save()?;
    drop(s);
    data_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn set_reminder_enabled(app: AppHandle, state: State<'_, Shared>, id: u64, enabled: bool) -> CmdResult {
    let mut s = lock(&state);
    s.data.reminder_mut(id).map_err(|e| e.to_string())?.enabled = enabled;
    s.clear_reminder_bubble(id);
    s.save()?;
    drop(s);
    data_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_reminder(app: AppHandle, state: State<'_, Shared>, id: u64) -> CmdResult {
    let mut s = lock(&state);
    s.data.delete_reminder(id).map_err(|e| e.to_string())?;
    s.clear_reminder_bubble(id);
    s.save()?;
    drop(s);
    data_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn add_note(app: AppHandle, state: State<'_, Shared>, text: String) -> CmdResult<u64> {
    let mut s = lock(&state);
    let id = s.data.add_note(&text, now()).map_err(|e| e.to_string())?;
    s.save()?;
    drop(s);
    data_changed(&app);
    Ok(id)
}

#[tauri::command]
pub fn delete_note(app: AppHandle, state: State<'_, Shared>, id: u64) -> CmdResult {
    let mut s = lock(&state);
    s.data.delete_note(id).map_err(|e| e.to_string())?;
    s.save()?;
    drop(s);
    data_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<'_, Shared>, settings: Settings) -> CmdResult<Settings> {
    let mut s = lock(&state);
    let settings = settings.sanitized();
    let old = std::mem::replace(&mut s.data.settings, settings.clone());
    s.save()?;
    let pack_changed = old.pet_id != settings.pet_id || old.scale != settings.scale;
    if pack_changed {
        s.invalidate_view();
    }
    let pack = if pack_changed { Some(s.loaded_pack()) } else { None };
    drop(s);
    if let Some(pack) = pack {
        let _ = app.emit_to("pet", "pet://pack", pack?);
    }
    data_changed(&app);
    Ok(settings)
}

#[tauri::command]
pub fn reload_packs(app: AppHandle, state: State<'_, Shared>) -> CmdResult {
    let mut s = lock(&state);
    s.reload_packs();
    let pack = s.loaded_pack();
    drop(s);
    let _ = app.emit_to("pet", "pet://pack", pack?);
    data_changed(&app);
    Ok(())
}

/// Preview the speech bubble from the dashboard.
#[tauri::command]
pub fn pet_say(state: State<'_, Shared>, text: String) {
    let text = text.trim();
    if !text.is_empty() {
        lock(&state).say(text.to_owned());
    }
}
