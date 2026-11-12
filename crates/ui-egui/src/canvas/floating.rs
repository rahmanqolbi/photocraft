//! Floating document windows drawn *inside* the workspace, the way Photoshop's document
//! windows float over the canvas: one application window, a title bar per document with
//! minimize / maximize / close, a border and a drop shadow, the canvas inside.
//!
//! Preferences › Workspace › Floating windows picks between this (`Workspace`) and one operating
//! system window per document (`Os`, see [`super::tabs::extra_windows`]).

use super::{PhotocraftApp, Rect, Sense, Stroke, canvas_view, pos2};
use crate::theme::Tokens;

/// Title bar height in points (Photoshop's document title bar is about this tall).
const TITLE_H: f32 = 24.0;
/// Never shrink a window below this, so its title bar and buttons stay usable.
const MIN_W: f32 = 240.0;
const MIN_H: f32 = 140.0;
/// The invisible grab band along an edge or corner, in points.
const GRIP: f32 = 5.0;
/// Cascade step between windows that have no remembered rectangle.
const CASCADE: f32 = 26.0;

/// Which edge or corner a resize drag pulled.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Edge {
    Left,
    Right,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// How big a new floating window is: the document at its current zoom (plus the title bar
/// and border), capped at 94% of `area` and never below the minimum, so a huge document
/// still opens on screen and a small one stays grabbable. The tear-off placeholder uses the
/// same size, so it previews the window that will appear on drop.
pub(super) fn default_size(area: Rect, doc_px: [f32; 2], zoom_pt: f32) -> egui::Vec2 {
    let want_w = (doc_px[0] * zoom_pt + 2.0).max(MIN_W);
    let want_h = (doc_px[1] * zoom_pt + TITLE_H + 2.0).max(MIN_H);
    let w = want_w.min((area.width() * 0.94).max(1.0)).min(area.width().max(1.0));
    let h = want_h.min((area.height() * 0.94).max(1.0)).min(area.height().max(1.0));
    egui::vec2(w, h)
}

/// A rectangle for a window that has none yet, cascaded by `slot`.
///
/// Like Photoshop, a new window is as big as the document at its current zoom (plus the title
/// bar and border), capped at most of the workspace so a huge document still opens on screen.
fn default_rect(area: Rect, slot: usize, doc_px: [f32; 2], zoom_pt: f32) -> Rect {
    let size = default_size(area, doc_px, zoom_pt);
    let step = CASCADE * (slot.min(12) as f32);
    let x = (area.left() + step).min(area.right() - size.x);
    let y = (area.top() + step).min(area.bottom() - size.y);
    Rect::from_min_size(pos2(x, y), size)
}

/// The tear-off placeholder, Photoshop-style: an empty, window-sized dark rectangle with the
/// document's title bar (title + ×), drawn on top of everything while the tab is dragged, so
/// a dragged tab shows exactly the window it will become before it is dropped.
///
/// Unlike the real window it is all paint: no interaction, no canvas, and the buttons are
/// strokes instead of icons (icons rasterize into the UI's own layer, which sits *under* the
/// tooltip layer the frame is drawn on, so they'd be invisible).
pub(super) fn draw_placeholder(ui: &egui::Ui, t: &Tokens, rect: Rect, title: &str) {
    let painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, ui.scope_id().with("tab-tear-off-preview")));
    painter.rect_filled(rect.translate(egui::vec2(0.0, 3.0)).expand(2.0), t.radius_sm, t.shadow);
    painter.rect_filled(rect, t.radius_sm, t.card);
    painter.rect_stroke(rect, t.radius_sm, Stroke::new(1.5, t.accent), egui::StrokeKind::Inside);
    let bar = Rect::from_min_size(rect.min, egui::vec2(rect.width(), TITLE_H));
    painter.rect_filled(bar.shrink(1.0), t.radius_sm, t.hover);
    let galley = crate::tab_strip::elided(ui, title, crate::theme::medium(12.0), t.text, (bar.width() - 56.0).max(8.0));
    painter.galley(pos2(bar.left() + 8.0, bar.center().y - galley.size().y / 2.0), galley, t.text);
    // The window's own buttons, right to left (close, maximize, minimize): strokes in 16 pt
    // boxes, mirroring the real title bar layout.
    let mut bx = rect.right() - 6.0;
    for i in 0..3 {
        bx -= 16.0;
        let c = pos2(bx, bar.center().y);
        let s = Stroke::new(1.25, t.text_dim);
        let d = egui::vec2(3.5, 3.5);
        match i {
            0 => {
                painter.line_segment([c - d, c + d], s);
                painter.line_segment([c - egui::vec2(3.5, -3.5), c + egui::vec2(-3.5, 3.5)], s);
            }
            1 => {
                painter.rect_stroke(Rect::from_center_size(c, egui::vec2(7.5, 7.5)), 1.0, s, egui::StrokeKind::Inside);
            }
            _ => {
                painter.line_segment([c - egui::vec2(4.0, 0.0), c + egui::vec2(4.0, 0.0)], s);
            }
        }
    }
}

