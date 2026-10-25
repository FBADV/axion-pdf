//! Axion PDF's window shell (the approved Axion layout): a sidebar card on the left with the
//! documents and tools, a floating command bar on top (title, page, zoom, search, Save and the
//! Convert menu), and on a phone a top bar and a bottom tab bar. It replaces the tab strip and the
//! mode bar; every command they offered is still here (the Menu holds the full menu).

use egui::{Align, Align2, Color32, CornerRadius, Layout, Rect, Sense, Stroke, vec2};

use crate::theme::{self, ThemePreference, Tokens};
use crate::{Dialog, Mode, PdfCraftApp, PropsTab, icons, window_controls};

/// The window is a phone below this width: top bar and bottom tabs, no sidebar.
pub const PHONE: f32 = 760.0;
/// Below this width (a tablet) the sidebar is an icon rail.
pub const TABLET: f32 = 1180.0;

const SIDEBAR: f32 = 268.0;
const RAIL: f32 = 100.0;
const GAP: f32 = 14.0;
const CARD_RADIUS: u8 = 20;

/// Which layout the window width calls for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    Phone,
    Tablet,
    Desktop,
}

pub fn size(ctx: &egui::Context) -> Size {
    let w = ctx.content_rect().width();
    if w < PHONE {
        Size::Phone
    } else if w < TABLET {
        Size::Tablet
    } else {
        Size::Desktop
    }
}

fn shadow(t: &Tokens) -> egui::Shadow {
    egui::Shadow { offset: [0, 6], blur: 22, spread: 0, color: t.page_shadow.gamma_multiply(0.75) }
}

/// A floating white card on the pasteboard.
fn card(t: &Tokens, radius: u8) -> egui::Frame {
    egui::Frame::NONE.fill(t.card).corner_radius(CornerRadius::same(radius)).shadow(shadow(t)).stroke(Stroke::new(1.0, t.divider))
}

/// The Axion mark, `size` points square.
fn mark(ui: &mut egui::Ui, size: f32) -> egui::Response {
    crate::widgets::artcraft_mark(ui, size)
}

/// A pill button: filled in the accent (`primary`) or white with a border.
fn pill(ui: &mut egui::Ui, t: &Tokens, icon: Option<&str>, label: &str, primary: bool, height: f32) -> egui::Response {
    let font = theme::semibold(13.5);
    let text_w = ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), font.clone(), t.text).size().x);
    let icon_w = if icon.is_some() { 24.0 } else { 0.0 };
    let (rect, resp) = ui.allocate_exact_size(vec2(text_w + icon_w + 32.0, height), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label));
    let press = theme::Press::track(ui, &resp);
    let (fill, ink, stroke) = if primary {
        (press.fill(t.accent, t.accent_text, t.accent_text), Color32::WHITE, Stroke::NONE)
    } else {
        (press.fill(t.card, t.hover, t.pressed), t.text, Stroke::new(1.0, t.border))
    };
    let body = rect.translate(press.offset());
    if primary {
        let lift = egui::Shadow { offset: [0, 4], blur: 12, spread: 0, color: t.accent.gamma_multiply(0.30) };
        ui.painter().add(lift.as_shape(body, CornerRadius::same((height / 2.0) as u8)));
    }
    ui.painter().rect(body, CornerRadius::same((height / 2.0) as u8), fill, stroke, egui::StrokeKind::Inside);
    let mut x = body.left() + 16.0;
    if let Some(icon) = icon {
        icons::paint(ui, Rect::from_center_size(egui::pos2(x + 8.0, body.center().y), vec2(17.0, 17.0)), icon, 16.0, ink);
        x += icon_w;
    }
    ui.painter().text(egui::pos2(x, body.center().y), Align2::LEFT_CENTER, label, font, ink);
    theme::hand(resp)
}

/// A soft grey capsule, `width` wide and 36 tall, around a few controls (page and zoom steppers).
fn capsule<R>(ui: &mut egui::Ui, t: &Tokens, width: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(18), t.pasteboard);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink2(vec2(3.0, 0.0))).layout(Layout::left_to_right(Align::Center)));
    child.spacing_mut().item_spacing.x = 2.0;
    add(&mut child)
}

/// The window controls this desktop needs drawn (none when the system draws them).
fn controls(app: &PdfCraftApp, ctx: &egui::Context) -> window_controls::Layout {
    let fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
    app.window_controls
        .clone()
        .filter(|_| app.draws_window_frame() && !fullscreen)
        .unwrap_or_else(|| window_controls::Layout { left: Vec::new(), right: Vec::new() })
}

/// Dragging an empty part of the shell moves the window; a double click maximizes it.
fn window_drag(ui: &egui::Ui, rect: Rect, salt: &str) {
    let drag = ui.interact(rect, ui.id().with(("axion-drag", salt)), Sense::click_and_drag());
    if drag.drag_started() {
        window_controls::desktop_grab(ui.ctx(), egui::ViewportCommand::StartDrag);
    }
    if drag.double_clicked() {
        let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
    }
}

