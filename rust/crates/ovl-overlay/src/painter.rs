//! Rendu du panneau en image ARGB prémultipliée, pour `UpdateLayeredWindow`.
//!
//! GDI n'écrit pas de canal alpha. Chaque élément est donc dessiné deux fois :
//! en blanc sur noir (masque = couverture, antialiasing en niveaux de gris compris) et dans sa
//! couleur sur noir (= couleur × couverture, donc déjà prémultipliée). Le fond, l'ombre éventuelle
//! et le texte sont ensuite composés pixel par pixel. Résultat : fond à l'opacité choisie,
//! texte toujours opaque.
//!
//! Toutes les mesures dérivent d'un seul facteur `k` (taille × échelle Windows du moniteur).

use ovl_core::hud::{Cell, Layout, Palette, Row};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontW, CreateSolidBrush, DeleteDC, DeleteObject,
    FillRect, GetTextExtentPoint32W, SelectObject, SetBkMode, SetTextAlign, SetTextColor, TextOutW,
    ANTIALIASED_QUALITY, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CLIP_DEFAULT_PRECIS,
    DEFAULT_CHARSET, DIB_RGB_COLORS, FF_DONTCARE, HBITMAP, HDC, HFONT, HGDIOBJ, OUT_DEFAULT_PRECIS,
    TA_BASELINE, TA_LEFT, TRANSPARENT,
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn colorref(rgb: [u8; 3]) -> COLORREF {
    COLORREF(u32::from(rgb[0]) | u32::from(rgb[1]) << 8 | u32::from(rgb[2]) << 16)
}

const WHITE: [u8; 3] = [255, 255, 255];

/// Ce qu'il faut dessiner à un instant donné.
pub struct Scene<'a> {
    pub rows: &'a [Row],
    pub layout: Layout,
    pub palette: Palette,
    /// Opacité du fond (ignorée par le thème transparent).
    pub opacity: f32,
    /// Temps d'image (ms) pour la disposition Graphique, du plus ancien au plus récent.
    pub graph: &'a [f32],
    /// Ligne d'alerte (style « dans le panneau »).
    pub alert: Option<&'a str>,
    /// Phase allumée du clignotement.
    pub flash: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Font {
    Label,
    Value,
    Detail,
}

struct Fonts {
    label: HFONT,
    value: HFONT,
    detail: HFONT,
}

impl Fonts {
    fn get(&self, f: Font) -> HFONT {
        match f {
            Font::Label => self.label,
            Font::Value => self.value,
            Font::Detail => self.detail,
        }
    }
}

/// Une chaîne placée (x, ligne de base).
struct Text {
    x: i32,
    baseline: i32,
    font: Font,
    color: [u8; 3],
    text: String,
}

/// Un rectangle plein.
struct Block {
    rect: RECT,
    color: [u8; 3],
}

struct Plan {
    width: i32,
    height: i32,
    texts: Vec<Text>,
    blocks: Vec<Block>,
}

/// Image prête pour `UpdateLayeredWindow` (DC mémoire avec la DIB sélectionnée).
pub struct Image {
    pub dc: HDC,
    pub width: i32,
    pub height: i32,
}

struct Surface {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    bits: *mut u8,
}

impl Surface {
    unsafe fn new(width: i32, height: i32) -> Option<Self> {
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // de haut en bas
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let dc = CreateCompatibleDC(None);
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
        let previous = SelectObject(dc, HGDIOBJ(bitmap.0));
        SetBkMode(dc, TRANSPARENT);
        SetTextAlign(dc, TA_LEFT | TA_BASELINE);
        Some(Self {
            dc,
            bitmap,
            previous,
            bits: bits as *mut u8,
        })
    }

    unsafe fn pixels(&mut self, len: usize) -> &mut [u8] {
        std::slice::from_raw_parts_mut(self.bits, len)
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.previous);
            let _ = DeleteObject(HGDIOBJ(self.bitmap.0));
            let _ = DeleteDC(self.dc);
        }
    }
}

pub struct Renderer {
    fonts: Fonts,
    k: f32,
    measure_dc: HDC,
    /// Image couleur (devient l'image finale) et masque, recréés si la taille change.
    surfaces: Option<(i32, i32, Surface, Surface)>,
}

