//! The OS calls the pet needs, all in physical screen pixels.
//!
//! Windows is the target platform. Elsewhere most of these return `None` so
//! the app still builds and runs for development, just without dragging,
//! idle detection, the active-window perch, and most system readings.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Rect {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }

    pub fn width(&self) -> f64 {
        self.right - self.left
    }
}

/// The window the user is working in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Foreground {
    /// The desktop, the taskbar, or nothing usable.
    None,
    /// One of our own windows (keep using the previous one).
    Ours,
    Window(Rect),
}

#[cfg(windows)]
mod imp {
    use super::{Foreground, Rect};
    use pet_core::system::{Battery, Sample};
    use std::sync::Mutex;
    use windows_sys::Win32::Foundation::{FILETIME, HWND, POINT, RECT};
    use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    use windows_sys::Win32::System::SystemInformation::{
        GetTickCount, GetWindowsDirectoryW, GlobalMemoryStatusEx, MEMORYSTATUSEX,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcessId, GetSystemTimes};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, GetLastInputInfo, LASTINPUTINFO, VK_LBUTTON,
    };
    use windows_sys::Win32::UI::Shell::{
        SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetCursorPos, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId, IsIconic,
    };

    pub fn cursor_pos() -> Option<(f64, f64)> {
        let mut p = POINT { x: 0, y: 0 };
        // SAFETY: `p` is a valid, writable POINT.
        (unsafe { GetCursorPos(&mut p) } != 0).then(|| (f64::from(p.x), f64::from(p.y)))
    }

    pub fn left_button_down() -> Option<bool> {
        // SAFETY: plain query with a valid virtual-key code.
        let state = unsafe { GetAsyncKeyState(i32::from(VK_LBUTTON)) };
        Some(state < 0) // high bit set = currently down
    }

    pub fn user_idle_secs() -> u64 {
        let mut info = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
        // SAFETY: `info` is valid with cbSize set as the API requires.
        if unsafe { GetLastInputInfo(&mut info) } == 0 {
            return 0;
        }
        // SAFETY: no arguments. Both values wrap every ~49 days, so subtract wrapping.
        u64::from(unsafe { GetTickCount() }.wrapping_sub(info.dwTime)) / 1000
    }

    fn rect(r: RECT) -> Rect {
        Rect { left: r.left.into(), top: r.top.into(), right: r.right.into(), bottom: r.bottom.into() }
    }

    /// Work area (screen minus taskbar) of the monitor nearest to a point.
    pub fn work_area_near(x: f64, y: f64) -> Option<Rect> {
        let pt = POINT { x: x as i32, y: y as i32 };
        let empty = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            rcMonitor: empty,
            rcWork: empty,
            dwFlags: 0,
        };
        // SAFETY: MONITOR_DEFAULTTONEAREST always yields a monitor handle, and
        // `info` is valid with cbSize set.
        let ok = unsafe { GetMonitorInfoW(MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST), &mut info) } != 0;
        ok.then(|| rect(info.rcWork))
    }

    pub fn foreground_window() -> Foreground {
        // SAFETY: no arguments; may return null.
        let hwnd: HWND = unsafe { GetForegroundWindow() };
        if hwnd.is_null() {
            return Foreground::None;
        }
        let mut pid = 0u32;
        // SAFETY: valid window handle and out pointer.
        unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
        // SAFETY: no arguments.
        if pid == unsafe { GetCurrentProcessId() } {
            return Foreground::Ours;
        }
        // SAFETY: valid window handle.
        if unsafe { IsIconic(hwnd) } != 0 {
            return Foreground::None;
        }
        let mut class = [0u16; 64];
        // SAFETY: buffer and its length match.
        let n = unsafe { GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32) };
        let class = String::from_utf16_lossy(&class[..n.max(0) as usize]);
        if matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") {
            return Foreground::None;
        }
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        // The visible frame, without the invisible resize borders around it.
        // SAFETY: `r` is a RECT and the size matches.
        let dwm = unsafe {
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS as u32,
                (&mut r as *mut RECT).cast(),
                std::mem::size_of::<RECT>() as u32,
            )
        };
        // SAFETY: valid handle and out pointer.
        if dwm != 0 && unsafe { GetWindowRect(hwnd, &mut r) } == 0 {
            return Foreground::None;
        }
        Foreground::Window(rect(r))
    }

    /// A full-screen game or video, or presentation mode.
    pub fn fullscreen_app() -> bool {
        let mut state = 0;
        // SAFETY: out pointer to a QUERY_USER_NOTIFICATION_STATE (i32).
        let ok = unsafe { SHQueryUserNotificationState(&mut state) } >= 0;
        ok && matches!(state, QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE)
    }

    fn ticks(f: FILETIME) -> u64 {
        (u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime)
    }

    static LAST_CPU: Mutex<Option<(u64, u64)>> = Mutex::new(None);

    fn cpu_percent() -> Option<f32> {
        let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let (mut idle, mut kernel, mut user) = (zero, zero, zero);
        // SAFETY: three valid out pointers.
        if unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } == 0 {
            return None;
        }
        // Kernel time includes idle time.
        let (idle, total) = (ticks(idle), ticks(kernel) + ticks(user));
        let mut last = LAST_CPU.lock().ok()?;
        let prev = last.replace((idle, total));
        let (pi, pt) = prev?;
        let dt = total.checked_sub(pt).filter(|d| *d > 0)?;
        let busy = dt.saturating_sub(idle.saturating_sub(pi));
        Some((busy as f32 / dt as f32 * 100.0).clamp(0.0, 100.0))
    }

    fn ram_percent() -> Option<f32> {
        // SAFETY: MEMORYSTATUSEX is plain data; zeroed is valid before the call.
        let mut m: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
        m.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        // SAFETY: `m` is valid with dwLength set.
        (unsafe { GlobalMemoryStatusEx(&mut m) } != 0).then_some(m.dwMemoryLoad as f32)
    }

    fn battery() -> Option<Battery> {
        // SAFETY: plain data out parameter.
        let mut s: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
        // SAFETY: valid out pointer.
        if unsafe { GetSystemPowerStatus(&mut s) } == 0 {
            return None;
        }
        // 128 = no system battery, 255 = unknown.
        if s.BatteryFlag == 128 || s.BatteryFlag == 255 || s.BatteryLifePercent > 100 {
            return None;
        }
        Some(Battery { percent: f32::from(s.BatteryLifePercent), charging: s.ACLineStatus == 1 })
    }

    fn disk_free_percent() -> Option<f32> {
        let mut dir = [0u16; 260];
        // SAFETY: buffer and its length match.
        let n = unsafe { GetWindowsDirectoryW(dir.as_mut_ptr(), dir.len() as u32) } as usize;
        if n < 3 {
            return None;
        }
        // "C:\" plus a terminating NUL.
        let root = [dir[0], dir[1], dir[2], 0];
        let (mut free, mut total, mut total_free) = (0u64, 0u64, 0u64);
        // SAFETY: NUL-terminated path and three valid out pointers.
        let ok = unsafe { GetDiskFreeSpaceExW(root.as_ptr(), &mut free, &mut total, &mut total_free) } != 0;
        (ok && total > 0).then(|| free as f32 / total as f32 * 100.0)
    }

    pub fn system_sample() -> Sample {
        Sample {
            cpu_percent: cpu_percent(),
            ram_percent: ram_percent(),
            battery: battery(),
            disk_free_percent: disk_free_percent(),
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::{Foreground, Rect};
    use pet_core::system::Sample;
    use std::sync::Mutex;

    pub fn cursor_pos() -> Option<(f64, f64)> {
        None
    }

    pub fn left_button_down() -> Option<bool> {
        None
    }

    pub fn user_idle_secs() -> u64 {
        0
    }

    pub fn work_area_near(_x: f64, _y: f64) -> Option<Rect> {
        None
    }

    pub fn foreground_window() -> Foreground {
        Foreground::None
    }

    pub fn fullscreen_app() -> bool {
        false
    }

    static LAST_CPU: Mutex<Option<(u64, u64)>> = Mutex::new(None);

    /// Linux only, for development: /proc readings.
    fn cpu_percent() -> Option<f32> {
        let stat = std::fs::read_to_string("/proc/stat").ok()?;
        let nums: Vec<u64> = stat.lines().next()?.split_whitespace().skip(1).filter_map(|n| n.parse().ok()).collect();
        let idle = nums.get(3)? + nums.get(4).unwrap_or(&0);
        let total: u64 = nums.iter().sum();
        let prev = LAST_CPU.lock().ok()?.replace((idle, total));
        let (pi, pt) = prev?;
        let dt = total.checked_sub(pt).filter(|d| *d > 0)?;
        Some((dt.saturating_sub(idle.saturating_sub(pi)) as f32 / dt as f32 * 100.0).clamp(0.0, 100.0))
    }

    fn ram_percent() -> Option<f32> {
        let info = std::fs::read_to_string("/proc/meminfo").ok()?;
        let field = |name: &str| -> Option<f32> {
            info.lines().find(|l| l.starts_with(name))?.split_whitespace().nth(1)?.parse().ok()
        };
        let (total, avail) = (field("MemTotal:")?, field("MemAvailable:")?);
        Some((1.0 - avail / total) * 100.0)
    }

    pub fn system_sample() -> Sample {
        Sample { cpu_percent: cpu_percent(), ram_percent: ram_percent(), battery: None, disk_free_percent: None }
    }
}

pub use imp::*;
