// No console window behind the pet in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod hotkeys;
mod platform;
mod state;

use chrono::Local;
use platform::Rect;
use state::{AppState, Env, TickOutput, WindowPlan};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_notification::NotificationExt;

pub type Shared = Mutex<AppState>;

/// `DESKTOPPET_DEBUG=1` prints what the app is doing to the terminal and
/// opens the pet window's developer tools.
pub fn debug_enabled() -> bool {
    std::env::var_os("DESKTOPPET_DEBUG").is_some()
}

#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        if $crate::debug_enabled() {
            eprintln!("[debug] {}", format_args!($($arg)*));
        }
    };
}

const FRAME: Duration = Duration::from_millis(33);
const SYSTEM_SAMPLE_EVERY: Duration = Duration::from_secs(2);

fn main() {
    tauri::Builder::default()
        // Must be first: a second launch just opens the running pet's dashboard.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = show_dashboard(app);
        }))
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle();
            let data_dir = handle.path().app_data_dir()?;
            let user_pets = data_dir.join("pets");
            std::fs::create_dir_all(&user_pets)?;

            let data_path = data_dir.join("data.json");
            let data = load_data(&data_path);
            let settings = data.settings.clone();

            // Built-in packs first so a user pack with the same id wins.
            let mut roots: Vec<PathBuf> = Vec::new();
            if let Ok(res) = handle.path().resource_dir() {
                roots.push(res.join("pets"));
            }
            if cfg!(debug_assertions) {
                roots.push(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../pets")));
            }
            roots.push(user_pets);

            app.manage::<Shared>(Mutex::new(AppState::new(data, data_path, roots)));
            // Windows are created only now, after the state above exists:
            // their pages call commands that need it as soon as they load.
            for config in &app.config().app.windows {
                WebviewWindowBuilder::from_config(handle, config)?.build()?;
            }
            build_tray(handle)?;
            handle.on_menu_event(|app, event| {
                if event.id.as_ref().starts_with("ctx:") {
                    commands::on_context_menu(app, event.id.as_ref());
                }
            });
            if let Err(e) = hotkeys::apply(handle, &settings) {
                eprintln!("DesktopPet: {e}");
            }
            spawn_engine(handle.clone());

            debug_log!("data: {}", data_dir.display());
            #[cfg(debug_assertions)]
            if debug_enabled() {
                if let Some(pet) = handle.get_webview_window("pet") {
                    pet.open_devtools();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // Closing the dashboard only hides it; quit from the tray.
            WindowEvent::CloseRequested { api, .. } if window.label() != "pet" => {
                api.prevent_close();
                let _ = window.hide();
            }
            // The quick-add box goes away when you click elsewhere.
            WindowEvent::Focused(false) if window.label() == "quickadd" => {
                let _ = window.hide();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::pet_init,
            commands::log_from_page,
            commands::pet_pointer_down,
            commands::pet_pointer_up,
            commands::bubble_action,
            commands::pet_feed,
            commands::pet_play,
            commands::pet_context_menu,
            commands::open_dashboard,
            commands::parse_quick,
            commands::quick_add,
            commands::hide_quick_add,
            commands::get_overview,
            commands::get_status,
            commands::add_reminder,
            commands::update_reminder,
            commands::set_reminder_enabled,
            commands::delete_reminder,
            commands::add_preset,
            commands::add_note,
            commands::delete_note,
            commands::add_category,
            commands::update_category,
            commands::delete_category,
            commands::save_settings,
            commands::reload_packs,
            commands::pet_say,
            commands::focus_start,
            commands::focus_stop,
            commands::get_history,
            commands::export_data,
            commands::import_data,
            commands::get_autostart,
            commands::set_autostart,
            commands::list_monitors,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DesktopPet");
}

/// Load saved data. A corrupt file is moved aside rather than overwritten,
/// so nothing is lost silently.
fn load_data(path: &std::path::Path) -> pet_core::store::Data {
    match pet_core::store::Data::load(path) {
        Ok(data) => data,
        Err(e) => {
            let backup = path.with_extension(format!("corrupt-{}.json", Local::now().format("%Y%m%d-%H%M%S")));
            eprintln!("DesktopPet: {e}; moving it to {}", backup.display());
            let _ = std::fs::rename(path, &backup);
            pet_core::store::Data::default()
        }
    }
}

pub fn show_dashboard(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("dashboard") {
        w.show()?;
        w.unminimize()?;
        w.set_focus()?;
    }
    Ok(())
}

/// Open the quick-add box in the middle of the screen, ready to type.
pub fn show_quick_add(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("quickadd") {
        let _ = w.center();
        let _ = w.show();
        let _ = w.set_focus();
        let _ = app.emit_to("quickadd", "quickadd://open", ());
    }
}

fn toggle_pet(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("pet") {
        if w.is_visible()? {
            w.hide()?;
        } else {
            w.show()?;
        }
    }
    Ok(())
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let item = |id: &str, text: &str| MenuItem::with_id(app, id, text, true, None::<&str>);
    let menu = Menu::with_items(
        app,
        &[
            &item("open", "Open dashboard")?,
            &item("quick", "Quick add…")?,
            &item("toggle", "Show / hide pet")?,
            &PredefinedMenuItem::separator(app)?,
            &item("quit", "Quit")?,
        ],
    )?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("DesktopPet")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let _ = match event.id.as_ref() {
                "open" => show_dashboard(app),
                "quick" => {
                    show_quick_add(app);
                    Ok(())
                }
                "toggle" => toggle_pet(app),
                "quit" => {
                    // Keep the mood and timers' effects on disk.
                    let _ = app.state::<Shared>().lock().map(|s| s.save());
                    app.exit(0);
                    Ok(())
                }
                _ => Ok(()),
            };
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                let _ = show_dashboard(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Things that need the main thread, refreshed about once a second.
struct SlowEnv {
    scale_factor: f64,
    /// Work areas of all monitors, in the order the OS lists them.
    monitors: Vec<Rect>,
    current: Option<Rect>,
    pet_visible: bool,
    fullscreen: bool,
}

/// Runs the pet at ~30 fps on its own thread.
fn spawn_engine(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last = Instant::now();
        let mut slow_checked: Option<Instant> = None;
        let mut sampled: Option<Instant> = None;
        let mut slow =
            SlowEnv { scale_factor: 1.0, monitors: vec![], current: None, pet_visible: true, fullscreen: false };
        let mut applied: Option<WindowPlan> = None;

        loop {
            std::thread::sleep(FRAME);
            let dt = last.elapsed().as_secs_f64().min(0.25);
            last = Instant::now();

            let Some(win) = app.get_webview_window("pet") else { continue };
            let state = app.state::<Shared>();

            // Window queries go through the main thread, so only once a second.
            if slow_checked.is_none_or(|t| t.elapsed() >= Duration::from_secs(1)) {
                slow_checked = Some(Instant::now());
                slow.scale_factor = win.scale_factor().unwrap_or(slow.scale_factor);
                slow.pet_visible = win.is_visible().unwrap_or(true);
                slow.fullscreen = platform::fullscreen_app();
                if let Ok(list) = app.available_monitors() {
                    slow.monitors = list.iter().map(|m| work_rect(m.work_area())).collect();
                }
                if let Ok(Some(m)) = win.current_monitor() {
                    slow.current = Some(work_rect(m.work_area()));
                }
            }
            let system = (sampled.is_none_or(|t| t.elapsed() >= SYSTEM_SAMPLE_EVERY)).then(|| {
                let dt = sampled.map_or(SYSTEM_SAMPLE_EVERY.as_secs_f32(), |t| t.elapsed().as_secs_f32());
                sampled = Some(Instant::now());
                (platform::system_sample(), dt)
            });

            let (px, py, monitor) = {
                let s = state.lock().unwrap_or_else(|e| e.into_inner());
                (s.pet.x, s.pet.y, s.data.settings.monitor_index)
            };
            let fallback = Rect { left: 0.0, top: 0.0, right: 1920.0, bottom: 1080.0 };
            let work_area = monitor
                .and_then(|i| slow.monitors.get(i as usize).copied())
                .or_else(|| platform::work_area_near(px + 1.0, py + 1.0))
                .or(slow.current)
                .unwrap_or(fallback);
            let env = Env {
                now: Local::now().naive_local(),
                cursor: platform::cursor_pos(),
                left_down: platform::left_button_down(),
                idle_secs: platform::user_idle_secs(),
                work_area,
                scale_factor: slow.scale_factor,
                foreground: platform::foreground_window(),
                pet_visible: slow.pet_visible,
                fullscreen: slow.fullscreen,
                system: system.map(|(s, _)| s),
                system_dt: system.map_or(0.0, |(_, dt)| dt),
            };

            let out = state.lock().unwrap_or_else(|e| e.into_inner()).tick(dt, &env);
            apply(&app, &win, out, &mut applied);
        }
    });
}

fn work_rect(r: &tauri::PhysicalRect<i32, u32>) -> Rect {
    Rect {
        left: r.position.x.into(),
        top: r.position.y.into(),
        right: f64::from(r.position.x) + f64::from(r.size.width),
        bottom: f64::from(r.position.y) + f64::from(r.size.height),
    }
}

fn apply(app: &AppHandle, win: &WebviewWindow, out: TickOutput, applied: &mut Option<WindowPlan>) {
    if let Some(plan) = out.window {
        let prev = applied.replace(plan);
        if prev.map(|p| (p.width, p.height)) != Some((plan.width, plan.height)) {
            let _ = win.set_size(PhysicalSize::new(plan.width, plan.height));
        }
        if prev.map(|p| (p.x, p.y)) != Some((plan.x, plan.y)) {
            let _ = win.set_position(PhysicalPosition::new(plan.x, plan.y));
        }
        if prev.is_none() {
            debug_log!("first window placement: {plan:?}");
        } else if prev.map(|p| (p.width, p.height)) != Some((plan.width, plan.height)) {
            debug_log!("window resized: {plan:?}");
        }
    }
    if out.show_pet {
        let _ = win.show();
    }
    if let Some(frame) = out.frame {
        debug_log!("frame: {:?} facing {:?}", frame.anim, frame.facing);
        let _ = app.emit_to("pet", "pet://frame", frame);
    }
    if let Some(bubble) = out.bubble {
        let _ = app.emit_to("pet", "pet://bubble", bubble);
    }
    for (title, body) in out.notify {
        if let Err(e) = app.notification().builder().title(title).body(body).show() {
            eprintln!("DesktopPet: notification failed: {e}");
        }
    }
    if let Some(tip) = out.tooltip {
        if let Some(tray) = app.tray_by_id("main") {
            let _ = tray.set_tooltip(Some(tip));
        }
    }
    if out.data_changed {
        commands::data_changed(app);
    }
}