impl Renderer {
    pub fn new(k: f32) -> Option<Self> {
        unsafe {
            let family: Vec<u16> = "Segoe UI Variable Display\0".encode_utf16().collect();
            let make = |px: f32, weight: i32| -> HFONT {
                CreateFontW(
                    -(px * k).round() as i32,
                    0,
                    0,
                    0,
                    weight,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET.0 as u32,
                    OUT_DEFAULT_PRECIS.0 as u32,
                    CLIP_DEFAULT_PRECIS.0 as u32,
                    // niveaux de gris : ClearType colorerait le masque
                    ANTIALIASED_QUALITY.0 as u32,
                    FF_DONTCARE.0 as u32,
                    PCWSTR(family.as_ptr()),
                )
            };
            let fonts = Fonts {
                label: make(12.0, 600),
                value: make(15.0, 700),
                detail: make(12.0, 600),
            };
            if fonts.label.is_invalid() || fonts.value.is_invalid() || fonts.detail.is_invalid() {
                return None;
            }
            let measure_dc = CreateCompatibleDC(None);
            Some(Self {
                fonts,
                k,
                measure_dc,
                surfaces: None,
            })
        }
    }

    pub fn k(&self) -> f32 {
        self.k
    }

    fn px(&self, logical: f32) -> i32 {
        (logical * self.k).round() as i32
    }

    fn width(&self, font: Font, text: &str) -> i32 {
        unsafe {
            let previous = SelectObject(self.measure_dc, HGDIOBJ(self.fonts.get(font).0));
            let mut size = SIZE::default();
            let _ = GetTextExtentPoint32W(self.measure_dc, &wide(text), &mut size);
            SelectObject(self.measure_dc, previous);
            size.cx
        }
    }

