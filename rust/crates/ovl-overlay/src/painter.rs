//! Rendu du panneau (GDI : 6 lignes de texte, coût négligeable).

use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateFontW, CreateSolidBrush, DeleteObject, DrawTextW, FillRect, GetDC, ReleaseDC,
    SelectObject, SetBkMode, SetTextColor, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET,
    DT_LEFT, DT_SINGLELINE, DT_VCENTER, FF_DONTCARE, HBRUSH, HDC, HFONT, HGDIOBJ,
    OUT_DEFAULT_PRECIS, TRANSPARENT,
};
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

#[derive(Clone, Copy)]
pub struct Theme {
    pub background: u32,
    pub text: u32,
    pub label: u32,
}

impl Theme {
    pub fn dark() -> Self {
        Self { background: 0x0000_0000, text: 0x00F0_F0F0, label: 0x0090_9090 }
    }
}

pub struct PaintCtx {
    hwnd: HWND,
    font_label: HFONT,
    font_value: HFONT,
    brush_bg: HBRUSH,
    theme: Theme,
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

impl PaintCtx {
    pub fn new(hwnd: HWND, scale: f32) -> Option<Self> {
        unsafe {
            let mut family: Vec<u16> = wide("Segoe UI Variable Display");
            family.push(0);

            let mut make_font = |size: i32| -> HFONT {
                CreateFontW(
                    -size,
                    0,
                    0,
                    0,
                    700,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET.0 as u32,
                    OUT_DEFAULT_PRECIS.0 as u32,
                    CLIP_DEFAULT_PRECIS.0 as u32,
                    CLEARTYPE_QUALITY.0 as u32,
                    FF_DONTCARE.0 as u32,
                    PCWSTR(family.as_ptr()),
                )
            };

            let font_label = make_font((11.0 * scale) as i32);
            let font_value = make_font((15.0 * scale) as i32);
            if font_label.0.is_null() || font_value.0.is_null() {
                return None;
            }

            let theme = Theme::dark();
            let brush_bg = CreateSolidBrush(COLORREF(theme.background));

            Some(Self { hwnd, font_label, font_value, brush_bg, theme })
        }
    }

    /// Peint les lignes (label, valeur) sur deux colonnes.
    pub fn paint(&mut self, lines: &[(String, String)]) {
        unsafe {
            let hdc: HDC = GetDC(self.hwnd);
            if hdc.is_invalid() {
                return;
            }

            let mut rect = RECT { left: 0, top: 0, right: 400, bottom: 300 };
            let _ = GetClientRect(self.hwnd, &mut rect);
            FillRect(hdc, &rect, self.brush_bg);
            SetBkMode(hdc, TRANSPARENT);

            let pad = 10;
            let row_h = 20;
            let col_value = 78;
            let mut y = pad;

            for (label, value) in lines {
                SetTextColor(hdc, COLORREF(self.theme.label));
                let previous = SelectObject(hdc, HGDIOBJ(self.font_label.0));
                let mut r = RECT { left: pad, top: y, right: col_value - 4, bottom: y + row_h };
                let mut text = wide(label);
                DrawTextW(hdc, &mut text, &mut r, DT_LEFT | DT_SINGLELINE | DT_VCENTER);

                SetTextColor(hdc, COLORREF(self.theme.text));
                SelectObject(hdc, HGDIOBJ(self.font_value.0));
                let mut r2 = RECT { left: col_value, top: y, right: rect.right - pad, bottom: y + row_h };
                let mut text2 = wide(value);
                DrawTextW(hdc, &mut text2, &mut r2, DT_LEFT | DT_SINGLELINE | DT_VCENTER);

                SelectObject(hdc, previous);
                y += row_h;
            }

            ReleaseDC(self.hwnd, hdc);
        }
    }
}

impl Drop for PaintCtx {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(HGDIOBJ(self.font_label.0));
            let _ = DeleteObject(HGDIOBJ(self.font_value.0));
            let _ = DeleteObject(HGDIOBJ(self.brush_bg.0));
        }
    }
}