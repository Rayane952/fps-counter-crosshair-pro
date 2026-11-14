//! Composants dessinés à la main, tous utilisables au clavier (Tab, Entrée, Espace),
//! avec survol, anneau de focus et nom accessible.

use eframe::egui::{
    self, Align, Align2, Color32, Layout, Margin, Rect, Response, RichText, Rounding, Sense,
    Stroke, Ui, Vec2, WidgetInfo, WidgetType,
};

use crate::tokens::{self, *};

fn draw_focus(ui: &Ui, response: &Response, rect: Rect, rounding: f32) {
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(2.0),
            Rounding::same(rounding + 2.0),
            focus_ring(),
        );
    }
}

/// Entrée de la barre latérale.
pub fn nav_item(ui: &mut Ui, label: &str, selected: bool) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::click());
    response
        .widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, label));

    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, Rounding::same(RADIUS_SMALL), SURFACE);
        let bar = Rect::from_min_size(
            rect.left_top() + Vec2::new(0.0, 8.0),
            Vec2::new(3.0, rect.height() - 16.0),
        );
        painter.rect_filled(bar, Rounding::same(1.5), ACCENT);
    } else if response.hovered() {
        painter.rect_filled(rect, Rounding::same(RADIUS_SMALL), SURFACE_HOVER);
    }
    let color = if selected || response.hovered() {
        TEXT
    } else {
        TEXT_DIM
    };
    painter.text(
        rect.left_center() + Vec2::new(14.0, 0.0),
        Align2::LEFT_CENTER,
        label,
        tokens::font(14.0),
        color,
    );
    draw_focus(ui, &response, rect, RADIUS_SMALL);
    response
}

/// Entrée prévue mais pas encore disponible : visible, non cliquable, signalée comme telle.
pub fn nav_upcoming(ui: &mut Ui, label: &str) {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::hover());
    let painter = ui.painter();
    painter.text(
        rect.left_center() + Vec2::new(14.0, 0.0),
        Align2::LEFT_CENTER,
        label,
        tokens::font(13.5),
        TEXT_DISABLED,
    );
    painter.text(
        rect.right_center() - Vec2::new(8.0, 0.0),
        Align2::RIGHT_CENTER,
        "Bientôt",
        tokens::font(11.5),
        TEXT_DISABLED,
    );
    response.widget_info(|| {
        WidgetInfo::labeled(
            WidgetType::Label,
            false,
            format!("{label}, bientôt disponible"),
        )
    });
    response.on_hover_text(format!("{label} : pas encore disponible."));
}

/// Case à cocher carrée, contour 3:1 sur la surface, cochée = accent plein.
/// Le libellé fait partie de la zone cliquable.
pub fn checkbox(ui: &mut Ui, on: &mut bool, label: &str) -> Response {
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_owned(), tokens::font(13.5), Color32::PLACEHOLDER);
    let box_size = 18.0;
    let size = Vec2::new(
        box_size + 10.0 + galley.size().x,
        box_size.max(galley.size().y) + 4.0,
    );
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let value = *on;
    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, value, label));

    let painter = ui.painter();
    let square = Rect::from_center_size(
        egui::pos2(rect.left() + box_size / 2.0, rect.center().y),
        Vec2::splat(box_size),
    );
    let rounding = Rounding::same(4.0);
    let hovered = response.hovered();
    if value {
        painter.rect_filled(square, rounding, ACCENT);
        // coche en deux segments
        let p = |x: f32, y: f32| square.min + Vec2::new(x, y) * box_size;
        let stroke = Stroke::new(2.0_f32, Color32::from_rgb(0x0E, 0x18, 0x2A));
        painter.line_segment([p(0.24, 0.52), p(0.43, 0.70)], stroke);
        painter.line_segment([p(0.43, 0.70), p(0.77, 0.32)], stroke);
    } else {
        let border = if hovered {
            Color32::from_rgb(0x9A, 0xA0, 0xAB)
        } else {
            Color32::from_rgb(0x78, 0x7F, 0x8C)
        };
        painter.rect_filled(
            square,
            rounding,
            if hovered { SURFACE_HOVER } else { SIDEBAR },
        );
        painter.rect_stroke(square, rounding, Stroke::new(1.5_f32, border));
    }
    let text_color = if value || hovered { TEXT } else { TEXT_DIM };
    painter.galley(
        egui::pos2(
            square.right() + 10.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        text_color,
    );
    if response.has_focus() {
        painter.rect_stroke(rect.expand(3.0), Rounding::same(RADIUS_SMALL), focus_ring());
    }
    response
}

