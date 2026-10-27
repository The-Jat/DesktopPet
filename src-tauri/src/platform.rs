//! The few OS calls the pet needs, all in physical screen pixels.
//!
//! Windows is the target platform. Elsewhere these return `None`/0 so the app
//! still builds and runs for development, just without dragging, click-through
//! or idle detection.

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
}

#[cfg(windows)]
mod imp {
    use super::Rect;
    use windows_sys::Win32::Foundation::{POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, GetLastInputInfo, LASTINPUTINFO, VK_LBUTTON,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

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
        let r = info.rcWork;
        ok.then(|| Rect { left: r.left.into(), top: r.top.into(), right: r.right.into(), bottom: r.bottom.into() })
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Rect;

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
}

pub use imp::*;