// ---------------------------------------------------------------------------------------------
// Sidebar (desktop) and rail (tablet)
// ---------------------------------------------------------------------------------------------

/// The navigation entries: the home screen and the four workspaces of the original mode bar.
const NAV: [(&str, &str, Option<Mode>); 5] = [
    ("house", "Home", None),
    ("layout-grid", "All tools", Some(Mode::AllTools)),
    ("pen-line", "Edit", Some(Mode::Edit)),
    ("file-output", "Convert", Some(Mode::Convert)),
    ("signature", "E-Sign", Some(Mode::Sign)),
];

/// The four tool tiles under the open documents.
const TILES: [(&str, &str); 4] = [("organize", "Organize"), ("fill_sign", "Sign"), ("comment", "Comment"), ("protect", "Protect")];

pub fn sidebar(app: &mut PdfCraftApp, ui: &mut egui::Ui, rail: bool) {
    let t = Tokens::get(ui.ctx());
    let mac = cfg!(target_os = "macos") && app.integrated_titlebar;
    let top = if mac { crate::chrome::MAC_TITLE_BAR as i8 + 4 } else { GAP as i8 };
    egui::Panel::left("axion_sidebar")
        .resizable(false)
        .exact_size(if rail { RAIL } else { SIDEBAR })
        .show_separator_line(false)
        .frame(egui::Frame::NONE.fill(t.pasteboard).inner_margin(egui::Margin { left: GAP as i8, right: 0, top, bottom: GAP as i8 }))
        .show(ui, |ui| {
            if mac {
                // The band behind the traffic lights moves the window.
                let band = Rect::from_min_max(ui.max_rect().min - vec2(GAP, f32::from(top)), egui::pos2(ui.max_rect().right(), ui.max_rect().top()));
                window_drag(ui, band, "sidebar-band");
            }
            let height = ui.available_height();
            card(&t, CARD_RADIUS).inner_margin(egui::Margin::symmetric(if rail { 6 } else { 12 }, 16)).show(ui, |ui| {
                ui.set_min_height(height - 34.0);
                ui.set_width(ui.available_width());
                if rail {
                    rail_body(app, ui, &t);
                } else {
                    sidebar_body(app, ui, &t);
                }
            });
        });
}

fn sidebar_body(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ui.spacing_mut().item_spacing.y = 6.0;
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        mark(ui, 36.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.label(egui::RichText::new("Axion PDF").font(theme::semibold(15.5)).color(t.text));
            ui.label(egui::RichText::new("Bernardo Advogados").font(theme::regular(11.5)).color(t.text_muted));
        });
    });
    ui.add_space(10.0);
    // Open PDF, full width.
    let w = ui.available_width();
    ui.allocate_ui_with_layout(vec2(w, 44.0), Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
        let tip = crate::commands::command_tip(ui.ctx(), tl!("Open a PDF ({key})"), "file.open");
        if wide_primary(ui, t, "plus", tl!("Open PDF"), w).on_hover_text(tip).clicked() {
            app.open_dialog();
        }
    });
    ui.add_space(10.0);
    for (icon, label, mode) in NAV {
        let selected = match mode {
            None => app.active.is_none() && !app.combine_showing(),
            Some(m) => app.active.is_some() && app.mode == m,
        };
        if nav_row(ui, t, icon, tl!(label), selected).clicked() {
            go(app, mode);
        }
    }
    ui.add_space(12.0);
    section_title(ui, t, tl!("Open documents"));
    open_documents(app, ui, t);
    ui.add_space(12.0);
    section_title(ui, t, tl!("Tools"));
    let w = ui.available_width();
    let tile_w = ((w - 8.0) / 2.0).floor();
    egui::Grid::new("axion-tiles").num_columns(2).spacing(vec2(8.0, 8.0)).show(ui, |ui| {
        for (k, (id, short)) in TILES.iter().enumerate() {
            let Some(g) = pdfcraft_engine::catalog::group(id) else { continue };
            if tile(ui, t, g.icon, tl!(short), Color32::from_rgb(g.hue[0], g.hue[1], g.hue[2]), tile_w).on_hover_text(tl!(g.label)).clicked() {
                app.select_tool(g.id);
            }
            if k % 2 == 1 {
                ui.end_row();
            }
        }
    });
    // Settings and the theme at the bottom.
    ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
        ui.horizontal(|ui| {
            if nav_icon_text(ui, t, "settings", tl!("Preferences…")).clicked() {
                app.dialog = Some(Dialog::Preferences);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| theme_switch(app, ui, t));
        });
    });
}

