//! Fenêtre d'overlay : layered, transparente aux clics, topmost, suit le moniteur actif.

use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, UpdateWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::HiDpi::{
    GetDpiForWindow, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetForegroundWindow, GetMessageW,
    PeekMessageW, RegisterClassW, SetLayeredWindowAttributes, SetWindowPos, ShowWindow,
    TranslateMessage, CS_HREDRAW, CS_VREDRAW, HWND_TOPMOST, LWA_ALPHA, MSG, PM_REMOVE,
    SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE, SW_SHOW, WM_QUIT, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP,
};

use crate::painter::{PaintCtx, Theme};

#[derive(Clone, Copy)]
pub struct OverlayConfig {
    pub width: i32,
    pub height: i32,
    pub margin: i32,
    pub scale: f32,
    pub theme: Theme,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self { width: 200, height: 150, margin: 12, scale: 1.0, theme: Theme::dark() }
    }
}

static mut CTX: *mut PaintCtx = std::ptr::null_mut();
static mut ROWS: Option<Vec<(String, String)>> = None;

pub struct Overlay {
    hwnd: HWND,
    config: OverlayConfig,
    last_pos: Option<(i32, i32, i32, i32)>,
}

impl Overlay {
    pub fn create(config: OverlayConfig) -> Option<Self> {
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

            let class_name = w!("KoykOverlayRust");
            // fond noir = couleur transparente (color key), dès la création
            let black: windows::Win32::Graphics::Gdi::HBRUSH =
                windows::Win32::Graphics::Gdi::CreateSolidBrush(COLORREF(0));
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wndproc),
                lpszClassName: class_name,
                hbrBackground: black,
                ..Default::default()
            };
            RegisterClassW(&wc);

            let ex_style = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
            let hwnd = match CreateWindowExW(
                ex_style, class_name, w!("FPS Counter"), WS_POPUP,
                0, 0, config.width, config.height, None, None, None, None,
            ) {
                Ok(hwnd) => hwnd,
                Err(err) => {
                    log::error!("CreateWindowExW: {err}");
                    return None;
                }
            };

            // panneau translucide : lisibilité garantie sur tous les fonds
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 205, LWA_ALPHA);
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = UpdateWindow(hwnd);

            let ctx = PaintCtx::new(hwnd, config.scale)?;
            unsafe {
                CTX = Box::into_raw(Box::new(ctx));
            }
            Some(Self { hwnd, config, last_pos: None })
        }
    }

    /// Positionne la fenêtre en haut à droite du moniteur de la fenêtre au premier plan.
    pub fn follow_foreground_monitor(&mut self) {
        unsafe {
            let front = GetForegroundWindow();
            let monitor = MonitorFromWindow(front, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(monitor, &mut info).as_bool() {
                let rect: RECT = info.rcWork;
                let dpi = GetDpiForWindow(self.hwnd) as f32 / 96.0;
                let w = (self.config.width as f32 * dpi) as i32;
                let h = (self.config.height as f32 * dpi) as i32;
                let m = (self.config.margin as f32 * dpi) as i32;
                let x = rect.right - w - m;
                let y = rect.top + m;
                let target = (x, y, w, h);
                // ne pas repositionner si rien ne change : sinon la fenêtre est
                // réinitialisée (fond noir) juste après la peinture
                if self.last_pos != Some(target) {
                    self.last_pos = Some(target);
                    let _ = SetWindowPos(self.hwnd, HWND_TOPMOST, x, y, w, h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
                }
            }
        }
    }

    pub fn paint(&mut self, lines: &[(String, String)]) {
        unsafe {
            // peinture directe : sur une fenêtre layered, WM_PAINT ne reçoit pas de surface utile
            let hdc = windows::Win32::Graphics::Gdi::GetDC(self.hwnd);
            if !hdc.is_invalid() {
                if !CTX.is_null() {
                    (*CTX).paint_dc(hdc, lines);
                }
                windows::Win32::Graphics::Gdi::ReleaseDC(self.hwnd, hdc);
            }
        }
    }

    /// Change l'échelle du panneau (reconstruit les polices).
    pub fn set_scale(&mut self, scale: f32) {
        if (self.config.scale - scale).abs() < 0.01 {
            return;
        }
        self.config.scale = scale;
        if let Some(ctx) = PaintCtx::new(self.hwnd, scale) {
            unsafe {
                if !CTX.is_null() {
                    drop(Box::from_raw(CTX));
                }
                CTX = Box::into_raw(Box::new(ctx));
            }
        }
    }

    pub fn hide(&mut self) {
        unsafe { let _ = ShowWindow(self.hwnd, SW_HIDE); }
    }

    pub fn show(&mut self) {
        unsafe { let _ = ShowWindow(self.hwnd, SW_SHOW); }
    }

    /// Traite les messages en attente ; false si l'application doit quitter.
    pub fn pump() -> bool {
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    return false;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            true
        }
    }

    pub fn run_loop() {
        unsafe {
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        windows::Win32::UI::WindowsAndMessaging::WM_PAINT => {
            use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, PAINTSTRUCT};
            let mut ps = PAINTSTRUCT::default();
            let _hdc = BeginPaint(hwnd, &mut ps);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        windows::Win32::UI::WindowsAndMessaging::WM_ERASEBKGND => LRESULT(1),
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}