/// Keep a rectangle inside `area` (a window can never be dragged out of reach).
fn clamp_to(area: Rect, r: Rect) -> Rect {
    let w = r.width().clamp(MIN_W.min(area.width().max(1.0)), area.width().max(1.0));
    let h = r.height().clamp(MIN_H.min(area.height().max(1.0)), area.height().max(1.0));
    let x = r.left().clamp(area.left(), (area.right() - w).max(area.left()));
    let y = r.top().clamp(area.top(), (area.bottom() - TITLE_H).max(area.top()));
    Rect::from_min_size(pos2(x, y), egui::vec2(w, h))
}

/// The eight resize handles of `r` (the top one starts below the title bar's buttons).
fn handles(r: Rect, include_top: bool) -> Vec<(Edge, Rect)> {
    let (l, t, rr, b) = (r.left(), r.top(), r.right(), r.bottom());
    let top = if include_top { t + TITLE_H } else { t };
    let mut v = vec![
        (Edge::Left, Rect::from_min_max(pos2(l - GRIP, top), pos2(l + GRIP, b))),
        (Edge::Right, Rect::from_min_max(pos2(rr - GRIP, top), pos2(rr + GRIP, b))),
        (Edge::Bottom, Rect::from_min_max(pos2(l, b - GRIP), pos2(rr, b + GRIP))),
    ];
    if include_top {
        v.push((Edge::TopLeft, Rect::from_min_max(pos2(l - GRIP, t - GRIP), pos2(l + GRIP, t + GRIP))));
        v.push((Edge::TopRight, Rect::from_min_max(pos2(rr - GRIP, t - GRIP), pos2(rr + GRIP, t + GRIP))));
    }
    v.push((Edge::BottomLeft, Rect::from_min_max(pos2(l - GRIP, b - GRIP), pos2(l + GRIP, b + GRIP))));
    v.push((Edge::BottomRight, Rect::from_min_max(pos2(rr - GRIP, b - GRIP), pos2(rr + GRIP, b + GRIP))));
    v
}

fn resize(r: Rect, edge: Edge, d: egui::Vec2) -> Rect {
    let (mut l, mut t, mut rr, mut b) = (r.left(), r.top(), r.right(), r.bottom());
    match edge {
        Edge::Left => l += d.x,
        Edge::Right => rr += d.x,
        Edge::Bottom => b += d.y,
        Edge::TopLeft => {
            l += d.x;
            t += d.y;
        }
        Edge::TopRight => {
            rr += d.x;
            t += d.y;
        }
        Edge::BottomLeft => {
            l += d.x;
            b += d.y;
        }
        Edge::BottomRight => {
            rr += d.x;
            b += d.y;
        }
    }
    // Hitting the minimum pins the edge being dragged (the opposite edge stays put).
    let left_edge = matches!(edge, Edge::Left | Edge::TopLeft | Edge::BottomLeft);
    let top_edge = matches!(edge, Edge::TopLeft | Edge::TopRight);
    let (l, rr) = if rr - l < MIN_W { if left_edge { (rr - MIN_W, rr) } else { (l, l + MIN_W) } } else { (l, rr) };
    let (t, b) = if b - t < MIN_H { if top_edge { (b - MIN_H, b) } else { (t, t + MIN_H) } } else { (t, b) };
    Rect::from_min_max(pos2(l, t), pos2(rr, b))
}

