//! Pen pressure from the native pointer stack.
//!
//! Windows delivers tablet pressure through `WM_POINTER` messages. A window subclass records the
//! most recent pen pressure so the drawing loop can attach it to each stroke sample. No pointer
//! data is persisted, logged, or sent anywhere.

use raw_window_handle::WindowHandle;

#[cfg(target_os = "windows")]
mod imp {
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    use raw_window_handle::{RawWindowHandle, WindowHandle};
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::UI::Input::Pointer::{
        GetPointerPenInfo, GetPointerType, POINTER_PEN_INFO,
    };
    use windows_sys::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        PT_PEN, WM_POINTERDOWN, WM_POINTERLEAVE, WM_POINTERUP, WM_POINTERUPDATE,
    };

    /// Pressure in permille (0—1000) of the last pen sample.
    static PRESSURE: AtomicU32 = AtomicU32::new(0);
    /// True between pen down and pen up.
    static ACTIVE: AtomicBool = AtomicBool::new(false);
    /// True once any pen was seen, so callers can tell "no tablet" from "pen up".
    static SEEN: AtomicBool = AtomicBool::new(false);
    const SUBCLASS_ID: usize = 0x4456_5045;

    unsafe extern "system" fn pen_message(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _subclass_id: usize,
        _ref_data: usize,
    ) -> LRESULT {
        if matches!(
            msg,
            WM_POINTERDOWN | WM_POINTERUPDATE | WM_POINTERUP | WM_POINTERLEAVE
        ) {
            // GET_POINTERID_WPARAM: the low word of wParam is the pointer id.
            let pointer_id = (wparam & 0xffff) as u32;
            let mut pointer_type = PT_PEN;
            // SAFETY: Win32 pointer queries only read state for the id carried by this message.
            let is_pen = unsafe { GetPointerType(pointer_id, &mut pointer_type) } != 0
                && pointer_type == PT_PEN;
            if is_pen {
                if msg == WM_POINTERUP || msg == WM_POINTERLEAVE {
                    ACTIVE.store(false, Ordering::Relaxed);
                    PRESSURE.store(0, Ordering::Relaxed);
                } else {
                    // SAFETY: `POINTER_PEN_INFO` is a plain C struct valid when zeroed.
                    let mut info: POINTER_PEN_INFO = unsafe { std::mem::zeroed() };
                    // SAFETY: the same pointer id was just confirmed to be a pen.
                    if unsafe { GetPointerPenInfo(pointer_id, &mut info) } != 0 {
                        SEEN.store(true, Ordering::Relaxed);
                        ACTIVE.store(true, Ordering::Relaxed);
                        let pressure = info.pressure.min(1024) * 1000 / 1024;
                        PRESSURE.store(pressure, Ordering::Relaxed);
                    }
                }
            }
        }
        // SAFETY: every message must be forwarded to the original window procedure.
        unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
    }

    pub fn install(window: WindowHandle<'_>) -> bool {
        let RawWindowHandle::Win32(win32) = window.as_raw() else {
            return false;
        };
        // SAFETY: the HWND comes from the live eframe window; the subclass only reads pointer
        // state, forwards every message, and stays installed for the window's lifetime.
        unsafe {
            SetWindowSubclass(win32.hwnd.get() as HWND, Some(pen_message), SUBCLASS_ID, 0) != 0
        }
    }

    pub fn pressure() -> Option<f32> {
        if !SEEN.load(Ordering::Relaxed) || !ACTIVE.load(Ordering::Relaxed) {
            return None;
        }
        Some(PRESSURE.load(Ordering::Relaxed) as f32 / 1000.0)
    }

    pub fn available() -> bool {
        SEEN.load(Ordering::Relaxed)
    }
}

/// Installs the pressure probe on a live window. Returns false when the platform or target window
/// cannot provide one; drawing still works with simulated pressure.
pub fn install_pen_input(window: WindowHandle<'_>) -> bool {
    #[cfg(target_os = "windows")]
    {
        imp::install(window)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = window;
        false
    }
}

/// Pressure of the active pen sample, normalized to `0.0..=1.0`.
///
/// `None` means no pen is currently touching the screen; callers fall back to pressure simulated
/// from stroke speed and progress.
#[must_use]
pub fn pen_pressure() -> Option<f32> {
    #[cfg(target_os = "windows")]
    {
        imp::pressure()
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// Whether a tablet pen has been observed since the window was created.
#[must_use]
pub fn pen_available() -> bool {
    #[cfg(target_os = "windows")]
    {
        imp::available()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}