fn rail_body(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    ui.spacing_mut().item_spacing.y = 4.0;
    ui.vertical_centered(|ui| {
        mark(ui, 40.0);
        ui.add_space(8.0);
        let tip = crate::commands::command_tip(ui.ctx(), tl!("Open a PDF ({key})"), "file.open");
        if rail_item(ui, t, "plus", tl!("Open"), false, true).on_hover_text(tip).clicked() {
            app.open_dialog();
        }
        for (icon, label, mode) in NAV {
            let selected = match mode {
                None => app.active.is_none() && !app.combine_showing(),
                Some(m) => app.active.is_some() && app.mode == m,
            };
            if rail_item(ui, t, icon, tl!(label), selected, false).clicked() {
                go(app, mode);
            }
        }
        // Open documents as numbered chips: the active one is highlighted.
        if !app.views.is_empty() {
            ui.add_space(6.0);
            ui.painter().hline(ui.max_rect().x_range().shrink(14.0), ui.cursor().top(), Stroke::new(1.0, t.divider));
            ui.add_space(8.0);
            let mut pick = None;
            for i in 0..app.views.len().min(6) {
                let name = crate::canvas::tab_label(app, i).unwrap_or_default();
                let selected = app.active == Some(i);
                let (rect, resp) = ui.allocate_exact_size(vec2(52.0, 40.0), Sense::click());
                let press = theme::Press::track(ui, &resp);
                press.wash(ui, rect, 12, selected);
                icons::paint(ui, rect, "file-text", 20.0, if selected { t.accent } else { t.icon });
                if resp.on_hover_text(&name).clicked() {
                    pick = Some(i);
                }
            }
            if let Some(i) = pick {
                app.active = Some(i);
            }
        }
    });
    ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
        theme_cycle(app, ui, t);
        if icons::button(ui, "settings", 40.0, false, tl!("Preferences…")).clicked() {
            app.dialog = Some(Dialog::Preferences);
        }
    });
}

/// Go to the home screen (`None`) or a workspace.
fn go(app: &mut PdfCraftApp, mode: Option<Mode>) {
    match mode {
        None => {
            app.active = None;
            app.combine_tab.focused = false;
        }
        Some(m) => {
            if app.active.is_none() && !app.views.is_empty() {
                app.active = Some(0);
            }
            app.switch_mode(m, true);
        }
    }
}

fn section_title(ui: &mut egui::Ui, t: &Tokens, text: &str) {
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        ui.label(egui::RichText::new(text.to_uppercase()).font(theme::semibold(11.0)).color(t.text_muted));
    });
}

fn wide_primary(ui: &mut egui::Ui, t: &Tokens, icon: &str, label: &str, width: f32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 44.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    let press = theme::Press::track(ui, &resp);
    let body = rect.translate(press.offset());
    let lift = egui::Shadow { offset: [0, 6], blur: 14, spread: 0, color: t.accent.gamma_multiply(0.28) };
    ui.painter().add(lift.as_shape(body, CornerRadius::same(22)));
    ui.painter().rect_filled(body, CornerRadius::same(22), press.fill(t.accent, t.accent_text, t.accent_text));
    let font = theme::semibold(14.0);
    let text_w = ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), font.clone(), Color32::WHITE).size().x);
    let start = body.center().x - (text_w + 26.0) / 2.0;
    icons::paint(ui, Rect::from_center_size(egui::pos2(start + 9.0, body.center().y), vec2(18.0, 18.0)), icon, 17.0, Color32::WHITE);
    ui.painter().text(egui::pos2(start + 26.0, body.center().y), Align2::LEFT_CENTER, label, font, Color32::WHITE);
    theme::hand(resp)
}

fn nav_row(ui: &mut egui::Ui, t: &Tokens, icon: &str, label: &str, selected: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, label));
    let press = theme::Press::track(ui, &resp);
    press.wash(ui, rect, 12, selected);
    let ink = if selected { t.accent_text } else { t.text };
    let icon_ink = if selected { t.accent } else { t.text_muted };
    let body = rect.translate(press.offset());
    icons::paint(ui, Rect::from_center_size(egui::pos2(body.left() + 22.0, body.center().y), vec2(18.0, 18.0)), icon, 18.0, icon_ink);
    let font = if selected { theme::semibold(14.0) } else { theme::medium(14.0) };
    ui.painter().text(egui::pos2(body.left() + 44.0, body.center().y), Align2::LEFT_CENTER, label, font, ink);
    theme::hand(resp)
}

fn nav_icon_text(ui: &mut egui::Ui, t: &Tokens, icon: &str, label: &str) -> egui::Response {
    let font = theme::regular(13.0);
    let w = ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), font.clone(), t.text).size().x);
    let (rect, resp) = ui.allocate_exact_size(vec2(w + 40.0, 34.0), Sense::click());
    let press = theme::Press::track(ui, &resp);
    press.wash(ui, rect, 10, false);
    icons::paint(ui, Rect::from_center_size(egui::pos2(rect.left() + 16.0, rect.center().y), vec2(17.0, 17.0)), icon, 17.0, t.text_muted);
    ui.painter().text(egui::pos2(rect.left() + 32.0, rect.center().y), Align2::LEFT_CENTER, label, font, t.text_muted);
    theme::hand(resp)
}

