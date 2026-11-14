//! Fenêtres de l'overlay : HUD et pop-up d'alerte. Layered (alpha par pixel), transparentes
//! aux clics, topmost, sans prise de focus, sur le moniteur de la fenêtre au premier plan.

use ovl_core::hud::{Cell, Corner, Layout, Palette, Row};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetDC, GetMonitorInfoW, MonitorFromWindow, ReleaseDC, AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION,
    MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::HiDpi::{
    GetDpiForMonitor, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetForegroundWindow, GetWindowLongW,
    PeekMessageW, RegisterClassW, SetWindowPos, ShowWindow, TranslateMessage, UpdateLayeredWindow,
    GWL_EXSTYLE, HWND_NOTOPMOST, HWND_TOPMOST, MSG, PM_REMOVE, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SW_HIDE, SW_SHOWNOACTIVATE, ULW_ALPHA, WM_HOTKEY, WM_QUIT, WNDCLASSW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

use crate::painter::{Renderer, Scene};

/// Apparence demandée par la configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudSettings {
    pub scale: f32,
    /// Pixels logiques.
    pub margin_x: i32,
    pub margin_y: i32,
    pub corner: Corner,
    pub layout: Layout,
    pub palette: Palette,
    pub opacity: f32,
}

/// Ce que `pump` a reçu depuis le dernier appel.
#[derive(Debug, Default, Clone, Copy)]
pub struct Pumped {
    pub quit: bool,
    /// Identifiant du raccourci global déclenché.
    pub hotkey: Option<i32>,
}

/// Moniteur de la fenêtre au premier plan : zone de travail et facteur DPI.
fn target_monitor() -> Option<(RECT, f32)> {
    unsafe {
        let monitor = MonitorFromWindow(GetForegroundWindow(), MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return None;
        }
        let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
        let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
        Some((info.rcWork, dpi_x as f32 / 96.0))
    }
}

/// Une fenêtre layered et son moteur de rendu.
struct Layer {
    hwnd: HWND,
    renderer: Option<Renderer>,
    visible: bool,
    /// Échec de passage au premier plan déjà signalé.
    warned: bool,
}

impl Layer {
    fn create(class: PCWSTR, title: PCWSTR) -> Option<Self> {
        unsafe {
            // topmost dès la création : un SetWindowPos(HWND_TOPMOST) ultérieur peut être ignoré
            // par Windows (constaté : le HUD restait sous un jeu plein écran fenêtré)
            let ex_style = WS_EX_LAYERED
                | WS_EX_TRANSPARENT
                | WS_EX_TOOLWINDOW
                | WS_EX_NOACTIVATE
                | WS_EX_TOPMOST;
            match CreateWindowExW(
                ex_style, class, title, WS_POPUP, 0, 0, 1, 1, None, None, None, None,
            ) {
                Ok(hwnd) => Some(Self {
                    hwnd,
                    renderer: None,
                    visible: false,
                    warned: false,
                }),
                Err(err) => {
                    log::error!("CreateWindowExW: {err}");
                    None
                }
            }
        }
    }

    fn renderer(&mut self, k: f32) -> Option<&mut Renderer> {
        if self
            .renderer
            .as_ref()
            .map_or(true, |r| (r.k() - k).abs() > 0.001)
        {
            self.renderer = Renderer::new(k);
        }
        self.renderer.as_mut()
    }

