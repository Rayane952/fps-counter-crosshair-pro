//! Aperçu du HUD : mêmes lignes (`ovl_core::hud::rows`), même géométrie et mêmes couleurs que le
//! rendu GDI de l'overlay (`ovl_overlay::painter`), à taille réelle.

use std::sync::Arc;

use eframe::egui::{
    self, Align2, Color32, Galley, Painter, Pos2, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2,
};
use ovl_core::hud::{Cell, Layout, Palette, Row, ThemeId};
use ovl_platform::Config;

use crate::tokens::{self, *};

/// Fond neutre moyen : ni clair ni sombre, pour juger tous les thèmes équitablement.
const BACKDROP: Color32 = Color32::from_rgb(0x3B, 0x40, 0x48);

fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

fn dim(c: [u8; 3]) -> Color32 {
    rgb([c[0] / 3, c[1] / 3, c[2] / 3])
}

enum Item {
    Text {
        pos: Pos2,
        galley: Arc<Galley>,
        color: Color32,
    },
    Block {
        rect: Rect,
        color: Color32,
    },
}

struct Plan {
    size: Vec2,
    items: Vec<Item>,
}

/// Reprend `Renderer::plan` de l'overlay (pixels logiques × `k`).
fn plan(
    painter: &Painter,
    rows: &[Row],
    layout: Layout,
    palette: Palette,
    graph: &[f32],
    k: f32,
) -> Plan {
    let pad_x = 10.0 * k;
    let pad_y = 6.0 * k;
    let row_h = 21.0 * k;
    let gap = 10.0 * k;
    let small_gap = 6.0 * k;
    let bar_w = (3.0 * k).round().max(2.0);
    let bar_gap = (1.0 * k).round().max(1.0);
    let label_font = tokens::font_semibold(12.0 * k);
    let value_font = tokens::font_bold(15.0 * k);
    let detail_font = tokens::font_semibold(12.0 * k);
    let text = |s: &str, font: &egui::FontId| {
        painter.layout_no_wrap(s.to_owned(), font.clone(), Color32::PLACEHOLDER)
    };
    // egui place un texte par son coin haut-gauche : on centre chaque police dans la ligne,
    // puis on la descend de 15 % de sa hauteur pour approcher l'alignement sur la ligne de base
    let at = |x: f32, top: f32, g: &Arc<Galley>| {
        Pos2::new(
            x,
            top + (row_h - g.size().y) / 2.0 + (row_h - g.size().y) * 0.15,
        )
    };

    let mut items = Vec::new();
    let push_cell = |items: &mut Vec<Item>, cell: &Cell, x: f32, top: f32, detail_x: f32| match cell
    {
        Cell::Text { value, detail } => {
            let g = text(value, &value_font);
            items.push(Item::Text {
                pos: at(x, top, &g),
                galley: g,
                color: rgb(palette.value),
            });
            if let Some(d) = detail {
                let g = text(d, &detail_font);
                items.push(Item::Text {
                    pos: at(detail_x, top, &g),
                    galley: g,
                    color: rgb(palette.detail),
                });
            }
        }
        Cell::Bars(bars) => {
            let h = (row_h * 0.62).round();
            let bottom = top + (row_h + h) / 2.0;
            for (i, load) in bars.iter().enumerate() {
                let left = x + i as f32 * (bar_w + bar_gap);
                let base = Rect::from_min_max(
                    Pos2::new(left, bottom - h),
                    Pos2::new(left + bar_w, bottom),
                );
                items.push(Item::Block {
                    rect: base,
                    color: dim(palette.label),
                });
                let fill = (h * load.clamp(0.0, 1.0)).round().max(1.0);
                let level = Rect::from_min_max(
                    Pos2::new(left, bottom - fill),
                    Pos2::new(left + bar_w, bottom),
                );
                items.push(Item::Block {
                    rect: level,
                    color: rgb(palette.value),
                });
            }
        }
    };
    let cell_width = |cell: &Cell| -> (f32, f32) {
        match cell {
            Cell::Text { value, detail } => (
                text(value, &value_font).size().x,
                detail
                    .as_ref()
                    .map_or(0.0, |d| text(d, &detail_font).size().x),
            ),
            Cell::Bars(bars) => (bars.len() as f32 * (bar_w + bar_gap), 0.0),
        }
    };

    let mut width: f32;
    let mut y: f32;
    match layout {
        Layout::Panel | Layout::Graph => {
            let label_col = rows
                .iter()
                .map(|r| text(r.label, &label_font).size().x)
                .fold(0.0, f32::max);
            let widths: Vec<(f32, f32)> = rows.iter().map(|r| cell_width(&r.cell)).collect();
            let value_col = widths
                .iter()
                .filter(|(_, d)| *d > 0.0)
                .map(|(v, _)| *v)
                .fold(0.0, f32::max);
            let value_x = pad_x + label_col + gap;
            let detail_x = value_x + value_col + small_gap;
            width = 0.0;
            y = pad_y;
            for (row, (v, d)) in rows.iter().zip(&widths) {
                let g = text(row.label, &label_font);
                items.push(Item::Text {
                    pos: at(pad_x, y, &g),
                    galley: g,
                    color: rgb(palette.label),
                });
                push_cell(&mut items, &row.cell, value_x, y, detail_x);
                let right = if *d > 0.0 { detail_x + d } else { value_x + v };
                width = width.max(right + pad_x);
                y += row_h;
            }
        }
        Layout::Line => {
            let mut x = pad_x;
            y = pad_y;
            for (i, row) in rows.iter().enumerate() {
                if i > 0 {
                    x += 14.0 * k;
                }
                let g = text(row.label, &label_font);
                let value_x = x + g.size().x + 5.0 * k;
                items.push(Item::Text {
                    pos: at(x, y, &g),
                    galley: g,
                    color: rgb(palette.label),
                });
                let (v, d) = cell_width(&row.cell);
                let detail_x = value_x + v + small_gap;
                push_cell(&mut items, &row.cell, value_x, y, detail_x);
                x = if d > 0.0 { detail_x + d } else { value_x + v };
            }
            width = x + pad_x;
            if !rows.is_empty() {
                y += row_h;
            }
        }
    }

    if layout == Layout::Graph && graph.len() >= 2 {
        let graph_h = 38.0 * k;
        let graph_w = (width - pad_x * 2.0).max(160.0 * k).round();
        width = width.max(graph_w + pad_x * 2.0);
        let top = y + 4.0 * k;
        let bottom = top + graph_h;
        let samples = &graph[graph.len().saturating_sub(graph_w as usize)..];
        let mut sorted = samples.to_vec();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let median = sorted[sorted.len() / 2].max(0.1);
        let ceiling = sorted[sorted.len() - 1].max(median * 2.0).max(1.0);
        let axis = Rect::from_min_max(
            Pos2::new(pad_x, bottom),
            Pos2::new(pad_x + graph_w, bottom + 1.0),
        );
        items.push(Item::Block {
            rect: axis,
            color: dim(palette.label),
        });
        let x0 = pad_x + graph_w - samples.len() as f32;
        for (i, ms) in samples.iter().enumerate() {
            let h = ((ms / ceiling) * graph_h).round().clamp(1.0, graph_h);
            let color = if *ms > median * 2.0 {
                rgb(palette.alert)
            } else {
                rgb(palette.value)
            };
            let x = x0 + i as f32;
            items.push(Item::Block {
                rect: Rect::from_min_max(Pos2::new(x, bottom - h), Pos2::new(x + 1.0, bottom)),
                color,
            });
        }
        let g = text(&format!("{ceiling:.0} ms"), &detail_font);
        items.push(Item::Text {
            pos: Pos2::new(pad_x + graph_w - g.size().x, top),
            galley: g,
            color: rgb(palette.detail),
        });
        y = bottom + 4.0 * k;
    }
    Plan {
        size: Vec2::new(width.max(1.0), y + pad_y),
        items,
    }
}