fn rail_item(ui: &mut egui::Ui, t: &Tokens, icon: &str, label: &str, selected: bool, primary: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(68.0, 56.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, label));
    let press = theme::Press::track(ui, &resp);
    let body = rect.translate(press.offset());
    if primary {
        let chip = Rect::from_center_size(egui::pos2(body.center().x, body.top() + 18.0), vec2(44.0, 30.0));
        ui.painter().rect_filled(chip, CornerRadius::same(15), press.fill(t.accent, t.accent_text, t.accent_text));
        icons::paint(ui, chip, icon, 18.0, Color32::WHITE);
    } else {
        press.wash(ui, body, 14, selected);
        icons::paint(ui, Rect::from_center_size(egui::pos2(body.center().x, body.top() + 18.0), vec2(20.0, 20.0)), icon, 20.0, if selected { t.accent } else { t.icon });
    }
    let font = if selected { theme::semibold(10.5) } else { theme::medium(10.5) };
    let ink = if selected { t.accent_text } else { t.text_muted };
    // Long labels (other languages) are cut to the rail's width.
    let mut job = egui::text::LayoutJob::simple_singleline(label.to_owned(), font, ink);
    job.wrap = egui::text::TextWrapping { max_width: 64.0, max_rows: 1, break_anywhere: true, ..Default::default() };
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let pos = egui::pos2(body.center().x - galley.size().x / 2.0, body.bottom() - 18.0);
    ui.painter().galley(pos, galley, ink);
    theme::hand(resp.on_hover_text(label))
}

fn tile(ui: &mut egui::Ui, t: &Tokens, icon: &str, label: &str, hue: Color32, width: f32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 64.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    let press = theme::Press::track(ui, &resp);
    let body = rect.translate(press.offset());
    ui.painter().rect(body, CornerRadius::same(14), press.fill(t.card, t.hover, t.pressed), Stroke::new(1.0, t.divider), egui::StrokeKind::Inside);
    let chip = Rect::from_min_size(body.min + vec2(10.0, 9.0), vec2(26.0, 26.0));
    ui.painter().rect_filled(chip, CornerRadius::same(8), hue.gamma_multiply(if t.dark() { 0.28 } else { 0.16 }));
    icons::paint(ui, chip, icon, 16.0, hue);
    let mut job = egui::text::LayoutJob::simple_singleline(label.to_owned(), theme::semibold(12.0), t.text);
    job.wrap = egui::text::TextWrapping { max_width: width - 20.0, max_rows: 1, break_anywhere: true, ..Default::default() };
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    ui.painter().galley(egui::pos2(body.left() + 10.0, body.bottom() - 22.0), galley, t.text);
    theme::hand(resp.on_hover_text(label))
}

/// The documents open in the app, as cards: click to show, × to close (unsaved changes ask).
fn open_documents(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    if app.views.is_empty() && !app.combine_tab.open {
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.label(egui::RichText::new(tl!("No document open")).font(theme::regular(12.5)).color(t.text_muted));
        });
        return;
    }
    let mut pick = None;
    let mut close = None;
    let max_h = (ui.available_height() - 240.0).clamp(60.0, 320.0);
    egui::ScrollArea::vertical().id_salt("axion-open-docs").max_height(max_h).auto_shrink([false, true]).show(ui, |ui| {
        for i in 0..app.views.len() {
            let Some(doc) = app.session.get(app.views[i].id) else { continue };
            let Some(name) = crate::canvas::tab_label(app, i) else { continue };
            let pages = doc.info.pages.len();
            let dirty = doc.dirty;
            let selected = app.active == Some(i) && !app.combine_showing();
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::click());
            resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, &name));
            let press = theme::Press::track(ui, &resp);
            if selected {
                ui.painter().rect(rect, CornerRadius::same(12), t.pasteboard, Stroke::new(1.0, t.divider), egui::StrokeKind::Inside);
            } else {
                press.wash(ui, rect, 12, false);
            }
            // A page with a coral PDF band.
            let thumb = Rect::from_min_size(rect.min + vec2(10.0, 8.0), vec2(28.0, 36.0));
            ui.painter().rect(thumb, CornerRadius::same(5), Color32::WHITE, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
            let band = Rect::from_min_size(egui::pos2(thumb.left() + 3.0, thumb.bottom() - 12.0), vec2(22.0, 8.0));
            ui.painter().rect_filled(band, CornerRadius::same(2), t.badge_new);
            let text_w = rect.width() - 84.0;
            crate::widgets::row_text(ui, rect.min + vec2(48.0, 17.0), name.clone(), if selected { theme::semibold(13.0) } else { theme::medium(13.0) }, t.text, text_w);
            let detail = format!("{}{}", crate::i18n::trn(crate::i18n::current(), pages as u64, "{n} page", "{n} pages"), if dirty { " · ●" } else { "" });
            ui.painter().text(rect.min + vec2(48.0, 36.0), Align2::LEFT_CENTER, detail, theme::regular(11.5), t.text_muted);
            let hovered = ui.rect_contains_pointer(rect);
            if hovered || selected {
                let x_rect = Rect::from_center_size(rect.right_center() - vec2(18.0, 0.0), vec2(26.0, 26.0));
                let x = ui.interact(x_rect, ui.id().with(("axion-close-doc", i)), Sense::click());
                if x.hovered() {
                    ui.painter().rect_filled(x_rect, CornerRadius::same(8), t.pressed);
                }
                icons::paint(ui, x_rect, "x", 14.0, t.icon);
                if x.on_hover_text(tl!("Close Tab")).clicked() {
                    close = Some(i);
                }
            }
            if resp.clicked() {
                pick = Some(i);
            }
            if resp.middle_clicked() {
                close = Some(i);
            }
        }
        if app.combine_tab.open {
            let selected = app.combine_showing();
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
            let press = theme::Press::track(ui, &resp);
            press.wash(ui, rect, 12, selected);
            icons::paint(ui, Rect::from_center_size(rect.left_center() + vec2(24.0, 0.0), vec2(18.0, 18.0)), "files", 18.0, t.icon);
            ui.painter().text(rect.left_center() + vec2(48.0, 0.0), Align2::LEFT_CENTER, tl!("Combine files"), theme::medium(13.0), t.text);
            if resp.clicked() {
                app.open_combine_tab();
            }
        }
    });
    if let Some(i) = close
        && let Some(view) = app.views.get(i)
    {
        app.tab_action(crate::tab_context::Action::Close(view.id));
    } else if let Some(i) = pick {
        app.active = Some(i);
        app.combine_tab.focused = false;
    }
}

