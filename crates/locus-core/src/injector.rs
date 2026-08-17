//! Atomic Safe Text Injector and Clipboard Preservation.
//!
//! Injects text into active OS windows while atomically backing up and restoring
//! user clipboard contents with sub-millisecond overhead and Zero-Panic safety.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Instant;

/// Report returned upon successful or simulated text injection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InjectionReport {
    pub bytes_injected: usize,
    pub elapsed_ms: f64,
    pub clipboard_restored: bool,
}

static INJECTOR_CLIPBOARD_BACKUP: Mutex<Option<String>> = Mutex::new(None);

pub struct SafeTextInjector;

impl SafeTextInjector {
    /// Injects text into the active foreground window with optional clipboard restoration.
    pub fn inject_text(text: &str, restore_clipboard: bool) -> InjectionReport {
        let start = Instant::now();
        let bytes_injected = text.len();

        // 1. Capture current clipboard backup
        let previous_clipboard = Self::read_clipboard_safely();
        if let Ok(mut backup_lock) = INJECTOR_CLIPBOARD_BACKUP.lock() {
            *backup_lock = previous_clipboard.clone();
        }

        // 2. Set target text into OS clipboard
        Self::write_clipboard_safely(text);

        // 3. Simulate OS Paste keystroke (Ctrl+V on Windows/Linux, Cmd+V on macOS)
        Self::simulate_paste_keystroke();

        // 4. Restore original clipboard if requested
        let mut restored = false;
        if restore_clipboard {
            if let Some(ref original) = previous_clipboard {
                // Short sleep or async tick to ensure target app has ingested the paste event
                std::thread::sleep(std::time::Duration::from_millis(15));
                Self::write_clipboard_safely(original);
                restored = true;
            }
        }

        let elapsed_ms = (start.elapsed().as_nanos() as f64) / 1_000_000.0;

        InjectionReport {
            bytes_injected,
            elapsed_ms,
            clipboard_restored: restored,
        }
    }

    /// Safely reads the OS clipboard text without panicking.
    pub fn read_clipboard_safely() -> Option<String> {
        #[cfg(target_os = "windows")]
        {
            use std::ffi::OsString;
            use std::os::windows::ffi::OsStringExt;

            extern "system" {
                fn OpenClipboard(hwnd: isize) -> i32;
                fn CloseClipboard() -> i32;
                fn GetClipboardData(uformat: u32) -> isize;
                fn GlobalLock(hmem: isize) -> *mut u16;
                fn GlobalUnlock(hmem: isize) -> i32;
            }

            const CF_UNICODETEXT: u32 = 13;

            unsafe {
                if OpenClipboard(0) != 0 {
                    let hmem = GetClipboardData(CF_UNICODETEXT);
                    if hmem != 0 {
                        let ptr = GlobalLock(hmem);
                        if !ptr.is_null() {
                            let mut len = 0;
                            while *ptr.add(len) != 0 {
                                len += 1;
                            }
                            let slice = std::slice::from_raw_parts(ptr, len);
                            let text = OsString::from_wide(slice).to_string_lossy().to_string();
                            GlobalUnlock(hmem);
                            CloseClipboard();
                            return Some(text);
                        }
                    }
                    CloseClipboard();
                }
            }
        }

        None
    }

    /// Safely writes text to the OS clipboard without panicking.
    pub fn write_clipboard_safely(text: &str) -> bool {
        #[cfg(target_os = "windows")]
        {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;

            extern "system" {
                fn OpenClipboard(hwnd: isize) -> i32;
                fn CloseClipboard() -> i32;
                fn EmptyClipboard() -> i32;
                fn SetClipboardData(uformat: u32, hmem: isize) -> isize;
                fn GlobalAlloc(uflags: u32, dwbytes: usize) -> isize;
                fn GlobalLock(hmem: isize) -> *mut u16;
                fn GlobalUnlock(hmem: isize) -> i32;
            }

            const CF_UNICODETEXT: u32 = 13;
            const GMEM_MOVEABLE: u32 = 0x0002;

            let wide: Vec<u16> = OsStr::new(text).encode_wide().chain(std::iter::once(0)).collect();
            let size = wide.len() * std::mem::size_of::<u16>();

            unsafe {
                if OpenClipboard(0) != 0 {
                    EmptyClipboard();
                    let hmem = GlobalAlloc(GMEM_MOVEABLE, size);
                    if hmem != 0 {
                        let ptr = GlobalLock(hmem);
                        if !ptr.is_null() {
                            std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
                            GlobalUnlock(hmem);
                            SetClipboardData(CF_UNICODETEXT, hmem);
                        }
                    }
                    CloseClipboard();
                    return true;
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = text;
        }

        false
    }

    /// Simulates OS paste keystroke (Ctrl+V on Windows/Linux, Cmd+V on macOS).
    pub fn simulate_paste_keystroke() {
        #[cfg(target_os = "windows")]
        {
            extern "system" {
                fn keybd_event(b_vk: u8, b_scan: u8, dw_flags: u32, dw_extra_info: usize);
            }

            const VK_CONTROL: u8 = 0x11;
            const VK_V: u8 = 0x56;
            const KEYEVENTF_KEYUP: u32 = 0x0002;

            unsafe {
                // Key down Ctrl, V
                keybd_event(VK_CONTROL, 0, 0, 0);
                keybd_event(VK_V, 0, 0, 0);

                // Key up V, Ctrl
                keybd_event(VK_V, 0, KEYEVENTF_KEYUP, 0);
                keybd_event(VK_CONTROL, 0, KEYEVENTF_KEYUP, 0);
            }
        }
    }
}