fn paint(painter: &Painter, origin: Pos2, plan: Plan, palette: Palette, opacity: f32) {
    if !palette.clear {
        let a = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
        let [r, g, b] = palette.background;
        painter.rect_filled(
            Rect::from_min_size(origin, plan.size),
            Rounding::ZERO,
            Color32::from_rgba_unmultiplied(r, g, b, a),
        );
    }
    for item in plan.items {
        match item {
            Item::Text { pos, galley, color } => {
                if palette.clear {
                    // même ombre que l'overlay : décalée d'un pixel, noire à 85 %
                    painter.galley(
                        origin + pos.to_vec2() + Vec2::splat(1.0),
                        galley.clone(),
                        Color32::from_black_alpha(217),
                    );
                }
                painter.galley(origin + pos.to_vec2(), galley, color);
            }
            Item::Block { rect, color } => {
                painter.rect_filled(rect.translate(origin.to_vec2()), Rounding::ZERO, color);
            }
        }
    }
}

/// Aperçu dans un cadre représentant l'écran ; `rows` à `None` : première mesure en cours.
pub fn show(ui: &mut Ui, config: &Config, rows: Option<&[Row]>, graph: &[f32]) {
    let palette = config.theme.palette_with(config.text_rgb());
    let plan = rows.map(|rows| {
        plan(
            ui.painter(),
            rows,
            config.layout,
            palette,
            graph,
            config.scale,
        )
    });
    let (mx, my) = (config.margin_x as f32, config.margin_y as f32);
    let hud = plan.as_ref().map_or(Vec2::ZERO, |p| p.size);
    let height = (hud.y + my * 2.0 + 24.0).max(170.0);
    let (frame, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let painter = ui.painter_at(frame);
    painter.rect_filled(frame, Rounding::same(RADIUS), BACKDROP);
    painter.rect_stroke(frame, Rounding::same(RADIUS), Stroke::new(1.0_f32, BORDER));

    let note = |text: &str| {
        painter.text(
            frame.center(),
            Align2::CENTER_CENTER,
            text,
            tokens::font(13.5),
            Color32::from_rgb(0xE4, 0xE7, 0xEC),
        );
    };
    let Some(plan) = plan else {
        return note("Mesure en cours…");
    };
    if plan.items.is_empty() {
        return note("Aucune mesure cochée n'a de valeur : le HUD resterait vide.");
    }
    let x = if config.corner.is_left() {
        frame.left() + mx
    } else {
        frame.right() - mx - hud.x
    };
    let y = if config.corner.is_top() {
        frame.top() + my
    } else {
        frame.bottom() - my - hud.y
    };
    paint(&painter, Pos2::new(x, y), plan, palette, config.opacity);
}

/// Vignette de thème : le HUD réel en petit, avec ses couleurs. Cliquable et focusable.
pub fn theme_swatch(
    ui: &mut Ui,
    theme: ThemeId,
    selected: bool,
    rows: &[Row],
    config: &Config,
) -> Response {
    let size = Vec2::new(132.0, 96.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::RadioButton,
            true,
            selected,
            format!("Thème : {}", theme.name()),
        )
    });
    let painter = ui.painter_at(rect.expand(3.0));
    let preview = Rect::from_min_size(rect.min, Vec2::new(size.x, 70.0));
    painter.rect_filled(preview, Rounding::same(RADIUS_SMALL), BACKDROP);

    let palette = theme.palette_with(config.text_rgb());
    let sample: Vec<Row> = rows
        .iter()
        .filter(|r| matches!(r.cell, Cell::Text { .. }))
        .take(3)
        .cloned()
        .collect();
    if !sample.is_empty() {
        let plan = plan(&painter, &sample, Layout::Panel, palette, &[], 0.78);
        let origin = Pos2::new(
            preview.center().x - plan.size.x / 2.0,
            preview.center().y - plan.size.y / 2.0,
        );
        let clipped = painter.with_clip_rect(preview.shrink(2.0));
        paint(&clipped, origin, plan, palette, config.opacity);
    }

    let hovered = response.hovered();
    let stroke = if selected {
        Stroke::new(2.0_f32, ACCENT)
    } else if hovered {
        Stroke::new(1.0_f32, Color32::from_rgb(0x5A, 0x61, 0x6D))
    } else {
        Stroke::new(1.0_f32, BORDER)
    };
    painter.rect_stroke(preview, Rounding::same(RADIUS_SMALL), stroke);
    let color = if selected || hovered { TEXT } else { TEXT_DIM };
    painter.text(
        Pos2::new(rect.center().x, rect.bottom() - 11.0),
        Align2::CENTER_CENTER,
        theme.name(),
        tokens::font(12.5),
        color,
    );
    if response.has_focus() {
        painter.rect_stroke(
            rect.expand(2.0),
            Rounding::same(RADIUS_SMALL + 2.0),
            focus_ring(),
        );
    }
    response
}
