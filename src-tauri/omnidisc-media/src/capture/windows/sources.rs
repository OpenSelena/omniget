use crate::stream::{AudioApp, SourceId, StreamError, StreamSource};
use base64::Engine;
use std::ffi::c_void;
use windows::core::BOOL;
use windows::core::PWSTR;
use windows::Win32::Foundation::{FALSE, HWND, LPARAM, RECT, TRUE};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, EnumDisplayMonitors, GetDC,
    GetMonitorInfoW, ReleaseDC, SelectObject, SetStretchBltMode, StretchBlt, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HALFTONE, HDC, HGDIOBJ, HMONITOR, MONITORINFO,
    MONITORINFOEXW, SRCCOPY,
};
use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetAncestor, GetWindowLongW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, GA_ROOT, GWL_EXSTYLE, GWL_STYLE,
    MONITORINFOF_PRIMARY, WS_CHILD, WS_EX_TOOLWINDOW,
};

const THUMB_WIDTH: u32 = 320;
const PW_RENDERFULLCONTENT: PRINT_WINDOW_FLAGS = PRINT_WINDOW_FLAGS(2);
const MIN_WINDOW_EDGE: i32 = 64;

pub struct Monitor {
    pub handle: HMONITOR,
    pub rect: RECT,
    pub primary: bool,
}

// SAFETY: HMONITOR is a plain handle; it is only used to build a capture item.
unsafe impl Send for Monitor {}

unsafe extern "system" fn monitor_proc(
    monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let list = unsafe { &mut *(data.0 as *mut Vec<Monitor>) };
    let mut info = MONITORINFOEXW {
        monitorInfo: MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    let ok = unsafe {
        GetMonitorInfoW(
            monitor,
            &mut info as *mut MONITORINFOEXW as *mut MONITORINFO,
        )
    };
    if ok.as_bool() {
        list.push(Monitor {
            handle: monitor,
            rect: info.monitorInfo.rcMonitor,
            primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
        });
    }
    TRUE
}

pub fn monitors() -> Vec<Monitor> {
    let mut list: Vec<Monitor> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(monitor_proc),
            LPARAM(&mut list as *mut Vec<Monitor> as isize),
        );
    }
    list
}

pub struct Window {
    pub handle: HWND,
    pub title: String,
    pub app: Option<String>,
    pub width: u32,
    pub height: u32,
}

// SAFETY: HWND is a plain handle; used only to build a capture item.
unsafe impl Send for Window {}

fn window_text(hwnd: HWND) -> String {
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return String::new();
    }
    let mut buf = vec![0u16; len as usize + 1];
    let written = unsafe { GetWindowTextW(hwnd, &mut buf) };
    if written <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..written as usize])
}

fn is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked: u32 = 0;
    let ok = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut c_void,
            std::mem::size_of::<u32>() as u32,
        )
    };
    ok.is_ok() && cloaked != 0
}

pub fn process_path(pid: u32) -> Option<String> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let mut buf = vec![0u16; 512];
    let mut len = buf.len() as u32;
    let ok = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
    };
    let _ = unsafe { windows::Win32::Foundation::CloseHandle(handle) };
    if ok.is_err() || len == 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&buf[..len as usize]))
}

