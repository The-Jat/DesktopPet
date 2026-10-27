// No console window behind the pet in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod platform;
mod state;

use chrono::Local;
use platform::Rect;
use state::{AppState, Env, TickOutput, WindowPlan};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

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

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle();
            let data_dir = handle.path().app_data_dir()?;
            let user_pets = data_dir.join("pets");
            std::fs::create_dir_all(&user_pets)?;

            let data_path = data_dir.join("data.json");
            let data = load_data(&data_path);

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
        .on_window_event(|window, event| {
            // Closing the dashboard only hides it; quit from the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "dashboard" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::pet_init,
            commands::pet_pointer_down,
            commands::pet_pointer_up,
            commands::reminder_done,
            commands::reminder_snooze,
            commands::open_dashboard,
            commands::get_overview,
            commands::get_status,
            commands::add_reminder,
            commands::update_reminder,
            commands::set_reminder_enabled,
            commands::delete_reminder,
            commands::add_note,
            commands::delete_note,
            commands::save_settings,
            commands::reload_packs,
            commands::pet_say,
            commands::log_from_page,
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
    let open = MenuItem::with_id(app, "open", "Open dashboard", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "Show / hide pet", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &toggle, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("DesktopPet")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let _ = match event.id.as_ref() {
                "open" => show_dashboard(app),
                "toggle" => toggle_pet(app),
                "quit" => {
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

/// Runs the pet at ~30 fps on its own thread.
fn spawn_engine(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last = Instant::now();
        let mut monitor_checked: Option<Instant> = None;
        let mut scale_factor = 1.0;
        let mut fallback_area = Rect { left: 0.0, top: 0.0, right: 1920.0, bottom: 1080.0 };
        let mut applied: Option<WindowPlan> = None;

        loop {
            std::thread::sleep(FRAME);
            let dt = last.elapsed().as_secs_f64().min(0.25);
            last = Instant::now();

            let Some(win) = app.get_webview_window("pet") else { continue };
            let state = app.state::<Shared>();

            // Window queries go through the main thread, so only once a second.
            if monitor_checked.is_none_or(|t| t.elapsed() >= Duration::from_secs(1)) {
                monitor_checked = Some(Instant::now());
                scale_factor = win.scale_factor().unwrap_or(scale_factor);
                if let Ok(Some(m)) = win.current_monitor() {
                    let (p, s) = (m.position(), m.size());
                    fallback_area = Rect {
                        left: p.x.into(),
                        top: p.y.into(),
                        right: f64::from(p.x) + f64::from(s.width),
                        bottom: f64::from(p.y) + f64::from(s.height),
                    };
                }
            }

            let (px, py) = {
                let s = state.lock().unwrap_or_else(|e| e.into_inner());
                (s.pet.x, s.pet.y)
            };
            let env = Env {
                now: Local::now().naive_local(),
                cursor: platform::cursor_pos(),
                left_down: platform::left_button_down(),
                idle_secs: platform::user_idle_secs(),
                work_area: platform::work_area_near(px, py).unwrap_or(fallback_area),
                scale_factor,
            };

            let out = state.lock().unwrap_or_else(|e| e.into_inner()).tick(dt, &env);
            apply(&app, &win, out, &mut applied);
        }
    });
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
}