    fn plan(&self, scene: &Scene) -> Plan {
        let p = scene.palette;
        let pad_x = self.px(10.0);
        let pad_y = self.px(6.0);
        let row_h = self.px(21.0);
        let gap = self.px(10.0);
        let small_gap = self.px(6.0);
        let baseline = |top: i32| top + (row_h as f32 * 0.74) as i32;
        let bar_w = self.px(3.0).max(2);
        let bar_gap = self.px(1.0).max(1);

        let mut texts = Vec::new();
        let mut blocks = Vec::new();

        // largeur de la partie valeur d'une ligne
        let cell_width = |cell: &Cell| -> (i32, i32) {
            match cell {
                Cell::Text { value, detail } => {
                    let v = self.width(Font::Value, value);
                    let d = detail.as_ref().map_or(0, |d| self.width(Font::Detail, d));
                    (v, d)
                }
                Cell::Bars(bars) => (bars.len() as i32 * (bar_w + bar_gap), 0),
            }
        };

        let place_cell = |texts: &mut Vec<Text>,
                          blocks: &mut Vec<Block>,
                          cell: &Cell,
                          x: i32,
                          top: i32,
                          detail_x: i32| {
            match cell {
                Cell::Text { value, detail } => {
                    texts.push(Text {
                        x,
                        baseline: baseline(top),
                        font: Font::Value,
                        color: p.value,
                        text: value.clone(),
                    });
                    if let Some(d) = detail {
                        texts.push(Text {
                            x: detail_x,
                            baseline: baseline(top),
                            font: Font::Detail,
                            color: p.detail,
                            text: d.clone(),
                        });
                    }
                }
                Cell::Bars(bars) => {
                    let h = (row_h as f32 * 0.62) as i32;
                    let bottom = top + (row_h + h) / 2;
                    for (i, load) in bars.iter().enumerate() {
                        let left = x + i as i32 * (bar_w + bar_gap);
                        // fond de la barre, puis remplissage proportionnel
                        blocks.push(Block {
                            rect: RECT {
                                left,
                                top: bottom - h,
                                right: left + bar_w,
                                bottom,
                            },
                            color: dim(p.label),
                        });
                        let fill = ((h as f32) * load.clamp(0.0, 1.0)).round().max(1.0) as i32;
                        blocks.push(Block {
                            rect: RECT {
                                left,
                                top: bottom - fill,
                                right: left + bar_w,
                                bottom,
                            },
                            color: p.value,
                        });
                    }
                }
            }
        };

        let mut width;
        let mut y;
        match scene.layout {
            Layout::Panel | Layout::Graph => {
                let label_col = scene
                    .rows
                    .iter()
                    .map(|r| self.width(Font::Label, r.label))
                    .max()
                    .unwrap_or(0);
                let widths: Vec<(i32, i32)> =
                    scene.rows.iter().map(|r| cell_width(&r.cell)).collect();
                let value_col = widths
                    .iter()
                    .filter(|(_, d)| *d > 0)
                    .map(|(v, _)| *v)
                    .max()
                    .unwrap_or(0);
                let value_x = pad_x + label_col + gap;
                let detail_x = value_x + value_col + small_gap;
                width = 0;
                y = pad_y;
                for (row, (v, d)) in scene.rows.iter().zip(&widths) {
                    texts.push(Text {
                        x: pad_x,
                        baseline: baseline(y),
                        font: Font::Label,
                        color: p.label,
                        text: row.label.to_owned(),
                    });
                    place_cell(&mut texts, &mut blocks, &row.cell, value_x, y, detail_x);
                    let right = if *d > 0 { detail_x + d } else { value_x + v };
                    width = width.max(right + pad_x);
                    y += row_h;
                }
            }
            Layout::Line => {
                let mut x = pad_x;
                y = pad_y;
                for (i, row) in scene.rows.iter().enumerate() {
                    if i > 0 {
                        x += self.px(14.0);
                    }
                    texts.push(Text {
                        x,
                        baseline: baseline(y),
                        font: Font::Label,
                        color: p.label,
                        text: row.label.to_owned(),
                    });
                    let value_x = x + self.width(Font::Label, row.label) + self.px(5.0);
                    let (v, d) = cell_width(&row.cell);
                    let detail_x = value_x + v + small_gap;
                    place_cell(&mut texts, &mut blocks, &row.cell, value_x, y, detail_x);
                    x = if d > 0 { detail_x + d } else { value_x + v };
                }
                width = x + pad_x;
                if !scene.rows.is_empty() {
                    y += row_h;
                }
            }
        }

        if let Some(alert) = scene.alert {
            let w = self.width(Font::Value, alert);
            texts.push(Text {
                x: pad_x,
                baseline: baseline(y),
                font: Font::Value,
                color: p.alert,
                text: alert.to_owned(),
            });
            width = width.max(pad_x * 2 + w);
            y += row_h;
        }

        if scene.layout == Layout::Graph && scene.graph.len() >= 2 {
            let graph_h = self.px(38.0);
            let graph_w = (width - pad_x * 2).max(self.px(160.0));
            width = width.max(graph_w + pad_x * 2);
            let top = y + self.px(4.0);
            let bottom = top + graph_h;
            let samples = &scene.graph[scene.graph.len().saturating_sub(graph_w as usize)..];
            let mut sorted = samples.to_vec();
            sorted.sort_by(|a, b| a.total_cmp(b));
            let median = sorted[sorted.len() / 2].max(0.1);
            // échelle : au moins 2× la médiane, pour que les saccades ressortent
            let ceiling = (sorted[sorted.len() - 1]).max(median * 2.0).max(1.0);
            blocks.push(Block {
                rect: RECT {
                    left: pad_x,
                    top: bottom,
                    right: pad_x + graph_w,
                    bottom: bottom + 1,
                },
                color: dim(p.label),
            });
            let x0 = pad_x + graph_w - samples.len() as i32;
            for (i, ms) in samples.iter().enumerate() {
                let h = ((ms / ceiling) * graph_h as f32)
                    .round()
                    .clamp(1.0, graph_h as f32) as i32;
                let color = if *ms > median * 2.0 { p.alert } else { p.value };
                let x = x0 + i as i32;
                blocks.push(Block {
                    rect: RECT {
                        left: x,
                        top: bottom - h,
                        right: x + 1,
                        bottom,
                    },
                    color,
                });
            }
            let label = format!("{ceiling:.0} ms");
            texts.push(Text {
                x: pad_x + graph_w - self.width(Font::Detail, &label),
                baseline: top + self.px(11.0),
                font: Font::Detail,
                color: p.detail,
                text: label,
            });
            y = bottom + self.px(4.0);
        }

        Plan {
            width: width.max(1),
            height: (y + pad_y).max(1),
            texts,
            blocks,
        }
    }

