//! Home tab: recommended tools, open card, pinned folders and recent files (local only, never
//! another app's list).

use egui::{Align2, CornerRadius, Rect, Sense, Stroke, vec2};
use pdfcraft_engine::catalog;

use crate::theme::{self, Tokens};
use crate::{PdfCraftApp, icons, panels::human_size, widgets};

const RECOMMENDED: [&str; 5] = ["organize", "comment", "form", "edit", "protect"];

/// Axion: the welcome banner, a soft blue gradient card with the mark, the app's promise and the
/// actions people come for (open, convert to Word or Markdown). It wraps on narrow screens.
fn hero(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let width = ui.available_width();
    let narrow = width < 560.0;
    let (top, bottom) = if t.dark() {
        (egui::Color32::from_rgb(0x1F, 0x34, 0x5E), egui::Color32::from_rgb(0x16, 0x22, 0x3C))
    } else {
        (egui::Color32::from_rgb(0xDC, 0xE9, 0xFD), egui::Color32::from_rgb(0xF4, 0xF8, 0xFF))
    };
    // The background goes under the content once the content's height is known.
    let background = ui.painter().add(egui::Shape::Noop);
    let pad = if narrow { vec2(18.0, 18.0) } else { vec2(24.0, 22.0) };
    let frame = egui::Frame::NONE.inner_margin(egui::Margin::symmetric(pad.x as i8, pad.y as i8)).show(ui, |ui| {
        ui.set_width(width - 2.0 * pad.x);
        ui.horizontal(|ui| {
            widgets::artcraft_mark(ui, if narrow { 38.0 } else { 44.0 });
            ui.add_space(6.0);
            let room = ui.available_width();
            ui.vertical(|ui| {
                ui.set_max_width(room);
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(tl!("Welcome to Axion PDF")).font(theme::semibold(if narrow { 21.0 } else { 24.0 })).color(t.text),
                    )
                    .wrap(),
                );
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(tl!("Your PDF workbench: read, sign, organize and convert, privately on your own device."))
                            .color(t.text_muted)
                            .font(theme::regular(14.0)),
                    )
                    .wrap(),
                );
            });
        });
        ui.add_space(if narrow { 14.0 } else { 18.0 });
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
            if hero_button(ui, t, "folder-open", tl!("Open PDF"), true, true).clicked() {
                app.open_dialog();
            }
            for (icon, label, command) in
                [("file-text", tl!("Save as Word"), "export.docx"), ("file-text", tl!("Save as Markdown"), "export.markdown")]
            {
                let enabled = app.active.is_some();
                // Not `add_enabled_ui`: a child Ui would keep the buttons from wrapping on a phone.
                let resp = hero_button(ui, t, icon, label, false, enabled);
                let resp = if enabled { resp } else { resp.on_hover_text(tl!("Open a PDF first")) };
                if enabled && resp.clicked() {
                    app.execute(command);
                }
            }
            if hero_button(ui, t, "layout-grid", tl!("All tools"), false, true).clicked() {
                app.select_tool("export");
            }
        });
    });
    let rect = frame.response.rect;
    let mut shapes = Vec::new();
    let shadow = egui::Shadow { offset: [0, 10], blur: 30, spread: 0, color: t.page_shadow.gamma_multiply(0.7) };
    shapes.push(shadow.as_shape(rect, CornerRadius::same(20)).into());
    shapes.push(egui::Shape::mesh(gradient(rect, 20.0, top, bottom)));
    shapes.push(egui::Shape::rect_stroke(rect, CornerRadius::same(20), Stroke::new(1.0, t.border), egui::StrokeKind::Inside));
    ui.painter().set(background, egui::Shape::Vec(shapes));
}