/// Light / dark as a two-segment switch (right-click: system and appearance settings).
fn theme_switch(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let dark = t.dark();
    let (rect, resp) = ui.allocate_exact_size(vec2(68.0, 30.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tl!("Display theme")));
    ui.painter().rect_filled(rect, CornerRadius::same(15), t.pasteboard);
    let (sun, moon) = rect.split_left_right_at_fraction(0.5);
    let knob = if dark { moon } else { sun }.shrink(3.0);
    ui.painter().rect_filled(knob, CornerRadius::same(12), if dark { t.hover } else { Color32::WHITE });
    icons::paint(ui, sun, "sun", 14.0, if dark { t.text_muted } else { t.text });
    icons::paint(ui, moon, "moon", 14.0, if dark { t.text } else { t.text_muted });
    let resp = resp.on_hover_text(format!(
        "{}: {}",
        tl!("Display theme"),
        match app.theme_preference {
            ThemePreference::System => tl!("Use system setting"),
            ThemePreference::Light => tl!("Light gray"),
            ThemePreference::Dark => tl!("Dark gray"),
        }
    ));
    if resp.clicked() {
        app.execute(if dark { "view.theme.light" } else { "view.theme.dark" });
    }
    egui::Popup::context_menu(&resp).id(resp.id.with("axion-theme")).show(|ui| {
        if ui.button(tl!("Use system setting")).clicked() {
            app.execute("view.theme.system");
            ui.close();
        }
        if ui.button(tl!("Appearance settings…")).clicked() {
            app.dialog = Some(Dialog::Preferences);
            ui.close();
        }
    });
}

fn theme_cycle(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let icon = if t.dark() { "moon" } else { "sun" };
    if icons::button(ui, icon, 40.0, false, tl!("Display theme")).clicked() {
        app.execute(if t.dark() { "view.theme.light" } else { "view.theme.dark" });
    }
}

// ---------------------------------------------------------------------------------------------
// Command bar (desktop and tablet)
// ---------------------------------------------------------------------------------------------

/// The Save as… formats of the Convert menu: format badge, label, hint, command.
pub const CONVERT: [(&str, &str, &str, &str); 7] = [
    ("DOCX", "Word", "Editable text, headings and tables", "export.docx"),
    ("MD", "Markdown", "Clean text for AI and notes", "export.markdown"),
    ("PPT", "PowerPoint", "One page per slide", "export.pptx"),
    ("IMG", "Images", "PNG, JPG or TIFF per page", "export.image"),
    ("HTML", "Web page (HTML)", "Text and images in one file", "export.html"),
    ("RTF", "Rich Text Format (.rtf)", "Formatted text", "export.rtf"),
    ("TXT", "Text (plain)", "Text only", "export.text"),
];