/// Draw every open floating window inside `area` (back to front). Returns false when there is
/// nothing to draw.
pub(super) fn show(app: &mut PhotocraftApp, ui: &mut egui::Ui, area: Rect) -> bool {
    let open: Vec<u64> = app.ui.windows.iter().filter(|w| w.open).map(|w| w.id).collect();
    if open.is_empty() {
        return false;
    }
    let t = Tokens::get(ui.ctx());
    // Which window the pointer pressed or dragged: it goes to the front (Photoshop raises a
    // window when it is clicked anywhere, title or canvas).
    let mut front: Option<u64> = None;
    // Geometry and flags are written back after the loop, when no borrow of `app` is held.
    let mut frames: Vec<WinFrame> = Vec::with_capacity(open.len());
    for (slot, wid) in open.iter().copied().enumerate() {
        if let Some(f) = paint_window(app, ui, area, wid, slot, &t, &mut front) {
            frames.push(f);
        }
    }
    let mut close_docs = Vec::new();
    for f in frames {
        if let Some(w) = app.ui.windows.iter_mut().find(|w| w.id == f.wid) {
            w.rect = Some([f.rect.min.x, f.rect.min.y, f.rect.width(), f.rect.height()]);
            w.maximized = f.maximized;
            w.minimized = f.minimized;
        }
        if f.close
            && let Some(w) = app.ui.windows.iter().find(|w| w.id == f.wid)
        {
            close_docs.push(w.document);
        }
    }
    // Raise the clicked window: the last entry draws last (and takes input first).
    if let Some(wid) = front
        && let Some(pos) = app.ui.windows.iter().position(|w| w.id == wid)
    {
        let raised = app.ui.windows.remove(pos);
        app.ui.windows.push(raised);
    }
    app.ui.windows.retain(|w| w.open);
    // Like Photoshop, closing a floating window closes its *document* (the guarded
    // `file.close` raises the save dialog on a dirty document); it never docks it back.
    for doc in close_docs {
        if let Err(e) = crate::menus::invoke(app, ui.ctx(), "file.close", serde_json::json!({"document": doc})) {
            app.ui.status = e;
            app.ui.status_error = true;
        }
    }
    true
}

/// What `paint_window` reports back to `show`: the window's new placement and flags, plus
/// whether its × asked to close the *document* (Photoshop closes it; it never docks it back).
struct WinFrame {
    wid: u64,
    rect: Rect,
    maximized: bool,
    minimized: bool,
    /// The × was pressed: close this document (guarded `file.close` prompts to save first).
    close: bool,
}