pub fn process_name(pid: u32) -> Option<String> {
    let path = process_path(pid)?;
    let stem = path
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(&path)
        .trim_end_matches(".exe")
        .trim_end_matches(".EXE")
        .to_string();
    if stem.is_empty() {
        None
    } else {
        Some(stem)
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, data: LPARAM) -> BOOL {
    let ctx = unsafe { &mut *(data.0 as *mut (Vec<Window>, u32)) };
    let (list, own_pid) = (&mut ctx.0, ctx.1);
    if list.len() >= 128 {
        return FALSE;
    }
    if !unsafe { IsWindowVisible(hwnd) }.as_bool() || unsafe { IsIconic(hwnd) }.as_bool() {
        return TRUE;
    }
    if unsafe { GetAncestor(hwnd, GA_ROOT) } != hwnd {
        return TRUE;
    }
    let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;
    if style & WS_CHILD.0 != 0 {
        return TRUE;
    }
    let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32;
    if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
        return TRUE;
    }
    if is_cloaked(hwnd) {
        return TRUE;
    }
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == 0 || pid == own_pid {
        return TRUE;
    }
    let mut rect = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
        return TRUE;
    }
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;
    if w < MIN_WINDOW_EDGE || h < MIN_WINDOW_EDGE {
        return TRUE;
    }
    let title = window_text(hwnd);
    let app = process_name(pid);
    if title.trim().is_empty() && app.is_none() {
        return TRUE;
    }
    list.push(Window {
        handle: hwnd,
        title,
        app,
        width: w as u32,
        height: h as u32,
    });
    TRUE
}

pub fn windows_list() -> Vec<Window> {
    let mut ctx: (Vec<Window>, u32) = (Vec::new(), std::process::id());
    unsafe {
        let _ = EnumWindows(
            Some(window_proc),
            LPARAM(&mut ctx as *mut (Vec<Window>, u32) as isize),
        );
    }
    ctx.0
}

struct Dib {
    dc: HDC,
    bitmap: windows::Win32::Graphics::Gdi::HBITMAP,
    old: HGDIOBJ,
    bits: *mut c_void,
    width: i32,
    height: i32,
}

impl Dib {
    fn new(reference: HDC, width: i32, height: i32) -> Option<Self> {
        if width <= 0 || height <= 0 {
            return None;
        }
        let dc = unsafe { CreateCompatibleDC(Some(reference)) };
        if dc.is_invalid() {
            return None;
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                // Negative height gives a top-down DIB, which matches the row
                // order the JPEG encoder expects.
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let bitmap =
            unsafe { CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0) };
        let bitmap = match bitmap {
            Ok(b) if !bits.is_null() => b,
            _ => {
                unsafe {
                    let _ = DeleteDC(dc);
                };
                return None;
            }
        };
        let old = unsafe { SelectObject(dc, bitmap.into()) };
        Some(Self {
            dc,
            bitmap,
            old,
            bits,
            width,
            height,
        })
    }

    fn to_jpeg_data_url(&self) -> Option<String> {
        let pixels = self.width as usize * self.height as usize * 4;
        let bgra = unsafe { std::slice::from_raw_parts(self.bits as *const u8, pixels) };
        let mut rgb = Vec::with_capacity(self.width as usize * self.height as usize * 3);
        for px in bgra.chunks_exact(4) {
            rgb.push(px[2]);
            rgb.push(px[1]);
            rgb.push(px[0]);
        }
        let mut jpeg = Vec::new();
        let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 60);
        enc.encode(
            &rgb,
            self.width as u32,
            self.height as u32,
            image::ExtendedColorType::Rgb8,
        )
        .ok()?;
        Some(format!(
            "data:image/jpeg;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(jpeg)
        ))
    }
}