/// A pill-shaped action on the welcome banner: filled in the accent for the main one, white glass
/// for the others.
fn hero_button(ui: &mut egui::Ui, t: &Tokens, icon: &str, label: &str, primary: bool, enabled: bool) -> egui::Response {
    let font = theme::semibold(13.0);
    let w = ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), font.clone(), t.text).size().x);
    let (rect, resp) = ui.allocate_exact_size(vec2(w + 52.0, 38.0), if enabled { Sense::click() } else { Sense::hover() });
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    let press = theme::Press::track(ui, &resp);
    let (fill, text) = if primary {
        (
            press.fill(t.accent, t.accent.lerp_to_gamma(egui::Color32::BLACK, 0.15), t.accent.lerp_to_gamma(egui::Color32::BLACK, 0.25)),
            egui::Color32::WHITE,
        )
    } else if t.dark() {
        (press.fill(egui::Color32::from_white_alpha(18), egui::Color32::from_white_alpha(30), egui::Color32::from_white_alpha(40)), t.text)
    } else {
        (press.fill(egui::Color32::from_white_alpha(235), egui::Color32::WHITE, t.pressed), t.text)
    };
    let text = if enabled { text } else { t.text_faint.gamma_multiply(0.6) };
    let body = rect.translate(press.offset());
    if primary || enabled {
        let lift = egui::Shadow { offset: [0, 3], blur: 10, spread: 0, color: t.page_shadow.gamma_multiply(0.6) };
        ui.painter().add(lift.as_shape(body, CornerRadius::same(19)));
    }
    ui.painter().rect(body, CornerRadius::same(19), fill, Stroke::new(1.0, if primary { t.accent } else { t.border }), egui::StrokeKind::Inside);
    icons::paint(ui, Rect::from_min_size(body.min + vec2(14.0, 10.0), vec2(18.0, 18.0)), icon, 17.0, text);
    ui.painter().text(body.min + vec2(38.0, 19.0), Align2::LEFT_CENTER, label, font, text);
    if enabled { theme::hand(resp) } else { resp }
}

/// A rounded rectangle filled with a vertical gradient (top to bottom), drawn as a mesh fan.
fn gradient(rect: Rect, radius: f32, top: egui::Color32, bottom: egui::Color32) -> egui::Mesh {
    let r = radius.min(rect.width() * 0.5).min(rect.height() * 0.5).max(0.0);
    let mut points = Vec::with_capacity(4 * 9);
    // Corners clockwise from the top-right, eight segments each.
    let corners = [
        (egui::pos2(rect.right() - r, rect.top() + r), -90.0f32),
        (egui::pos2(rect.right() - r, rect.bottom() - r), 0.0),
        (egui::pos2(rect.left() + r, rect.bottom() - r), 90.0),
        (egui::pos2(rect.left() + r, rect.top() + r), 180.0),
    ];
    for (c, start) in corners {
        for k in 0..=8 {
            let a = (start + 90.0 * k as f32 / 8.0).to_radians();
            points.push(c + vec2(a.cos(), a.sin()) * r);
        }
    }
    let color_at = |y: f32| {
        let k = if rect.height() > 0.0 { ((y - rect.top()) / rect.height()).clamp(0.0, 1.0) } else { 0.0 };
        top.lerp_to_gamma(bottom, k)
    };
    let mut mesh = egui::Mesh::default();
    let center = rect.center();
    mesh.colored_vertex(center, color_at(center.y));
    for p in &points {
        mesh.colored_vertex(*p, color_at(p.y));
    }
    let n = points.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    mesh
}

/// A soft shadow that rises on hover and settles when the card is pressed.
pub(crate) fn card_lift(ui: &egui::Ui, t: &Tokens, rect: Rect, press: theme::Press) {
    let lift = press.hover * (1.0 - press.down);
    if lift > 0.04 {
        let shadow = egui::Shadow { offset: [0, 3], blur: (14.0 * lift) as u8, spread: 0, color: t.page_shadow.gamma_multiply(0.85 * lift) };
        ui.painter().add(shadow.as_shape(rect, CornerRadius::same(10)));
    }
}