    pub fn render(&mut self, scene: &Scene) -> Option<Image> {
        let plan = self.plan(scene);
        let (w, h) = (plan.width, plan.height);
        unsafe {
            if !matches!(&self.surfaces, Some((sw, sh, _, _)) if *sw == w && *sh == h) {
                self.surfaces = Some((w, h, Surface::new(w, h)?, Surface::new(w, h)?));
            }
            let (_, _, color, mask) = self.surfaces.as_mut()?;
            let len = (w * h * 4) as usize;
            color.pixels(len).fill(0);
            mask.pixels(len).fill(0);

            for block in &plan.blocks {
                for (surface, rgb) in [(&*color, block.color), (&*mask, WHITE)] {
                    let brush = CreateSolidBrush(colorref(rgb));
                    FillRect(surface.dc, &block.rect, brush);
                    let _ = DeleteObject(HGDIOBJ(brush.0));
                }
            }
            for text in &plan.texts {
                let w16 = wide(&text.text);
                for (surface, rgb) in [(&*color, text.color), (&*mask, WHITE)] {
                    let previous = SelectObject(surface.dc, HGDIOBJ(self.fonts.get(text.font).0));
                    SetTextColor(surface.dc, colorref(rgb));
                    let _ = TextOutW(surface.dc, text.x, text.baseline, &w16);
                    SelectObject(surface.dc, previous);
                }
            }

            let p = scene.palette;
            let (bg, bg_alpha) = if p.clear {
                ([0, 0, 0], 0.0)
            } else if scene.flash {
                (mix(p.background, p.alert, 0.35), scene.opacity.max(0.6))
            } else {
                (p.background, scene.opacity)
            };
            let mask_px: Vec<u8> = mask.pixels(len).to_vec();
            compose(
                color.pixels(len),
                &mask_px,
                w as usize,
                h as usize,
                bg,
                bg_alpha,
                p.clear,
            );
            Some(Image {
                dc: color.dc,
                width: w,
                height: h,
            })
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.surfaces = None;
            let _ = DeleteDC(self.measure_dc);
            for f in [self.fonts.label, self.fonts.value, self.fonts.detail] {
                let _ = DeleteObject(HGDIOBJ(f.0));
            }
        }
    }
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    [m(a[0], b[0]), m(a[1], b[1]), m(a[2], b[2])]
}

/// Couleur atténuée pour les fonds de barres et les axes.
fn dim(c: [u8; 3]) -> [u8; 3] {
    [c[0] / 3, c[1] / 3, c[2] / 3]
}

/// Compose fond, ombre (thème transparent) et texte en BGRA prémultiplié, en place dans `color`.
fn compose(
    color: &mut [u8],
    mask: &[u8],
    w: usize,
    h: usize,
    bg: [u8; 3],
    bg_alpha: f32,
    shadow: bool,
) {
    let bg_pm = [
        bg[2] as f32 * bg_alpha,
        bg[1] as f32 * bg_alpha,
        bg[0] as f32 * bg_alpha,
    ]; // B, G, R
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            let cover = mask[i + 2] as f32 / 255.0;
            let (mut b, mut g, mut r, mut a) = (bg_pm[0], bg_pm[1], bg_pm[2], bg_alpha * 255.0);
            if shadow && x > 0 && y > 0 {
                let j = ((y - 1) * w + (x - 1)) * 4;
                let s = mask[j + 2] as f32 / 255.0 * 0.85;
                b *= 1.0 - s;
                g *= 1.0 - s;
                r *= 1.0 - s;
                a = s * 255.0 + a * (1.0 - s);
            }
            b = color[i] as f32 + b * (1.0 - cover);
            g = color[i + 1] as f32 + g * (1.0 - cover);
            r = color[i + 2] as f32 + r * (1.0 - cover);
            a = cover * 255.0 + a * (1.0 - cover);
            color[i] = b.round().min(a) as u8;
            color[i + 1] = g.round().min(a) as u8;
            color[i + 2] = r.round().min(a) as u8;
            color[i + 3] = a.round() as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opaque_text_over_translucent_background() {
        // pixel 0 : fond seul ; pixel 1 : texte blanc plein
        let mut color = vec![0, 0, 0, 0, 255, 255, 255, 0];
        let mask = vec![0, 0, 0, 0, 255, 255, 255, 0];
        compose(&mut color, &mask, 2, 1, [100, 0, 0], 0.5, false);
        assert_eq!(
            &color[0..4],
            &[0, 0, 50, 128],
            "fond rouge à 50 %, prémultiplié"
        );
        assert_eq!(&color[4..8], &[255, 255, 255, 255], "le texte reste opaque");
    }
}