impl Drop for Dib {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

fn thumb_size(width: u32, height: u32) -> (i32, i32) {
    let w = THUMB_WIDTH.min(width.max(1));
    let h = ((height as u64 * w as u64) / width.max(1) as u64).clamp(1, THUMB_WIDTH as u64) as u32;
    (w as i32, h as i32)
}

pub fn monitor_thumbnail(m: &Monitor) -> Option<String> {
    let w = m.rect.right - m.rect.left;
    let h = m.rect.bottom - m.rect.top;
    if w <= 0 || h <= 0 {
        return None;
    }
    let screen = unsafe { GetDC(None) };
    if screen.is_invalid() {
        return None;
    }
    let (tw, th) = thumb_size(w as u32, h as u32);
    let out = (|| {
        let dib = Dib::new(screen, tw, th)?;
        unsafe {
            SetStretchBltMode(dib.dc, HALFTONE);
            StretchBlt(
                dib.dc,
                0,
                0,
                tw,
                th,
                Some(screen),
                m.rect.left,
                m.rect.top,
                w,
                h,
                SRCCOPY,
            )
            .ok()
            .ok()?;
        }
        dib.to_jpeg_data_url()
    })();
    unsafe { ReleaseDC(None, screen) };
    out
}

pub fn window_thumbnail(win: &Window) -> Option<String> {
    let screen = unsafe { GetDC(None) };
    if screen.is_invalid() {
        return None;
    }
    let (tw, th) = thumb_size(win.width, win.height);
    let out = (|| {
        let full = Dib::new(screen, win.width as i32, win.height as i32)?;
        let printed = unsafe { PrintWindow(win.handle, full.dc, PW_RENDERFULLCONTENT) }.as_bool();
        if !printed {
            return None;
        }
        let thumb = Dib::new(screen, tw, th)?;
        unsafe {
            SetStretchBltMode(thumb.dc, HALFTONE);
            StretchBlt(
                thumb.dc,
                0,
                0,
                tw,
                th,
                Some(full.dc),
                0,
                0,
                win.width as i32,
                win.height as i32,
                SRCCOPY,
            )
            .ok()
            .ok()?;
        }
        thumb.to_jpeg_data_url()
    })();
    unsafe { ReleaseDC(None, screen) };
    out
}

/// HWNDs are 32-bit significant by contract (USER handles are sign-extended on
/// 64-bit Windows), so the frontend's `u32` source id round-trips exactly.
pub fn window_id(hwnd: HWND) -> u32 {
    hwnd.0 as usize as u32
}

pub fn window_from_id(id: u32) -> Option<HWND> {
    // Rebuild the handle rather than re-enumerating: a window that was
    // minimised or lost its title between the picker and the start of the share
    // is still a valid capture target.
    let hwnd = HWND(id as usize as *mut c_void);
    if unsafe { IsWindow(Some(hwnd)) }.as_bool() {
        Some(hwnd)
    } else {
        None
    }
}

pub fn display_sources(thumbnails: bool) -> Vec<StreamSource> {
    monitors()
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let w = (m.rect.right - m.rect.left).max(0) as u32;
            let h = (m.rect.bottom - m.rect.top).max(0) as u32;
            StreamSource {
                id: SourceId::Display { id: i as u32 + 1 },
                title: if m.primary {
                    format!("Display {} ({w}×{h}, primary)", i + 1)
                } else {
                    format!("Display {} ({w}×{h})", i + 1)
                },
                app_name: None,
                width: w,
                height: h,
                thumbnail: if thumbnails {
                    monitor_thumbnail(m)
                } else {
                    None
                },
            }
        })
        .collect()
}

pub fn window_sources(thumbnails: bool) -> Vec<StreamSource> {
    let mut out = Vec::new();
    for (i, w) in windows_list().into_iter().enumerate() {
        let title = if w.title.trim().is_empty() {
            w.app.clone().unwrap_or_default()
        } else {
            w.title.clone()
        };
        out.push(StreamSource {
            id: SourceId::Window {
                id: window_id(w.handle),
            },
            title,
            app_name: w.app.clone(),
            width: w.width,
            height: w.height,
            thumbnail: if thumbnails && i < 16 {
                window_thumbnail(&w)
            } else {
                None
            },
        });
        if out.len() >= 40 {
            break;
        }
    }
    out
}

pub fn monitor_from_id(id: u32) -> Result<Monitor, StreamError> {
    let mut list = monitors();
    if id == 0 || id as usize > list.len() {
        return Err(StreamError::SourceGone);
    }
    Ok(list.remove(id as usize - 1))
}

pub fn audio_app(pid: u32) -> Option<AudioApp> {
    let path = process_path(pid)?;
    let name = path
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(&path)
        .trim_end_matches(".exe")
        .trim_end_matches(".EXE")
        .to_string();
    if name.is_empty() {
        return None;
    }
    Some(AudioApp {
        pid: pid as i32,
        name,
        bundle_id: path,
    })
}
