//! Commands invoked from the pet window, the quick-add box and the dashboard.

use crate::state::{AppState, BubbleView, FrameView};
use crate::{hotkeys, platform, Shared};
use chrono::{Duration, Local, NaiveDateTime};
use pet_core::history::{self, HistoryEntry, Stats};
use pet_core::mood::{Feeling, Mood};
use pet_core::pack::{LoadedPack, PackInfo};
use pet_core::productivity::FocusView;
use pet_core::quick;
use pet_core::reminder::{Reminder, Repeat};
use pet_core::settings::Settings;
use pet_core::store::{Category, Data, ImportMode, Note, ReminderDraft};
use pet_core::system::{Condition, Sample};
use serde::Serialize;
use std::sync::MutexGuard;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_clipboard_manager::ClipboardExt as _;

type CmdResult<T = ()> = Result<T, String>;

fn now() -> NaiveDateTime {
    Local::now().naive_local()
}

fn lock<'a>(state: &'a State<'_, Shared>) -> MutexGuard<'a, AppState> {
    // A panic elsewhere must not brick the UI; the data is still usable.
    state.lock().unwrap_or_else(|e| e.into_inner())
}

/// Tell the dashboard to refresh after data changed anywhere.
pub fn data_changed(app: &AppHandle) {
    let _ = app.emit_to("dashboard", "data-changed", ());
}

/// Run a change against the state, save, and refresh the dashboard.
fn mutate<T>(
    app: &AppHandle,
    state: &State<'_, Shared>,
    f: impl FnOnce(&mut AppState) -> CmdResult<T>,
) -> CmdResult<T> {
    let mut s = lock(state);
    let out = f(&mut s)?;
    s.save()?;
    drop(s);
    data_changed(app);
    Ok(out)
}

// ---------------------------------------------------------------- pet --

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetInit {
    debug: bool,
    pack: LoadedPack,
    frame: Option<FrameView>,
    bubble: Option<BubbleView>,
}

#[tauri::command]
pub fn pet_init(state: State<'_, Shared>) -> CmdResult<PetInit> {
    let s = lock(&state);
    crate::debug_log!("pet_init called by the pet window");
    // Hand over the current frame and bubble: the window may have missed
    // their events while it was loading.
    let init = s.loaded_pack().map(|pack| PetInit {
        debug: crate::debug_enabled(),
        pack,
        frame: s.last_frame.clone(),
        bubble: s.bubble_view(),
    });
    if let Err(e) = &init {
        eprintln!("[pet window] pet_init failed: {e}");
    }
    init
}