/// Donne un nom accessible à un widget egui qui n'affiche pas son propre libellé (curseurs).
pub fn named(response: Response, typ: WidgetType, name: &str) -> Response {
    response.widget_info(|| WidgetInfo::labeled(typ, true, name));
    response
}

/// Interrupteur marche/arrêt. `label` sert de nom accessible.
pub fn toggle(ui: &mut Ui, on: &mut bool, label: &str) -> Response {
    let size = Vec2::new(40.0, 22.0);
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let value = *on;
    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, value, label));

    let t = ui.ctx().animate_bool(response.id, value);
    let painter = ui.painter();
    let radius = Rounding::same(rect.height() / 2.0);
    let hovered = response.hovered();
    let track = if value {
        ACCENT
    } else if hovered {
        Color32::from_rgb(0x45, 0x4B, 0x56)
    } else {
        Color32::from_rgb(0x3A, 0x3F, 0x48)
    };
    painter.rect_filled(rect, radius, track);
    if !value {
        // contour 3:1 sur la surface : l'état « arrêt » reste perceptible
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(1.0_f32, Color32::from_rgb(0x78, 0x7F, 0x8C)),
        );
    }
    let cx = egui::lerp((rect.left() + 11.0)..=(rect.right() - 11.0), t);
    let knob = if value {
        Color32::WHITE
    } else {
        Color32::from_rgb(0xCF, 0xD3, 0xDA)
    };
    painter.circle_filled(egui::pos2(cx, rect.center().y), 7.5, knob);
    draw_focus(ui, &response, rect, rect.height() / 2.0);
    response
}

/// Choix exclusif parmi quelques options, en boutons accolés. Renvoie vrai si la valeur change.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    value: &mut T,
    options: &[(T, &str)],
    label: &str,
) -> bool {
    const GAP: f32 = 2.0;
    const INSET: f32 = 3.0;
    let galleys: Vec<_> = options
        .iter()
        // PLACEHOLDER : la couleur réelle (normal, survol, sélection) est donnée au dessin
        .map(|(_, name)| {
            ui.painter().layout_no_wrap(
                (*name).to_owned(),
                tokens::font(13.0),
                Color32::PLACEHOLDER,
            )
        })
        .collect();
    let sizes: Vec<Vec2> = galleys
        .iter()
        .map(|g| g.size() + Vec2::new(24.0, 12.0))
        .collect();
    let total = Vec2::new(
        sizes.iter().map(|s| s.x).sum::<f32>()
            + GAP * (sizes.len().saturating_sub(1)) as f32
            + INSET * 2.0,
        sizes.iter().map(|s| s.y).fold(0.0, f32::max) + INSET * 2.0,
    );

    let mut changed = false;
    // taille fixée d'avance : le contrôle s'aligne à droite de sa ligne, mais ses options
    // restent de gauche à droite (ordre visuel = ordre de tabulation)
    ui.allocate_ui_with_layout(total, Layout::left_to_right(Align::Center), |ui| {
        egui::Frame::none()
            .fill(SIDEBAR)
            .rounding(Rounding::same(RADIUS_SMALL))
            .stroke(Stroke::new(1.0_f32, BORDER))
            .inner_margin(Margin::same(INSET))
            .show(ui, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = GAP;
                    for (i, &(option, name)) in options.iter().enumerate() {
                        let selected = *value == option;
                        let galley = galleys[i].clone();
                        let size = sizes[i];
                        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
                        response.widget_info(|| {
                            WidgetInfo::selected(
                                WidgetType::RadioButton,
                                true,
                                selected,
                                format!("{label} : {name}"),
                            )
                        });
                        let painter = ui.painter();
                        let rounding = Rounding::same(RADIUS_SMALL - 2.0);
                        if selected {
                            painter.rect_filled(rect, rounding, accent_tint());
                            painter.rect_stroke(rect, rounding, Stroke::new(1.0_f32, ACCENT));
                        } else if response.hovered() {
                            painter.rect_filled(rect, rounding, SURFACE_HOVER);
                        }
                        let color = if selected || response.hovered() {
                            TEXT
                        } else {
                            TEXT_DIM
                        };
                        painter.galley(rect.center() - galley.size() / 2.0, galley, color);
                        draw_focus(ui, &response, rect, RADIUS_SMALL - 2.0);
                        if response.clicked() && !selected {
                            *value = option;
                            changed = true;
                        }
                    }
                });
            });
    });
    changed
}