pub fn command_bar(app: &mut PdfCraftApp, ui: &mut egui::Ui, tablet: bool) {
    let t = Tokens::get(ui.ctx());
    let ctx = ui.ctx().clone();
    let mac = cfg!(target_os = "macos") && app.integrated_titlebar;
    let controls = controls(app, &ctx);
    let top = if mac { 6 } else { GAP as i8 };
    egui::Panel::top("axion_command_bar")
        .exact_size(64.0 + f32::from(top))
        .show_separator_line(false)
        .frame(egui::Frame::NONE.fill(t.pasteboard).inner_margin(egui::Margin { left: GAP as i8, right: GAP as i8, top, bottom: 0 }))
        .show(ui, |ui| {
            let full = ui.max_rect();
            card(&t, 18).inner_margin(egui::Margin { left: 16, right: 10, top: 0, bottom: 0 }).show(ui, |ui| {
                ui.set_height(60.0);
                ui.set_width(ui.available_width());
                window_drag(ui, full, "command-bar");
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if !controls.left.is_empty() {
                        ui.scope(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            window_controls::buttons(ui, &controls.left);
                        });
                        ui.add_space(window_controls::SIDE_GAP - 4.0);
                    }
                    crate::chrome::main_menu(app, ui);
                    title_block(app, ui, &t, tablet);
                    if let Some(index) = app.active
                        && !app.combine_showing()
                        && index < app.views.len()
                    {
                        page_capsule(app, ui, &t, index);
                        if !tablet {
                            zoom_capsule(app, ui, &t, index);
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        if !controls.right.is_empty() {
                            let size = egui::vec2(window_controls::width(&controls.right) - window_controls::SIDE_GAP, window_controls::BUTTON);
                            ui.allocate_ui_with_layout(size, Layout::left_to_right(Align::Center), |ui| {
                                ui.spacing_mut().item_spacing.x = 0.0;
                                window_controls::buttons(ui, &controls.right);
                            });
                            ui.add_space(window_controls::SIDE_GAP - 4.0);
                        }
                        let has_doc = app.active.is_some() && !app.combine_showing();
                        ui.add_enabled_ui(has_doc, |ui| convert_button(app, ui, &t));
                        if !tablet {
                            ui.add_enabled_ui(has_doc, |ui| {
                                let tip = crate::commands::command_tip(ui.ctx(), tl!("Save ({key})"), "file.save");
                                if pill(ui, &t, Some("save"), tl!("Save"), false, 40.0).on_hover_text(tip).clicked() {
                                    app.run_command("file.save");
                                }
                            });
                        } else {
                            ui.add_enabled_ui(has_doc, |ui| {
                                let tip = crate::commands::command_tip(ui.ctx(), tl!("Save ({key})"), "file.save");
                                if icons::button(ui, "save", 40.0, false, &tip).clicked() {
                                    app.run_command("file.save");
                                }
                            });
                        }
                        ui.add_enabled_ui(has_doc, |ui| {
                            let tip = crate::commands::command_tip(ui.ctx(), tl!("Print ({key})"), "print.dialog");
                            let may_print = !has_doc || pdfcraft_engine::commands::command("print.dialog").is_some_and(|c| app.command_enabled(c));
                            let mut print = ui.add_enabled_ui(may_print, |ui| icons::button(ui, "printer", 38.0, false, &tip)).inner;
                            if !may_print {
                                print = print.on_disabled_hover_text(crate::print_refused());
                            }
                            if print.clicked() {
                                app.run_command("print.dialog");
                            }
                            let tip = crate::commands::command_tip(ui.ctx(), tl!("Document properties ({key})"), "file.properties");
                            if icons::button(ui, "info", 38.0, false, &tip).clicked() {
                                app.dialog = Some(Dialog::Properties(PropsTab::Description));
                            }
                        });
                        if tablet {
                            if icons::button(ui, "search", 38.0, false, tl!("Find tools and commands")).clicked() {
                                app.palette_open = true;
                            }
                        } else if crate::widgets::search_box(ui, tl!("Find tools and commands"), 230.0).clicked() {
                            app.palette_open = true;
                        }
                    });
                });
            });
        });
}

/// The document's name and where it was saved, or the app's name on the home screen.
fn title_block(app: &PdfCraftApp, ui: &mut egui::Ui, t: &Tokens, tablet: bool) {
    let (title, detail) = match app.active.and_then(|i| app.views.get(i).map(|v| (i, v.id))) {
        Some((i, id)) if !app.combine_showing() => {
            let name = crate::canvas::tab_label(app, i).unwrap_or_default();
            let detail = match app.session.get(id) {
                Some(doc) if doc.dirty => tl!("Unsaved changes").to_owned(),
                Some(doc) => crate::i18n::trn(crate::i18n::current(), doc.info.pages.len() as u64, "{n} page", "{n} pages"),
                None => String::new(),
            };
            (name, detail)
        }
        _ if app.combine_showing() => (tl!("Combine files").to_owned(), String::new()),
        _ => ("Axion PDF".to_owned(), tl!("Home").to_owned()),
    };
    let width = if tablet { 200.0 } else { 300.0 };
    ui.allocate_ui_with_layout(vec2(width, 44.0), Layout::top_down(Align::Min), |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.add_space(3.0);
        ui.add(egui::Label::new(egui::RichText::new(title).font(theme::semibold(15.0)).color(t.text)).truncate());
        ui.add(egui::Label::new(egui::RichText::new(detail).font(theme::regular(12.0)).color(t.text_muted)).truncate());
    });
}