/// Lets the windows report problems to the terminal.
#[tauri::command]
pub fn log_from_page(window: WebviewWindow, msg: String) {
    eprintln!("[{} window] {msg}", window.label());
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
pub fn bubble_action(app: AppHandle, state: State<'_, Shared>, id: String) -> CmdResult {
    lock(&state).bubble_action(&id, now())?;
    data_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn pet_feed(state: State<'_, Shared>) {
    lock(&state).feed();
}

#[tauri::command]
pub fn pet_play(state: State<'_, Shared>) {
    lock(&state).play();
}

/// Right-click menu on the pet.
#[tauri::command]
pub fn pet_context_menu(app: AppHandle, window: WebviewWindow, state: State<'_, Shared>) -> CmdResult {
    let (name, care, focusing) = {
        let s = lock(&state);
        (s.data.settings.pet_name.clone(), s.data.settings.care_enabled, s.focus.view(s.clock()).phase != "off")
    };
    let item = |id: &str, text: &str| MenuItem::with_id(&app, id, text, true, None::<&str>);
    let err = |e: tauri::Error| e.to_string();
    let menu = Menu::new(&app).map_err(err)?;
    menu.append(&item("ctx:pet", &format!("Pet {name}")).map_err(err)?).map_err(err)?;
    if care {
        menu.append(&item("ctx:feed", "Feed").map_err(err)?).map_err(err)?;
        menu.append(&item("ctx:play", "Play").map_err(err)?).map_err(err)?;
    }
    menu.append(&PredefinedMenuItem::separator(&app).map_err(err)?).map_err(err)?;
    menu.append(&item("ctx:quick", "Quick add…").map_err(err)?).map_err(err)?;
    menu.append(&item("ctx:clip", "Save clipboard as note").map_err(err)?).map_err(err)?;
    let focus_text = if focusing { "Stop focus timer" } else { "Start focus timer" };
    menu.append(&item("ctx:focus", focus_text).map_err(err)?).map_err(err)?;
    menu.append(&PredefinedMenuItem::separator(&app).map_err(err)?).map_err(err)?;
    menu.append(&item("ctx:dashboard", "Open dashboard").map_err(err)?).map_err(err)?;
    menu.append(&item("ctx:hide", "Hide pet").map_err(err)?).map_err(err)?;
    window.popup_menu(&menu).map_err(err)
}

/// Handles the right-click menu's choices.
pub fn on_context_menu(app: &AppHandle, id: &str) {
    let state = app.state::<Shared>();
    let mut s = state.lock().unwrap_or_else(|e| e.into_inner());
    match id {
        "ctx:pet" => {
            s.pointer_down(None);
            s.pointer_up();
        }
        "ctx:feed" => s.feed(),
        "ctx:play" => s.play(),
        "ctx:focus" => {
            if s.focus.view(s.clock()).phase == "off" {
                s.start_focus();
            } else {
                s.stop_focus();
            }
        }
        "ctx:quick" => {
            drop(s);
            crate::show_quick_add(app);
        }
        "ctx:clip" => {
            drop(s);
            save_clipboard_note(app);
        }
        "ctx:dashboard" => {
            drop(s);
            let _ = crate::show_dashboard(app);
        }
        "ctx:hide" => {
            drop(s);
            if let Some(w) = app.get_webview_window("pet") {
                let _ = w.hide();
            }
        }
        _ => {}
    }
}

/// Save the clipboard's text as a note (hotkey and menu).
pub fn save_clipboard_note(app: &AppHandle) {
    let text = app.clipboard().read_text().unwrap_or_default();
    let state = app.state::<Shared>();
    let mut s = state.lock().unwrap_or_else(|e| e.into_inner());
    let text = text.trim();
    if text.is_empty() {
        s.say("The clipboard has no text to save.");
        return;
    }
    match s.data.add_note(text, now()) {
        Ok(_) => {
            let short: String = text.chars().take(40).collect();
            let more = if text.chars().count() > 40 { "…" } else { "" };
            s.say(format!("Saved to notes: \"{short}{more}\""));
            let _ = s.save();
            drop(s);
            data_changed(app);
        }
        Err(e) => s.say(format!("Couldn't save that: {e}")),
    }
}

#[tauri::command]
pub fn open_dashboard(app: AppHandle) -> CmdResult {
    crate::show_dashboard(&app).map_err(|e| e.to_string())
}

// ---------------------------------------------------------- quick add --

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickPreview {
    title: String,
    due: Option<NaiveDateTime>,
    repeat: Repeat,
}

#[tauri::command]
pub fn parse_quick(text: String) -> QuickPreview {
    let p = quick::parse(&text, now());
    QuickPreview { title: p.title, due: p.due, repeat: p.repeat }
}

/// Add from the quick box: a reminder if a time was found, else a note.
#[tauri::command]
pub fn quick_add(app: AppHandle, state: State<'_, Shared>, text: String) -> CmdResult<String> {
    if text.trim().is_empty() {
        return Err("Type something first.".into());
    }
    let p = quick::parse(&text, now());
    mutate(&app, &state, |s| match p.due {
        Some(due) => {
            let draft = ReminderDraft {
                title: p.title.clone(),
                details: String::new(),
                due,
                repeat: p.repeat,
                enabled: true,
                category_id: None,
            };
            s.data.add_reminder(draft).map_err(|e| e.to_string())?;
            let when = describe_when(due, now());
            s.pet.react(pet_core::behavior::Anim::Happy, 1.2);
            s.say(format!("Got it! I'll remind you {when}: {}", p.title));
            Ok(format!("Reminder set for {when}"))
        }
        None => {
            s.data.add_note(text.trim(), now()).map_err(|e| e.to_string())?;
            s.say("Saved to your notes.");
            Ok("No time found, so it was saved as a note".into())
        }
    })
}

#[tauri::command]
pub fn hide_quick_add(app: AppHandle) {
    if let Some(w) = app.get_webview_window("quickadd") {
        let _ = w.hide();
    }
}

fn describe_when(due: NaiveDateTime, now: NaiveDateTime) -> String {
    let time = due.format("%H:%M");
    if due.date() == now.date() {
        format!("today at {time}")
    } else if due.date() == now.date() + Duration::days(1) {
        format!("tomorrow at {time}")
    } else {
        format!("on {} at {time}", due.format("%a %d %b"))
    }
}

// ---------------------------------------------------------- dashboard --

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    reminders: Vec<Reminder>,
    notes: Vec<Note>,
    categories: Vec<Category>,
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
        categories: s.data.categories.clone(),
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
    focus: FocusView,
    system: Sample,
    cpu_average: Option<f32>,
    condition: Condition,
    screen_minutes: u32,
}

#[tauri::command]
pub fn get_status(state: State<'_, Shared>) -> Status {
    let s = lock(&state);
    Status {
        mood: s.data.mood,
        feeling: s.feeling(),
        anim: s.last_frame.as_ref().map(|f| f.anim),
        due_count: s.data.due_reminders(now()).len(),
        focus: s.focus.view(s.clock()),
        system: s.system.latest,
        cpu_average: s.system.cpu_average(),
        condition: s.system.condition,
        screen_minutes: (s.breaks.active_secs / 60.0) as u32,
    }
}

#[tauri::command]
pub fn add_reminder(app: AppHandle, state: State<'_, Shared>, draft: ReminderDraft) -> CmdResult<u64> {
    mutate(&app, &state, |s| s.data.add_reminder(draft).map_err(|e| e.to_string()))
}

#[tauri::command]
pub fn update_reminder(app: AppHandle, state: State<'_, Shared>, id: u64, draft: ReminderDraft) -> CmdResult {
    mutate(&app, &state, |s| {
        s.data.update_reminder(id, draft).map_err(|e| e.to_string())?;
        s.clear_reminder_bubble(id);
        Ok(())
    })
}

#[tauri::command]
pub fn set_reminder_enabled(app: AppHandle, state: State<'_, Shared>, id: u64, enabled: bool) -> CmdResult {
    mutate(&app, &state, |s| {
        s.data.reminder_mut(id).map_err(|e| e.to_string())?.enabled = enabled;
        s.clear_reminder_bubble(id);
        Ok(())
    })
}

#[tauri::command]
pub fn delete_reminder(app: AppHandle, state: State<'_, Shared>, id: u64) -> CmdResult {
    mutate(&app, &state, |s| {
        s.data.delete_reminder(id).map_err(|e| e.to_string())?;
        s.clear_reminder_bubble(id);
        Ok(())
    })
}

/// Ready-made healthy-habit reminders, filed under "Health".
#[tauri::command]
pub fn add_preset(app: AppHandle, state: State<'_, Shared>, kind: String) -> CmdResult<u64> {
    let (title, details, minutes) = match kind.as_str() {
        "water" => ("Drink water", "A glass of water keeps you sharp.", 60),
        "posture" => ("Check your posture", "Shoulders back, feet flat, screen at eye level.", 30),
        "stretch" => ("Stretch", "Stand up and stretch for a minute.", 60),
        "eyes" => ("Rest your eyes", "20-20-20: look at something 20 feet away for 20 seconds.", 20),
        _ => return Err(format!("unknown preset {kind}")),
    };
    mutate(&app, &state, |s| {
        let category_id = Some(s.data.category_named("Health", "#2e9e6a").map_err(|e| e.to_string())?);
        let due = whole_minute(now() + Duration::minutes(minutes));
        let draft = ReminderDraft {
            title: title.into(),
            details: details.into(),
            due,
            repeat: Repeat::EveryMinutes { minutes: minutes as u32 },
            enabled: true,
            category_id,
        };
        s.data.add_reminder(draft).map_err(|e| e.to_string())
    })
}

fn whole_minute(t: NaiveDateTime) -> NaiveDateTime {
    use chrono::Timelike;
    t.with_second(0).and_then(|t| t.with_nanosecond(0)).unwrap_or(t)
}

#[tauri::command]
pub fn add_note(app: AppHandle, state: State<'_, Shared>, text: String, category_id: Option<u64>) -> CmdResult<u64> {
    mutate(&app, &state, |s| s.data.add_note_in(&text, now(), category_id).map_err(|e| e.to_string()))
}

#[tauri::command]
pub fn delete_note(app: AppHandle, state: State<'_, Shared>, id: u64) -> CmdResult {
    mutate(&app, &state, |s| s.data.delete_note(id).map_err(|e| e.to_string()))
}

#[tauri::command]
pub fn add_category(app: AppHandle, state: State<'_, Shared>, name: String, color: String) -> CmdResult<u64> {
    mutate(&app, &state, |s| s.data.add_category(&name, &color).map_err(|e| e.to_string()))
}

#[tauri::command]
pub fn update_category(app: AppHandle, state: State<'_, Shared>, id: u64, name: String, color: String) -> CmdResult {
    mutate(&app, &state, |s| s.data.update_category(id, &name, &color).map_err(|e| e.to_string()))
}

#[tauri::command]
pub fn delete_category(app: AppHandle, state: State<'_, Shared>, id: u64) -> CmdResult {
    mutate(&app, &state, |s| s.data.delete_category(id).map_err(|e| e.to_string()))
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
    let hotkeys_changed = old.hotkey_quick_add != settings.hotkey_quick_add
        || old.hotkey_clipboard_note != settings.hotkey_clipboard_note;
    data_changed(&app);
    if hotkeys_changed {
        hotkeys::apply(&app, &settings)?;
    }
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

#[tauri::command]
pub fn focus_start(state: State<'_, Shared>) {
    lock(&state).start_focus();
}

#[tauri::command]
pub fn focus_stop(state: State<'_, Shared>) {
    lock(&state).stop_focus();
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryView {
    entries: Vec<HistoryEntry>,
    stats: Stats,
}

/// Newest first, at most `limit` entries.
#[tauri::command]
pub fn get_history(state: State<'_, Shared>, limit: Option<usize>) -> HistoryView {
    let s = lock(&state);
    let h = &s.data.history;
    let n = limit.unwrap_or(300).min(h.len());
    HistoryView { entries: h[h.len() - n..].iter().rev().cloned().collect(), stats: history::stats(h, now().date()) }
}

#[tauri::command]
pub fn export_data(state: State<'_, Shared>, path: String) -> CmdResult {
    let json = lock(&state).data.to_json().map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("Could not write {path}: {e}"))
}

#[tauri::command]
pub fn import_data(app: AppHandle, state: State<'_, Shared>, path: String, mode: ImportMode) -> CmdResult<String> {
    let text = std::fs::read_to_string(&path).map_err(|e| format!("Could not read {path}: {e}"))?;
    let incoming = Data::from_json(&text).map_err(|e| e.to_string())?;
    let summary = format!("{} reminders, {} notes", incoming.reminders.len(), incoming.notes.len());
    let mut s = lock(&state);
    let old_settings = s.data.settings.clone();
    // Keep a copy of what is being replaced, just in case.
    let backup = s.data_path.with_extension(format!("before-import-{}.json", Local::now().format("%Y%m%d-%H%M%S")));
    s.data.save(&backup).map_err(|e| e.to_string())?;
    s.data.import(incoming, mode);
    s.save()?;
    let settings = s.data.settings.clone();
    s.invalidate_view();
    let pack = s.loaded_pack();
    drop(s);
    if let Ok(pack) = pack {
        let _ = app.emit_to("pet", "pet://pack", pack);
    }
    if settings.hotkey_quick_add != old_settings.hotkey_quick_add
        || settings.hotkey_clipboard_note != old_settings.hotkey_clipboard_note
    {
        let _ = hotkeys::apply(&app, &settings);
    }
    data_changed(&app);
    Ok(format!("Imported {summary}. A backup of the previous data was saved next to it."))
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> CmdResult<bool> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> CmdResult<bool> {
    let auto = app.autolaunch();
    if enabled { auto.enable() } else { auto.disable() }.map_err(|e| e.to_string())?;
    auto.is_enabled().map_err(|e| e.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    index: u32,
    name: String,
    width: u32,
    height: u32,
}

#[tauri::command]
pub fn list_monitors(app: AppHandle) -> CmdResult<Vec<MonitorInfo>> {
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    Ok(monitors
        .iter()
        .enumerate()
        .map(|(i, m)| MonitorInfo {
            index: i as u32,
            name: m.name().cloned().unwrap_or_else(|| format!("Monitor {}", i + 1)),
            width: m.size().width,
            height: m.size().height,
        })
        .collect())
}