/// Groupe de réglages : un titre puis une surface unique, lignes séparées par un filet.
/// Titre vide : la surface seule (quand le titre et une introduction sont posés avant).
pub fn section<R>(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    if !title.is_empty() {
        ui.label(RichText::new(title).font(font_semibold(15.0)));
        ui.add_space(2.0);
    }
    let inner = egui::Frame::none()
        .fill(SURFACE)
        .rounding(Rounding::same(RADIUS))
        .stroke(Stroke::new(1.0_f32, BORDER))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            add(ui)
        })
        .inner;
    ui.add_space(18.0);
    inner
}

pub fn divider(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().hline(
        rect.x_range().shrink(16.0),
        rect.center().y,
        Stroke::new(1.0_f32, BORDER),
    );
}

/// Ligne de réglage : titre et description à gauche, contrôle à droite (`control_width` réservé).
pub fn row(
    ui: &mut Ui,
    title: &str,
    description: Option<RichText>,
    control_width: f32,
    control: impl FnOnce(&mut Ui),
) {
    egui::Frame::none()
        .inner_margin(Margin::symmetric(16.0, 12.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 3.0;
            ui.horizontal(|ui| {
                let text_width = (ui.available_width() - control_width - 24.0).max(160.0);
                ui.vertical(|ui| {
                    ui.set_width(text_width);
                    ui.label(RichText::new(title).font(tokens::font(14.0)));
                    if let Some(description) = description {
                        ui.add(
                            egui::Label::new(description.font(tokens::font(12.5)).color(TEXT_DIM))
                                .wrap(),
                        );
                    }
                });
                ui.with_layout(Layout::right_to_left(Align::Center), control);
            });
        });
}

/// Comme `row`, avec une pastille d'état devant la description.
pub fn status_row(
    ui: &mut Ui,
    title: &str,
    tone: Tone,
    text: &str,
    control_width: f32,
    control: impl FnOnce(&mut Ui),
) {
    egui::Frame::none()
        .inner_margin(Margin::symmetric(16.0, 12.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 3.0;
            ui.horizontal(|ui| {
                let text_width = (ui.available_width() - control_width - 24.0).max(160.0);
                ui.vertical(|ui| {
                    ui.set_width(text_width);
                    ui.label(RichText::new(title).font(tokens::font(14.0)));
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        status_dot(ui, tone);
                        ui.add(
                            egui::Label::new(
                                RichText::new(text).font(tokens::font(12.5)).color(TEXT_DIM),
                            )
                            .wrap(),
                        );
                    });
                });
                ui.with_layout(Layout::right_to_left(Align::Center), control);
            });
        });
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Ok,
    Warn,
    Error,
    Neutral,
}

impl Tone {
    pub fn color(self) -> Color32 {
        match self {
            Tone::Ok => OK,
            Tone::Warn => WARN,
            Tone::Error => ERROR,
            Tone::Neutral => TEXT_DISABLED,
        }
    }
}

/// Pastille d'état : uniquement pour un état réel (actif, arrêté, indisponible).
pub fn status_dot(ui: &mut Ui, tone: Tone) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, tone.color());
}

/// Bandeau de problème, avec une action corrective facultative. Renvoie vrai si l'action est cliquée.
pub fn notice(ui: &mut Ui, tone: Tone, text: &str, action: Option<&str>) -> bool {
    let mut clicked = false;
    egui::Frame::none()
        .fill(SURFACE)
        .rounding(Rounding::same(RADIUS))
        .stroke(Stroke::new(1.0_f32, tone.color()))
        .inner_margin(Margin::symmetric(14.0, 10.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                status_dot(ui, tone);
                let reserve = if action.is_some() { 180.0 } else { 0.0 };
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() - reserve).max(120.0));
                    ui.add(egui::Label::new(RichText::new(text).font(tokens::font(13.5))).wrap());
                });
                if let Some(action) = action {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        clicked = ui.button(action).clicked();
                    });
                }
            });
        });
    ui.add_space(10.0);
    clicked
}
