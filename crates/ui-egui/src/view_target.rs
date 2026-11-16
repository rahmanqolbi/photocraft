//! Floating document windows: which view the view commands act on, and how the window layout
//! survives a relaunch.
//!
//! View commands (Zoom, Print Size, Fit…) act on the active document's view. A document that
//! floats lives in its floating window (`Window > Arrange`), so the command edits that window's
//! view — the one on screen — not the docked view the document would return to.
//!
//! The layout (which files float, where, at what zoom/rotation) is remembered per file in
//! Preferences and reapplied by [`restore_session`]. Opens in the background, so a restored file
//! is floated as soon as its document lands, not when it is asked for.

use crate::PhotocraftApp;
use crate::state::View;
use photocraft_engine::prefs::SavedWindow;

/// The view the view commands act on: the active document's floating window when it has one
/// open, else the main (docked) view.
pub(crate) fn view_mut(app: &mut PhotocraftApp) -> Option<&mut View> {
    let i = app.session.active_index()?;
    if let Some(w) = app.ui.windows.iter_mut().find(|w| w.open && w.floating && w.document == i) {
        return Some(&mut w.view);
    }
    app.ui.views.get_mut(i)
}

/// The view the view commands act on, read-only.
pub(crate) fn view_ref(app: &PhotocraftApp) -> Option<&View> {
    let i = app.session.active_index()?;
    if let Some(w) = app.ui.windows.iter().find(|w| w.open && w.floating && w.document == i) {
        return Some(&w.view);
    }
    app.ui.views.get(i)
}

/// Persist the floating windows into prefs. Called every frame; writes only when something
/// changed (cheap snapshot compare).
pub(crate) fn sync_session(app: &mut PhotocraftApp) {
    let windows: Vec<SavedWindow> = app
        .ui
        .windows
        .iter()
        .filter(|w| w.open && w.floating)
        .filter_map(|w| {
            let path = app.session.documents().get(w.document)?.path.clone()?;
            Some(SavedWindow {
                path,
                zoom: w.view.zoom,
                center: w.view.center,
                rotation: w.view.rotation,
                position: w.position,
                rect: w.rect,
                maximized: w.maximized,
                minimized: w.minimized,
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

/// Restore the floating windows recorded at the last quit. Called once at startup, before the
/// first frame; files that are not open yet open in the background and are floated by
/// [`restore_tick`] as they land. A file passed on the command line that is also in the saved
/// layout is not opened twice.
pub fn restore_session(app: &mut PhotocraftApp) {
    if app.ui.session_restored {
        return;
    }
    app.ui.session_restored = true;
    let saved = app.session.prefs().file_handling.saved_windows.clone();
    if saved.is_empty() {
        return;
    }
    let present: Vec<String> = app.session.documents().iter().filter_map(|d| d.path.clone()).collect();
    let missing: Vec<String> = saved.iter().map(|g| g.path.clone()).filter(|p| !present.contains(p)).collect();
    if !missing.is_empty() {
        app.open_paths(&missing);
    }
    app.ui.session_restore = Some(saved);
    apply_ready(app);
}

/// Float every pending window whose document has landed. Called every frame while a restore is
/// in flight; cheap no-op once the layout is applied.
pub(crate) fn restore_tick(app: &mut PhotocraftApp) {
    if app.ui.session_restore.is_some() {
        apply_ready(app);
    }
}

fn apply_ready(app: &mut PhotocraftApp) {
    let Some(pending) = app.ui.session_restore.take() else {
        return;
    };
    let mut remaining = Vec::new();
    for saved in pending {
        match app.session.documents().iter().position(|d| d.path.as_deref() == Some(saved.path.as_str())) {
            Some(idx) => float_restored(app, idx, &saved),
            None => remaining.push(saved),
        }
    }
    app.ui.session_restore = (!remaining.is_empty()).then_some(remaining);
}

/// Float document `idx` with the saved view and window geometry.
fn float_restored(app: &mut PhotocraftApp, idx: usize, saved: &SavedWindow) {
    // Skip if the document already floats (e.g. the user floated it before the open landed).
    if app.ui.windows.iter().any(|w| w.open && w.floating && w.document == idx) {
        return;
    }
    let wid = crate::view_cmds::float_window(app, idx, 0, true);
    if let Some(w) = app.ui.windows.iter_mut().find(|w| w.id == wid) {
        w.view.zoom = saved.zoom;
        w.view.center = saved.center;
        w.view.rotation = saved.rotation;
        w.view.fit_pending = false;
        w.view.zoom_anchor = None;
        w.position = saved.position;
        w.rect = saved.rect;
        w.maximized = saved.maximized;
        w.minimized = saved.minimized;
    }
    if let Some(v) = app.ui.views.get_mut(idx) {
        v.zoom = saved.zoom;
        v.center = saved.center;
        v.rotation = saved.rotation;
        v.fit_pending = false;
        v.zoom_anchor = None;
    }
}