fn page_capsule(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens, index: usize) {
    let pages = app.active_ids().and_then(|(_, id)| app.session.get(id)).map_or(0, |d| d.info.pages.len());
    let view = &mut app.views[index];
    let current = view.current;
    capsule(ui, t, 128.0, |ui| {
        if icons::button(ui, "chevron-left", 30.0, false, tl!("Previous page")).clicked() && current > 0 {
            view.go_to_page(current - 1);
        }
        let shown = if pages == 0 { 0 } else { current.min(pages - 1) + 1 };
        let text = crate::i18n::fmt(tl!("{n} of {count}"), &[("n", &shown.to_string()), ("count", &pages.to_string())]);
        ui.allocate_ui_with_layout(vec2(56.0, 30.0), Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
            ui.label(egui::RichText::new(text).font(theme::semibold(13.0)).color(t.text));
        });
        if icons::button(ui, "chevron-right", 30.0, false, tl!("Next page")).clicked() && current + 1 < pages {
            view.go_to_page(current + 1);
        }
    });
}

fn zoom_capsule(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens, index: usize) {
    let view = &mut app.views[index];
    capsule(ui, t, 128.0, |ui| {
        if icons::button(ui, "zoom-out", 30.0, false, tl!("Zoom out")).clicked() {
            view.zoom_step(false);
        }
        ui.allocate_ui_with_layout(vec2(56.0, 30.0), Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
            ui.label(egui::RichText::new(format!("{:.0}%", view.zoom * 100.0)).font(theme::semibold(13.0)).color(t.text));
        });
        if icons::button(ui, "zoom-in", 30.0, false, tl!("Zoom in")).clicked() {
            view.zoom_step(true);
        }
    });
}

/// The Convert button and its menu of formats.
fn convert_button(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let resp = pill(ui, t, Some("file-output"), tl!("Convert"), true, 40.0);
    let mut run = None;
    egui::Popup::menu(&resp).id(resp.id.with("axion-convert")).show(|ui| {
        ui.set_min_width(300.0);
        ui.label(egui::RichText::new(tl!("Save a copy as").to_uppercase()).font(theme::semibold(11.0)).color(t.text_muted));
        ui.add_space(4.0);
        for (badge, label, hint, command) in CONVERT {
            if format_row(ui, t, badge, tl!(label), tl!(hint)).clicked() {
                run = Some(command);
                ui.close();
            }
        }
    });
    if let Some(command) = run {
        app.execute(command);
    }
}

/// One format in the Convert menu: a tinted badge, the format's name and a line about it.
pub fn format_row(ui: &mut egui::Ui, t: &Tokens, badge: &str, label: &str, hint: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width().max(280.0), 52.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    let press = theme::Press::track(ui, &resp);
    press.wash(ui, rect, 12, false);
    let hue = match badge {
        "DOCX" => Color32::from_rgb(0x2C, 0x5F, 0xD3),
        "MD" => Color32::from_rgb(0x0D, 0x74, 0x69),
        "PPT" => Color32::from_rgb(0xA8, 0x48, 0x1F),
        "IMG" => Color32::from_rgb(0x63, 0x45, 0xC0),
        _ => t.text_muted,
    };
    let chip = Rect::from_min_size(rect.min + vec2(10.0, 9.0), vec2(36.0, 34.0));
    ui.painter().rect_filled(chip, CornerRadius::same(10), hue.gamma_multiply(if t.dark() { 0.30 } else { 0.16 }));
    ui.painter().text(chip.center(), Align2::CENTER_CENTER, badge, theme::semibold(10.0), if t.dark() { t.text } else { hue });
    ui.painter().text(rect.min + vec2(58.0, 18.0), Align2::LEFT_CENTER, label, theme::semibold(13.5), t.text);
    ui.painter().text(rect.min + vec2(58.0, 36.0), Align2::LEFT_CENTER, hint, theme::regular(12.0), t.text_muted);
    theme::hand(resp)
}

// ---------------------------------------------------------------------------------------------
// Phone
// ---------------------------------------------------------------------------------------------

