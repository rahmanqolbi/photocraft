//! View commands apply to the window that most recently had focus: the floating window
//! showing the document when one is focused, else the main per-document view in `ui.views`.
//! Photoshop routes them to the front document window the same way.

use crate::PhotocraftApp;
use crate::state::View;

/// Record which of the windows owns the focus now. Every viewport calls this each frame with
/// its own `ViewportInfo::focused`; whichever window has OS focus wins the commands.
pub(crate) fn set_focused(app: &mut PhotocraftApp, id: Option<u64>) {
    if id != app.ui.focused_window {
        app.ui.focused_window = id;
    }
}

/// The view view-commands act on: the focused floating window's view, else the main view
/// of the active document.
pub(crate) fn view_mut(app: &mut PhotocraftApp) -> Option<&mut View> {
    if let Some(id) = app.ui.focused_window {
        if let Some(w) = app.ui.windows.iter_mut().find(|w| w.open && w.id == id) {
            return Some(&mut w.view);
        }
    }
    let i = app.session.active_index()?;
    app.ui.views.get_mut(i)
}

/// The view view-commands act on, read-only.
pub(crate) fn view_ref(app: &PhotocraftApp) -> Option<&View> {
    if let Some(id) = app.ui.focused_window {
        if let Some(w) = app.ui.windows.iter().find(|w| w.open && w.id == id) {
            return Some(&w.view);
        }
    }
    let i = app.session.active_index()?;
    app.ui.views.get(i)
}

/// The canvas area (in egui points) of the window the view commands act on: the focused
/// floating window's last canvas rect, else the main document area's.
pub(crate) fn canvas_rect(app: &PhotocraftApp) -> Option<egui::Rect> {
    if let Some(id) = app.ui.focused_window {
        if let Some(w) = app.ui.windows.iter().find(|w| w.open && w.id == id) {
            return w.last_canvas.map(|[x0, y0, x1, y1]| egui::Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x1, y1)));
        }
    }
    let r = app.last_canvas_rect;
    (r.width() > 0.0 && r.height() > 0.0).then_some(r)
}

/// Persist the floating windows' layout into prefs. Called from the frame loop; only writes
/// when something about the windows changed (cheap string compare).
pub(crate) fn sync_session(app: &mut PhotocraftApp) {
    let windows: Vec<photocraft_engine::prefs::SavedWindow> = app
        .ui
        .windows
        .iter()
        .filter_map(|w| {
            let path = app.session.documents().get(w.document)?.path.clone()?;
            Some(photocraft_engine::prefs::SavedWindow {
                path,
                zoom: w.view.zoom,
                center: w.view.center,
                rotation: w.view.rotation,
                position: w.position,
            })
        })
        .collect();
    let text = serde_json::to_string(&windows).unwrap_or_default();
    if text == app.ui.session_snapshot {
        return;
    }
    app.ui.session_snapshot = text;
    app.session.prefs.edit(|p| p.file_handling.saved_windows = windows);
}

/// Restore the floating windows recorded at the last quit. Called once, after the command line
/// had its chance to open documents: the command line (or a crash-recovery launch) wins.
pub fn restore_session(app: &mut PhotocraftApp) {
    if app.ui.session_restored {
        return;
    }
    app.ui.session_restored = true;
    if !app.session.documents().is_empty() {
        return;
    }
    let geom = app.session.prefs().file_handling.saved_windows.clone();
    for g in geom {
        let n = app.open_paths(&[g.path.clone()]);
        if n == 1 {
            let idx = app.session.documents().len().saturating_sub(1);
            if let Some(v) = app.ui.views.get_mut(idx) {
                v.zoom = g.zoom;
                v.center = g.center;
                v.rotation = g.rotation;
                v.fit_pending = false;
                v.zoom_anchor = None;
            }
            let wid = crate::menus::view_cmds::arrange::float_window(app, idx, 0);
            if let Some(w) = app.ui.windows.iter_mut().find(|w| w.id == wid) {
                w.view.zoom = g.zoom;
                w.view.center = g.center;
                w.view.rotation = g.rotation;
                w.view.fit_pending = false;
                w.view.zoom_anchor = None;
                w.position = g.position;
            }
        }
    }
}