/// Draw one floating window: shadow, frame, title bar with its buttons, resize handles and the
/// document canvas. Any drag, click or button sets `front` (raise + activate) and the returned
/// frame is the window's new geometry and flags to write back.
fn paint_window(app: &mut PhotocraftApp, ui: &mut egui::Ui, area: Rect, wid: u64, slot: usize, t: &Tokens, front: &mut Option<u64>) -> Option<WinFrame> {
    let i = app.ui.windows.iter().position(|w| w.id == wid)?;
    let (doc, stored, mut minimized, mut maximized) = {
        let w = &app.ui.windows[i];
        (w.document, w.rect, w.minimized, w.maximized)
    };
    let doc_px = app.session.documents().get(doc).map(|st| [st.doc.size.width as f32, st.doc.size.height as f32])?;
    let active = app.session.active_index();
    let zoom_pt = app.ui.windows[i].view.zoom / ui.ctx().pixels_per_point().max(0.1);
    let mut rect = stored.map(|[x, y, w, h]| Rect::from_min_size(pos2(x, y), egui::vec2(w, h))).unwrap_or_else(|| default_rect(area, slot, doc_px, zoom_pt));
    rect = clamp_to(area, rect);
    if maximized {
        rect = area;
    }
    let shown = if minimized { Rect::from_min_size(rect.min, egui::vec2(rect.width(), TITLE_H)) } else { rect };
    let focused = active == Some(doc);
    let id = ui.scope_id().with(("docwin", wid));
    // Drop shadow, body and border.
    ui.painter().rect_filled(shown.translate(egui::vec2(0.0, 3.0)).expand(2.0), t.radius_sm, t.shadow);
    ui.painter().rect_filled(shown, t.radius_sm, t.card);
    let border = if focused { t.accent } else { t.card_border };
    ui.painter().rect_stroke(shown, t.radius_sm, Stroke::new(if focused { 1.5 } else { 1.0 }, border), egui::StrokeKind::Inside);
    let title = Rect::from_min_size(shown.min, egui::vec2(shown.width(), TITLE_H));
    ui.painter().rect_filled(title.shrink(1.0), t.radius_sm, if focused { t.hover } else { t.card });
    // The Photoshop title: "name @ 66.7% (Layer 1, RGB/8)".
    let zoom = app.ui.windows[i].view.zoom * 100.0;
    let text = super::tabs::tab_title_at(app, doc, zoom);
    let galley = crate::tab_strip::elided(ui, &text, crate::theme::medium(12.0), t.text, (title.width() - 76.0).max(8.0));
    ui.painter().galley(pos2(title.left() + 8.0, title.center().y - galley.size().y / 2.0), galley, t.text);
    // Drag the title bar, double-click it to maximize, click it to raise and activate.
    let drag = ui.interact(title, id.with("title"), Sense::click_and_drag());
    if drag.clicked() || drag.drag_started() {
        *front = Some(wid);
        if app.session.active_index() != Some(doc) {
            app.session.set_active(doc);
        }
    }
    if drag.double_clicked() {
        maximized = !maximized;
    }
    if drag.dragged() && !maximized {
        rect = clamp_to(area, Rect::from_min_size(rect.min + drag.drag_delta(), rect.size()));
    }
    // Title buttons, right to left: close, maximize (restore), minimize.
    let mut bx = shown.right() - 6.0;
    let mut button = |ui: &mut egui::Ui, icon: &str, hint: &str| -> bool {
        bx -= 16.0;
        let r = Rect::from_center_size(pos2(bx, title.center().y), egui::vec2(16.0, 16.0));
        let resp = ui.interact(r, id.with(("btn", bx as i32)), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(r, 3.0, t.hover);
        }
        crate::icons::paint(ui, r, icon, 11.0, if resp.hovered() { t.text } else { t.text_dim });
        resp.on_hover_text(hint).clicked()
    };
    let mut close = false;
    if button(ui, "x", "Close") {
        // Close the document (guarded `file.close` prompts to save first), not back into docking.
        close = true;
        *front = Some(wid);
    }
    if button(ui, if maximized { "copy" } else { "square" }, if maximized { "Restore" } else { "Maximize" }) {
        maximized = !maximized;
        *front = Some(wid);
    }
    if button(ui, "minus", if minimized { "Expand" } else { "Minimize" }) {
        minimized = !minimized;
        *front = Some(wid);
    }
    // Resize handles (a maximized window has none, and the title bar owns the top edge).
    if !maximized && !minimized {
        for (edge, r) in handles(shown, false) {
            let resp = ui.interact(r, id.with(("edge", edge as u8)), Sense::drag());
            if resp.hovered() || resp.dragged() {
                ui.ctx().set_cursor_icon(match edge {
                    Edge::Left | Edge::Right => egui::CursorIcon::ResizeHorizontal,
                    Edge::Bottom => egui::CursorIcon::ResizeVertical,
                    Edge::TopLeft | Edge::BottomRight => egui::CursorIcon::ResizeNwSe,
                    Edge::TopRight | Edge::BottomLeft => egui::CursorIcon::ResizeNeSw,
                });
            }
            if resp.dragged() {
                rect = clamp_to(area, resize(rect, edge, resp.drag_delta()));
                *front = Some(wid);
            }
        }
    }
    // The document itself.
    if !minimized {
        let client = Rect::from_min_max(pos2(shown.left() + 1.0, shown.top() + TITLE_H), pos2(shown.right() - 1.0, shown.bottom() - 1.0));
        if client.width() > 2.0 && client.height() > 2.0 {
            if ui.input(|i| i.pointer.primary_pressed() && i.pointer.interact_pos().is_some_and(|p| client.contains(p))) {
                *front = Some(wid);
                if app.session.active_index() != Some(doc) {
                    app.session.set_active(doc);
                }
            }
            let view = app.ui.windows[i].view.clone();
            let v = canvas_view(app, ui, doc, client, view, focused);
            if let Some(w) = app.ui.windows.iter_mut().find(|w| w.id == wid) {
                w.view = v;
            }
        }
    }
    Some(WinFrame { wid, rect, maximized, minimized, close })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect::from_min_size(pos2(10.0, 20.0), egui::vec2(1000.0, 700.0))
    }

    #[test]
    fn a_window_with_no_remembered_rect_cascades_inside_the_workspace() {
        let a = area();
        let first = default_rect(a, 0, [400.0, 300.0], 1.0);
        let second = default_rect(a, 1, [400.0, 300.0], 1.0);
        assert!(a.contains_rect(first), "{first:?} escapes {a:?}");
        assert!(a.contains_rect(second), "{second:?} escapes {a:?}");
        assert_eq!(second.min - first.min, egui::vec2(CASCADE, CASCADE));
        // Far down the cascade it stays inside instead of walking off the edge.
        let last = default_rect(a, 99, [400.0, 300.0], 1.0);
        assert!(a.contains_rect(last), "{last:?} escapes {a:?}");
        // Like Photoshop, the window is the document at its zoom (plus title bar), not a fixed
        // fraction of the workspace…
        assert!((first.width() - 402.0).abs() < 0.01, "{first:?}");
        assert!((first.height() - (300.0 + TITLE_H + 2.0)).abs() < 0.01, "{first:?}");
        // …and a document too big for the workspace is capped instead of overflowing it.
        let huge = default_rect(a, 0, [20000.0, 20000.0], 1.0);
        assert!(a.contains_rect(huge), "{huge:?} escapes {a:?}");
    }

    #[test]
    fn default_size_is_the_document_at_its_zoom_capped_and_minimum() {
        let a = area();
        // The same numbers a window with no remembered rectangle gets (slot 0 has no cascade).
        assert_eq!(default_size(a, [400.0, 300.0], 1.0), default_rect(a, 0, [400.0, 300.0], 1.0).size());
        assert!((default_size(a, [10.0, 10.0], 0.5).x - MIN_W).abs() < 0.01, "the minimum wins");
        assert!((default_size(a, [10.0, 10.0], 0.5).y - MIN_H).abs() < 0.01, "the minimum wins");
        assert!((default_size(a, [20000.0, 20000.0], 1.0).x - a.width() * 0.94).abs() < 0.01, "the cap wins");
    }

    #[test]
    fn a_dragged_window_is_kept_reachable() {
        let a = area();
        // Dragged far right and down: only the title bar must stay on screen.
        let r = clamp_to(a, Rect::from_min_size(pos2(5000.0, 5000.0), egui::vec2(400.0, 300.0)));
        assert!(r.right() <= a.right() + 0.01, "{r:?}");
        assert!(r.top() <= a.bottom() - TITLE_H + 0.01, "{r:?}");
        // Dragged far left and up.
        let r = clamp_to(a, Rect::from_min_size(pos2(-5000.0, -5000.0), egui::vec2(400.0, 300.0)));
        assert!(r.left() >= a.left() - 0.01 && r.top() >= a.top() - 0.01, "{r:?}");
    }

    #[test]
    fn resizing_from_each_edge_moves_that_edge_only() {
        let r = Rect::from_min_size(pos2(100.0, 100.0), egui::vec2(400.0, 300.0));
        let right = resize(r, Edge::Right, egui::vec2(50.0, 0.0));
        assert_eq!((right.left(), right.right()), (100.0, 550.0));
        let bottom = resize(r, Edge::Bottom, egui::vec2(0.0, -60.0));
        assert_eq!((bottom.top(), bottom.bottom()), (100.0, 340.0));
        let corner = resize(r, Edge::TopLeft, egui::vec2(20.0, 30.0));
        assert_eq!((corner.left(), corner.top(), corner.right(), corner.bottom()), (120.0, 130.0, 500.0, 400.0));
    }

    #[test]
    fn resizing_past_the_minimum_pins_the_dragged_edge() {
        let r = Rect::from_min_size(pos2(100.0, 100.0), egui::vec2(400.0, 300.0));
        // Pull the left edge far past the right one: the right edge stays and the width is
        // the minimum, instead of the window growing eastwards.
        let tiny = resize(r, Edge::Left, egui::vec2(1000.0, 0.0));
        assert_eq!(tiny.right(), r.right());
        assert!((tiny.width() - MIN_W).abs() < 0.01, "{tiny:?}");
        let tiny = resize(r, Edge::TopLeft, egui::vec2(0.0, 1000.0));
        assert_eq!(tiny.bottom(), r.bottom());
        assert!((tiny.height() - MIN_H).abs() < 0.01, "{tiny:?}");
    }

    #[test]
    fn the_handles_cover_every_edge_and_corner() {
        let r = Rect::from_min_size(pos2(100.0, 100.0), egui::vec2(400.0, 300.0));
        let edges = handles(r, true);
        for edge in [Edge::Left, Edge::Right, Edge::Bottom, Edge::TopLeft, Edge::TopRight, Edge::BottomLeft, Edge::BottomRight] {
            assert!(edges.iter().any(|(e, _)| *e == edge), "missing {edge:?}");
        }
    }

    fn app_with_a_floating_window() -> PhotocraftApp {
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", serde_json::json!({"width": 8, "height": 8})).unwrap();
        app.sync_views();
        crate::menus::invoke(&mut app, &egui::Context::default(), "window.arrange.floatAllInWindows", serde_json::json!({})).unwrap();
        assert!(app.ui.windows.iter().any(|w| w.open && w.floating));
        app
    }

    fn harness(app: PhotocraftApp) -> egui_kittest::Harness<'static, PhotocraftApp> {
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(800.0, 600.0)).build_ui_state(
            |ui, app: &mut PhotocraftApp| {
                let ctx = ui.ctx().clone();
                if !ctx.fonts(|f| f.families().contains(&egui::FontFamily::Name("medium".into()))) {
                    return;
                }
                let area = ui.available_rect_before_wrap();
                show(app, ui, area);
            },
            app,
        );
        PhotocraftApp::setup_context(&h.ctx, crate::theme::ThemeKind::Pro);
        h.run_steps(3);
        h
    }

    fn window_rect(h: &egui_kittest::Harness<'static, PhotocraftApp>) -> [f32; 4] {
        h.state().ui.windows.iter().find(|w| w.open).and_then(|w| w.rect).expect("the window stores its rectangle")
    }

    fn click(h: &mut egui_kittest::Harness<'static, PhotocraftApp>, at: egui::Pos2) {
        h.hover_at(at);
        h.run_steps(1);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() });
            h.run_steps(1);
        }
        h.run_steps(1);
    }

    #[test]
    fn a_workspace_window_is_drawn_and_its_close_button_closes_the_document() {
        let mut h = harness(app_with_a_floating_window());
        let r = window_rect(&h);
        assert!(r[2] >= MIN_W && r[3] >= MIN_H, "{r:?}");
        // The × closes the document itself (Photoshop), not back into the tab strip.
        click(&mut h, pos2(r[0] + r[2] - 22.0, r[1] + TITLE_H / 2.0));
        assert!(h.state().session.documents().is_empty(), "a clean document closes without a dialog");
        assert!(h.state().ui.windows.is_empty(), "its window is gone with it");
        assert!(super::super::tabs::docked_document_indices(h.state()).is_empty(), "it does not dock back");
    }

    #[test]
    fn dragging_the_title_bar_moves_the_window() {
        let mut h = harness(app_with_a_floating_window());
        let before = window_rect(&h);
        let from = pos2(before[0] + 60.0, before[1] + TITLE_H / 2.0);
        h.hover_at(from);
        h.run_steps(1);
        h.event(egui::Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
        h.run_steps(1);
        let to = pos2(from.x + 40.0, from.y + 25.0);
        h.event(egui::Event::PointerMoved(to));
        h.run_steps(1);
        h.event(egui::Event::PointerMoved(to));
        h.run_steps(1);
        h.event(egui::Event::PointerButton { pos: to, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
        h.run_steps(1);
        let after = window_rect(&h);
        assert!((after[0] - before[0] - 40.0).abs() < 3.0, "{before:?} -> {after:?}");
        assert!((after[1] - before[1] - 25.0).abs() < 3.0, "{before:?} -> {after:?}");
    }
}