pub fn phone_top(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    egui::Panel::top("axion_phone_top").exact_size(64.0).show_separator_line(false).frame(egui::Frame::NONE.fill(t.pasteboard).inner_margin(egui::Margin::symmetric(12, 8))).show(
        ui,
        |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if app.active.is_some() || app.combine_showing() {
                    if round_button(ui, &t, "chevron-left", tl!("Home")).clicked() {
                        go(app, None);
                    }
                } else {
                    mark(ui, 36.0);
                }
                let room = (ui.available_width() - 2.0 * 52.0).max(60.0);
                ui.allocate_ui_with_layout(vec2(room, 48.0), Layout::top_down(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.add_space(4.0);
                    let (title, detail) = match app.active.and_then(|i| app.views.get(i).map(|v| (i, v.id, v.current))) {
                        Some((i, id, current)) if !app.combine_showing() => {
                            let pages = app.session.get(id).map_or(0, |d| d.info.pages.len());
                            (
                                crate::canvas::tab_label(app, i).unwrap_or_default(),
                                crate::i18n::fmt(tl!("Page {n} of {count}"), &[("n", &(current.min(pages.saturating_sub(1)) + 1).to_string()), ("count", &pages.to_string())]),
                            )
                        }
                        _ => ("Axion PDF".to_owned(), "Bernardo Advogados".to_owned()),
                    };
                    ui.add(egui::Label::new(egui::RichText::new(title).font(theme::semibold(15.0)).color(t.text)).truncate());
                    ui.label(egui::RichText::new(detail).font(theme::regular(12.0)).color(t.text_muted));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    crate::chrome::main_menu_with(app, ui, |ui, t| round_button(ui, t, "ellipsis", tl!("Menu")));
                });
            });
        },
    );
}

fn round_button(ui: &mut egui::Ui, t: &Tokens, icon: &str, tip: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tip));
    let press = theme::Press::track(ui, &resp);
    let body = rect.translate(press.offset());
    let lift = egui::Shadow { offset: [0, 1], blur: 4, spread: 0, color: t.page_shadow };
    ui.painter().add(lift.as_shape(body, CornerRadius::same(22)));
    ui.painter().rect_filled(body, CornerRadius::same(22), press.fill(t.card, t.hover, t.pressed));
    icons::paint(ui, body, icon, 19.0, t.text);
    theme::hand(resp.on_hover_text(tip))
}

/// The phone's bottom tab bar: Home, Document, Mark up, Tools, Send.
pub fn phone_tabs(app: &mut PdfCraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    egui::Panel::bottom("axion_phone_tabs").exact_size(88.0).show_separator_line(false).frame(egui::Frame::NONE.fill(t.pasteboard).inner_margin(egui::Margin { left: 12, right: 12, top: 6, bottom: 12 })).show(
        ui,
        |ui| {
            card(&t, 24).show(ui, |ui| {
                ui.set_height(68.0);
                ui.set_width(ui.available_width());
                let has_doc = app.active.is_some() && !app.combine_showing();
                let items: [(&str, &str, bool); 5] = [
                    ("house", tl!("Home"), app.active.is_none() && !app.combine_showing()),
                    ("file-text", tl!("Document"), has_doc && app.left_open == false),
                    ("highlighter", tl!("Comment"), has_doc && app.mode == Mode::Edit && app.left_open),
                    ("layout-grid", tl!("Tools"), app.left_open && app.mode == Mode::AllTools),
                    ("share-2", tl!("Convert"), false),
                ];
                let w = ui.available_width() / 5.0;
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for (k, (icon, label, selected)) in items.into_iter().enumerate() {
                        let (rect, resp) = ui.allocate_exact_size(vec2(w, 60.0), Sense::click());
                        resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, label));
                        let pill = Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + 20.0), vec2(52.0, 30.0));
                        if selected {
                            ui.painter().rect_filled(pill, CornerRadius::same(15), t.accent_soft);
                        }
                        let ink = if selected { t.accent_text } else { t.text_muted };
                        icons::paint(ui, pill, icon, 20.0, if selected { t.accent } else { t.text_muted });
                        ui.painter().text(egui::pos2(rect.center().x, rect.bottom() - 12.0), Align2::CENTER_CENTER, label, if selected { theme::semibold(11.0) } else { theme::medium(11.0) }, ink);
                        let resp = theme::hand(resp);
                        if k == 4 {
                            let mut run = None;
                            egui::Popup::menu(&resp).id(resp.id.with("axion-phone-convert")).show(|ui| {
                                ui.set_min_width(290.0);
                                for (badge, label, hint, command) in CONVERT {
                                    if format_row(ui, &t, badge, tl!(label), tl!(hint)).clicked() {
                                        run = Some(command);
                                        ui.close();
                                    }
                                }
                            });
                            if let Some(command) = run {
                                if has_doc {
                                    app.execute(command);
                                } else {
                                    app.open_dialog();
                                }
                            }
                        } else if resp.clicked() {
                            match k {
                                0 => go(app, None),
                                1 => {
                                    if app.active.is_none() && !app.views.is_empty() {
                                        app.active = Some(0);
                                    }
                                    app.left_open = false;
                                    app.right = None;
                                }
                                2 => {
                                    if has_doc {
                                        app.select_tool("comment");
                                    }
                                }
                                _ => {
                                    if app.active.is_none() && !app.views.is_empty() {
                                        app.active = Some(0);
                                    }
                                    app.switch_mode(Mode::AllTools, true);
                                    app.left_open = true;
                                }
                            }
                        }
                    }
                });
            });
        },
    );
}