pub fn show(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        // Axion: narrower margins on a phone.
        let side = if ui.available_width() < 560.0 { 14 } else { 36 };
        egui::Frame::NONE.inner_margin(egui::Margin { left: side, right: side, top: 22, bottom: 28 }).show(ui, |ui| {
            hero(app, ui, &t);
            ui.add_space(18.0);
            if !app.community_card_closed {
                egui::Frame::NONE
                    .fill(t.card)
                    .stroke(Stroke::new(1.0, t.border))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            widgets::artcraft_mark(ui, 28.0);
                            // Room for the close button on the right; the text wraps on a phone.
                            let room = (ui.available_width() - 36.0).max(120.0);
                            ui.vertical(|ui| {
                                ui.set_max_width(room);
                                ui.add(egui::Label::new(egui::RichText::new(tl!("Part of the Axion suite")).font(theme::semibold(15.0))).wrap());
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(tl!(
                                            "Axion PDF and Axion DocX work together: open, convert and save between PDF, Word and Markdown."
                                        ))
                                        .color(t.text_muted),
                                    )
                                    .wrap(),
                                );
                            });
                            // Closed for this run only: the card is back on the next launch.
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                if icons::button(ui, "x", 24.0, false, tl!("Close")).clicked() {
                                    app.community_card_closed = true;
                                }
                            });
                        });
                        ui.add_space(8.0);
                        if let Some(cmd) = widgets::community_links(ui) {
                            app.execute(cmd);
                        }
                    });
                ui.add_space(22.0);
            }

            egui::Frame::NONE
                .fill(t.card)
                .stroke(Stroke::new(1.0, t.border))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(egui::Margin::same(18))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(egui::RichText::new(tl!("Recommended tools")).font(theme::semibold(15.0)));
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(14.0, 14.0);
                        for id in RECOMMENDED {
                            let Some(g) = catalog::group(id) else { continue };
                            let (rect, resp) = ui.allocate_exact_size(vec2(190.0, 104.0), Sense::click());
                            resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tl!(g.label)));
                            let press = theme::Press::track(ui, &resp);
                            card_lift(ui, &t, rect, press);
                            let fill = press.fill(t.card, t.hover, t.pressed);
                            ui.painter().rect(rect, CornerRadius::same(10), fill, Stroke::new(1.0, t.divider), egui::StrokeKind::Inside);
                            let body = rect.translate(press.offset());
                            let color = egui::Color32::from_rgb(g.hue[0], g.hue[1], g.hue[2]);
                            icons::paint(ui, Rect::from_min_size(body.min + vec2(14.0, 14.0), vec2(22.0, 22.0)), g.icon, 21.0, color);
                            let mut title_job = egui::text::LayoutJob::simple_singleline(tl!(g.label).to_owned(), theme::semibold(13.5), t.text);
                            title_job.wrap =
                                egui::text::TextWrapping { max_width: body.width() - 58.0, max_rows: 1, break_anywhere: true, ..Default::default() };
                            let title = ui.fonts_mut(|f| f.layout_job(title_job));
                            let title_elided = title.elided;
                            ui.painter().galley(body.min + vec2(44.0, 25.0 - title.size().y * 0.5), title, t.text);
                            let blurb = g
                                .sections
                                .first()
                                .map(|s| s.items.iter().take(3).map(|i| tl!(i.label)).collect::<Vec<_>>().join(" · "))
                                .unwrap_or_default();
                            // Fixed-height cards must leave room for their action in every language.
                            let mut job = egui::text::LayoutJob::simple(blurb.clone(), theme::regular(11.5), t.text_muted, body.width() - 28.0);
                            job.wrap.max_rows = 2;
                            let galley = ui.fonts_mut(|f| f.layout_job(job));
                            let blurb_elided = galley.elided;
                            ui.painter().galley(body.min + vec2(14.0, 46.0), galley, t.text_muted);
                            ui.painter().text(
                                body.left_bottom() + vec2(14.0, -14.0),
                                Align2::LEFT_CENTER,
                                tl!("Use now"),
                                theme::medium(12.0),
                                t.accent_text,
                            );
                            let resp = if title_elided || blurb_elided { resp.on_hover_text(format!("{}\n{blurb}", tl!(g.label))) } else { resp };
                            if theme::hand(resp).clicked() {
                                app.select_tool(g.id);
                            }
                        }
                        let (rect, resp) = ui.allocate_exact_size(vec2(170.0, 104.0), Sense::click());
                        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tl!("Open file")));
                        let press = theme::Press::track(ui, &resp);
                        card_lift(ui, &t, rect, press);
                        ui.painter().rect(
                            rect,
                            CornerRadius::same(10),
                            press.fill(t.pasteboard, t.hover, t.pressed),
                            Stroke::new(1.0, t.divider),
                            egui::StrokeKind::Inside,
                        );
                        let body = rect.translate(press.offset());
                        icons::paint(ui, Rect::from_center_size(body.center() - vec2(0.0, 16.0), vec2(28.0, 28.0)), "folder-open", 26.0, t.icon);
                        ui.painter().text(body.center() + vec2(0.0, 22.0), Align2::CENTER_CENTER, tl!("Open file"), theme::semibold(13.0), t.text);
                        if theme::hand(resp).clicked() {
                            app.open_dialog();
                        }
                    });
                });

            #[cfg(not(target_arch = "wasm32"))]
            {
                ui.add_space(26.0);
                crate::folders_ui::section(app, ui);
            }

            ui.add_space(26.0);
            let mut clear = false;
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(tl!("Recent")).font(theme::semibold(17.0)));
                if !app.recent.is_empty() {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let resp = widgets::ghost_button(ui, "trash-2", tl!("Clear"));
                        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tl!("Clear Recent Files")));
                        clear = resp.on_hover_text(tl!("Clear Recent Files")).clicked();
                    });
                }
            });
            ui.add_space(8.0);
            if app.recent.is_empty() {
                ui.label(egui::RichText::new(tl!("Files you open in Axion PDF appear here. Drop a PDF anywhere to open it.")).color(t.text_muted));
            }
            let mut open = None;
            let mut remove = None;
            for r in &app.recent {
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &r.name));
                let press = theme::Press::track(ui, &resp);
                // The row, not its response: the pointer stays "in" the row over its remove button.
                let hovered = ui.rect_contains_pointer(rect);
                if hovered && press.hover < 0.04 && press.down < 0.04 {
                    // The remove button sits in the row, so the row response may not be hovered.
                    ui.painter().rect_filled(rect, CornerRadius::same(8), t.hover);
                } else {
                    press.wash(ui, rect, 8, false);
                }
                if hovered {
                    // Remove just this file from the list (#430); the file itself is untouched.
                    let x_rect = Rect::from_center_size(rect.right_center() - vec2(26.0, 0.0), vec2(28.0, 28.0));
                    let x = ui.interact(x_rect, ui.id().with(("remove-recent", &r.path)), Sense::click());
                    let label = crate::i18n::fmt(tl!("Remove {name} from Recent"), &[("name", &r.name)]);
                    x.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
                    if x.hovered() {
                        ui.painter().rect_filled(x_rect, CornerRadius::same(6), t.pressed);
                    }
                    icons::paint(ui, x_rect.shrink(6.0), "x", 16.0, t.icon);
                    if x.on_hover_text(tl!("Remove from Recent")).clicked() {
                        remove = Some(r.path.clone());
                    }
                }
                let body = rect.translate(press.offset());
                icons::paint(
                    ui,
                    Rect::from_min_size(body.min + vec2(10.0, 11.0), vec2(24.0, 24.0)),
                    "file-text",
                    22.0,
                    egui::Color32::from_rgb(0xE0, 0x3E, 0x3E),
                );
                // Room on the right for the remove button.
                let detail = ui.painter().text(
                    body.right_center() - vec2(48.0, 0.0),
                    Align2::RIGHT_CENTER,
                    format!("{}  ·  {}", crate::i18n::trn(crate::i18n::current(), r.pages as u64, "{n} page", "{n} pages"), human_size(r.size)),
                    theme::regular(12.0),
                    t.text_muted,
                );
                // The name and path stop short of the page count; hovering shows the whole path.
                let width = detail.left() - 16.0 - (body.left() + 46.0);
                widgets::row_text(ui, body.min + vec2(46.0, 15.0), crate::bidi::visual(&r.name), theme::medium(13.5), t.text, width);
                widgets::row_text(ui, body.min + vec2(46.0, 32.0), crate::bidi::visual(&r.path), theme::regular(11.0), t.text_faint, width);
                if theme::hand(resp).on_hover_text(&r.path).clicked() {
                    open = Some(r.path.clone());
                }
            }
            if let Some(p) = open {
                app.open_recent(&p);
            }
            if let Some(p) = remove {
                app.recent.retain(|r| r.path != p);
            }
            if clear {
                app.execute("file.clear_recent");
            }
            ui.add_space(20.0);
            widgets::section_title(ui, tl!("Privacy"));
            ui.label(
                egui::RichText::new(tl!("Axion PDF works offline. No telemetry, no account, and no cloud processing unless you add a provider."))
                    .color(t.text_muted),
            );
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translated_card_description_leaves_room_for_the_action() {
        let de = crate::i18n::Lang::from_code("de").unwrap();
        crate::i18n::set_current(de);
        let ctx = egui::Context::default();
        ctx.set_fonts(theme::font_definitions());
        let mut app = PdfCraftApp::new();
        let blurb = catalog::group("form")
            .unwrap()
            .sections
            .first()
            .unwrap()
            .items
            .iter()
            .take(3)
            .map(|item| crate::i18n::tr(de, item.label))
            .collect::<Vec<_>>()
            .join(" · ");
        let output = ctx
            .run_ui(egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1440.0, 900.0))), ..Default::default() }, |ui| {
                show(&mut app, ui)
            });
        crate::i18n::set_current(crate::i18n::Lang::EN);
        let text_shapes: Vec<_> =
            output.shapes.iter().filter_map(|shape| if let egui::Shape::Text(text) = &shape.shape { Some(text) } else { None }).collect();
        let description = text_shapes.iter().find(|text| text.galley.job.text == blurb).unwrap();
        let action = text_shapes
            .iter()
            .find(|text| text.galley.job.text == crate::i18n::tr(de, "Use now") && (text.pos.x - description.pos.x).abs() < 0.1)
            .unwrap();
        assert!(description.galley.rows.len() <= 2);
        assert!(
            description.pos.y + description.galley.size().y + 4.0 <= action.pos.y,
            "the translated description must leave a visible gap before its action"
        );
        output.drop_without_applying_deltas();
    }
}