    /// Dessine la scène et place la fenêtre ; `place` reçoit la taille et renvoie la position.
    fn draw(&mut self, k: f32, scene: &Scene, place: impl FnOnce(i32, i32) -> (i32, i32)) {
        let hwnd = self.hwnd;
        let Some(renderer) = self.renderer(k) else {
            return;
        };
        let Some(image) = renderer.render(scene) else {
            return;
        };
        let (x, y) = place(image.width, image.height);
        unsafe {
            let screen = GetDC(None);
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let _ = UpdateLayeredWindow(
                hwnd,
                screen,
                Some(&POINT { x, y }),
                Some(&SIZE {
                    cx: image.width,
                    cy: image.height,
                }),
                image.dc,
                Some(&POINT { x: 0, y: 0 }),
                windows::Win32::Foundation::COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
            ReleaseDC(None, screen);
        }
    }

    fn show(&mut self) {
        unsafe {
            if !self.visible {
                let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
                self.visible = true;
            }
            // un jeu plein écran peut repasser devant : on réaffirme le premier plan à chaque rendu
            let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
            let topmost = |hwnd| GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0;
            if !topmost(self.hwnd) {
                // drapeau perdu : Windows n'accepte de le reposer qu'après un passage en non-topmost
                let _ = SetWindowPos(self.hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, flags);
            }
            let _ = SetWindowPos(self.hwnd, HWND_TOPMOST, 0, 0, 0, 0, flags);
            if !topmost(self.hwnd) && !self.warned {
                self.warned = true;
                log::warn!("le HUD n'a pas pu passer au premier plan (topmost)");
            }
        }
    }

    fn hide(&mut self) {
        if self.visible {
            unsafe {
                let _ = ShowWindow(self.hwnd, SW_HIDE);
            }
            self.visible = false;
        }
    }
}

pub struct Overlay {
    hud: Layer,
    popup: Layer,
}

impl Overlay {
    /// Crée les fenêtres, masquées.
    pub fn create() -> Option<Self> {
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            let class = w!("FPSOverlayHud");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                lpszClassName: class,
                ..Default::default()
            };
            RegisterClassW(&wc);
            Some(Self {
                hud: Layer::create(class, w!("FPS Overlay HUD"))?,
                popup: Layer::create(class, w!("FPS Overlay alerte"))?,
            })
        }
    }

    /// Dessine le HUD dans le coin choisi, et l'affiche.
    pub fn render(
        &mut self,
        settings: &HudSettings,
        rows: &[Row],
        graph: &[f32],
        alert: Option<&str>,
        flash: bool,
    ) {
        let Some((work, dpi)) = target_monitor() else {
            return;
        };
        let k = settings.scale * dpi;
        let scene = Scene {
            rows,
            layout: settings.layout,
            palette: settings.palette,
            opacity: settings.opacity,
            graph,
            alert,
            flash,
        };
        let mx = (settings.margin_x as f32 * dpi).round() as i32;
        let my = (settings.margin_y as f32 * dpi).round() as i32;
        let corner = settings.corner;
        self.hud.draw(k, &scene, |w, h| {
            let x = if corner.is_left() {
                work.left + mx
            } else {
                work.right - w - mx
            };
            let y = if corner.is_top() {
                work.top + my
            } else {
                work.bottom - h - my
            };
            (x, y)
        });
        self.hud.show();
    }

    /// Pop-up centré en haut de l'écran du jeu ; `None` le masque.
    pub fn popup(&mut self, settings: &HudSettings, text: Option<&str>) {
        let Some(text) = text else {
            self.popup.hide();
            return;
        };
        let Some((work, dpi)) = target_monitor() else {
            return;
        };
        let mut palette = settings.palette;
        palette.value = palette.alert;
        let rows = [Row {
            label: "Alerte",
            cell: Cell::Text {
                value: text.to_owned(),
                detail: None,
            },
        }];
        let scene = Scene {
            rows: &rows,
            layout: Layout::Line,
            palette,
            opacity: settings.opacity.max(0.85),
            graph: &[],
            alert: None,
            flash: false,
        };
        self.popup.draw(1.3 * dpi, &scene, |w, _| {
            (
                (work.left + work.right - w) / 2,
                work.top + (work.bottom - work.top) / 12,
            )
        });
        self.popup.show();
    }

    pub fn hide(&mut self) {
        self.hud.hide();
        self.popup.hide();
    }

    /// Traite les messages en attente du thread.
    pub fn pump() -> Pumped {
        let mut out = Pumped::default();
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                match msg.message {
                    WM_QUIT => out.quit = true,
                    WM_HOTKEY => out.hotkey = Some(msg.wParam.0 as i32),
                    _ => {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }
        }
        out
    }
}

/// Le contenu est fourni par `UpdateLayeredWindow` : aucune peinture à faire ici.
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    DefWindowProcW(hwnd, msg, wp, lp)
}
