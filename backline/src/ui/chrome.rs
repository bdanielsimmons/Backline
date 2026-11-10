//! The window's own chrome when a skin is worn on Windows (no system title bar): the strips that move it
//! (`app_bar_strip`), Now Playing's bar and the visuals' fading one (`window_bar`), the edges that size it, and the
//! drag handles between the docked windows (`splitter`).

use super::*;

/// A tour is running (set every frame): the windows can't be resized under it.
pub(crate) static TOUR_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The skin's own bars are the window's title bar (no system one): they move it, double-click maximises, its edges
/// size it, the deck's buttons minimise and close it.
pub(crate) static APP_BAR: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) fn app_bar() -> bool {
    APP_BAR.load(std::sync::atomic::Ordering::Relaxed)
}

/// A window's title strip, when it is the app's title bar: dragged, the window moves (Windows snaps it to the
/// screen's edges as usual); double-clicked, it fills the screen or comes back.
pub(crate) fn app_bar_strip(ui: &egui::Ui, strip: Rect, id: egui::Id) {
    if !app_bar() {
        return;
    }
    let r = ui.interact(strip, id, Sense::click_and_drag());
    if r.double_clicked() {
        let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
    } else if r.drag_started() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
}

/// Now Playing's own bar along the top (a skin worn, Windows: there is no system title bar), always there: the
/// window's minimise, maximise and close at its right, the rest of it moves the window. The page sits under it.
pub(crate) const NP_BAR_H: f32 = 34.0;

/// The full-window visuals fill the window, with no system title bar: a slim bar comes in along the top when the mouse
/// moves and goes after a few still seconds, as a video player's does (Now Playing keeps its bar: `window_bar`).
pub(crate) fn floating_app_bar(ctx: &egui::Context) {
    let screen = ctx.screen_rect();
    let (still, pos) = ctx.input(|i| (i.pointer.time_since_last_movement(), i.pointer.hover_pos()));
    let near = pos.is_some_and(|p| p.y < screen.min.y + 56.0);
    // (a test copy can hold it up for its picture: BACKLINE_FLOAT_BAR)
    let show = near || still < 2.5 || std::env::var_os("BACKLINE_FLOAT_BAR").is_some();
    let a = ctx.animate_bool_with_time(egui::Id::new("float_app_bar"), show, 0.25);
    if still < 2.6 {
        ctx.request_repaint_after(Duration::from_millis(300));
    }
    if a <= 0.0 {
        return;
    }
    let bar = Rect::from_min_size(screen.min, Vec2::new(screen.width(), 36.0));
    egui::Area::new(egui::Id::new("float_app_bar_area")).order(egui::Order::Foreground).fixed_pos(bar.min).show(ctx, |ui| {
        window_bar(ui, bar, a, 150);
        ui.allocate_rect(bar, Sense::hover());
    });
}

/// A window bar: a soft shade from the top (`shade`: how dark at the very top) so the buttons read over any picture;
/// minimise, maximise / restore and close at the right, as Windows has them; the rest moves the window, double-click
/// fills the screen or brings it back. `a`: how far faded in.
pub(crate) fn window_bar(ui: &mut egui::Ui, bar: Rect, a: f32, shade: u8) {
    let h = bar.height();
    {
        let p = ui.painter().clone();
        let mut mesh = egui::Mesh::default();
        let (top, bottom) = (Color32::from_black_alpha((shade as f32 * a) as u8), Color32::from_black_alpha(0));
        mesh.colored_vertex(bar.left_top(), top);
        mesh.colored_vertex(bar.right_top(), top);
        mesh.colored_vertex(bar.right_bottom(), bottom);
        mesh.colored_vertex(bar.left_bottom(), bottom);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        p.add(egui::Shape::mesh(mesh));
    }
    {
        let p = ui.painter().clone();
        // the buttons first (they win over the dragging strip under them)
        let bw = 46.0;
        let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
        let ink = Color32::from_white_alpha((235.0 * a) as u8);
        let mut over_button = false;
        for (k, tip) in [(0, "Close"), (1, if max { "Restore" } else { "Maximize" }), (2, "Minimize")] {
            let r = Rect::from_min_size(Pos2::new(bar.max.x - bw * (k as f32 + 1.0), bar.min.y), Vec2::new(bw, h));
            let b = ui.interact(r, egui::Id::new(("float_app_bar_btn", k)), Sense::click()).tip(tip);
            over_button |= b.hovered();
            if b.hovered() {
                p.rect_filled(r, 0.0, if k == 0 { Color32::from_rgb(196, 43, 28) } else { Color32::from_white_alpha(28) });
            }
            let c = r.center();
            let s = Stroke::new(1.2_f32, ink);
            match k {
                0 => {
                    p.line_segment([c + Vec2::new(-5.0, -5.0), c + Vec2::new(5.0, 5.0)], s);
                    p.line_segment([c + Vec2::new(-5.0, 5.0), c + Vec2::new(5.0, -5.0)], s);
                }
                1 if max => {
                    p.rect_stroke(Rect::from_center_size(c + Vec2::new(-1.5, 1.5), Vec2::splat(8.0)), 1.0, s);
                    p.line_segment([c + Vec2::new(-2.5, -4.5), c + Vec2::new(5.5, -4.5)], s);
                    p.line_segment([c + Vec2::new(5.5, -4.5), c + Vec2::new(5.5, 3.5)], s);
                }
                1 => {
                    p.rect_stroke(Rect::from_center_size(c, Vec2::splat(10.0)), 1.0, s);
                }
                _ => {
                    p.line_segment([c + Vec2::new(-5.0, 0.0), c + Vec2::new(5.0, 0.0)], s);
                }
            }
            if b.clicked() {
                ui.ctx().send_viewport_cmd(match k {
                    0 => egui::ViewportCommand::Close,
                    1 => egui::ViewportCommand::Maximized(!max),
                    _ => egui::ViewportCommand::Minimized(true),
                });
            }
        }
        let drag = Rect::from_min_max(bar.min, Pos2::new(bar.max.x - 3.0 * bw, bar.max.y));
        if !over_button {
            let r = ui.interact(drag, egui::Id::new("float_app_bar_drag"), Sense::click_and_drag());
            if r.double_clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
            } else if r.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
        }
    }
}

/// With no system frame: the window's edges and corners size it, as a framed window's do (the pointer turns at
/// them). Not while it fills the screen.
pub(crate) fn edge_resize(ctx: &egui::Context) {
    use egui::viewport::ResizeDirection as D;
    let (max, full) = ctx.input(|i| (i.viewport().maximized.unwrap_or(false), i.viewport().fullscreen.unwrap_or(false)));
    if max || full {
        return;
    }
    let r = ctx.screen_rect();
    let Some(p) = ctx.input(|i| i.pointer.hover_pos()) else { return };
    let m = 5.0;
    let (l, rt, t, b) = (p.x - r.min.x < m, r.max.x - p.x < m, p.y - r.min.y < m, r.max.y - p.y < m);
    let dir = match (l, rt, t, b) {
        (true, _, true, _) => D::NorthWest,
        (_, true, true, _) => D::NorthEast,
        (true, _, _, true) => D::SouthWest,
        (_, true, _, true) => D::SouthEast,
        (true, _, _, _) => D::West,
        (_, true, _, _) => D::East,
        (_, _, true, _) => D::North,
        (_, _, _, true) => D::South,
        _ => return,
    };
    ctx.set_cursor_icon(match dir {
        D::North | D::South => egui::CursorIcon::ResizeVertical,
        D::East | D::West => egui::CursorIcon::ResizeHorizontal,
        D::NorthWest | D::SouthEast => egui::CursorIcon::ResizeNwSe,
        _ => egui::CursorIcon::ResizeNeSw,
    });
    if ctx.input(|i| i.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(dir));
    }
}

/// A drag handle drawn in the gap between two windows. Some(Some(pos)) while dragging, Some(None) on double click.
pub(crate) fn splitter(ctx: &egui::Context, id: &str, rect: Rect, vertical: bool) -> Option<Option<f32>> {
    // the windows keep their sizes while a tour is showing them
    if TOUR_ON.load(std::sync::atomic::Ordering::Relaxed) {
        return None;
    }
    let mut out = None;
    egui::Area::new(egui::Id::new(id)).order(egui::Order::Foreground).fixed_pos(rect.min).show(ctx, |ui| {
        let (r, resp) = ui.allocate_exact_size(rect.size(), Sense::click_and_drag());
        let hot = resp.hovered() || resp.dragged();
        let p = ui.painter();
        let t = thick(2.0);
        let (line, grip) = if vertical {
            (Rect::from_center_size(r.center(), Vec2::new(t * 2.0, r.height())), Vec2::new(t * 3.0, t * 6.0))
        } else {
            (Rect::from_center_size(r.center(), Vec2::new(r.width(), t * 2.0)), Vec2::new(t * 6.0, t * 3.0))
        };
        if hot {
            fill_rect(p, line, pal().trim);
            let g = Rect::from_center_size(r.center(), grip + if vertical { Vec2::new(t, 0.0) } else { Vec2::new(0.0, t) });
            fill_rect(p, g.expand(t), Color32::BLACK);
            fill_rect(p, g, pal().beige_lt);
            let step = if vertical { Vec2::new(0.0, t * 2.0) } else { Vec2::new(t * 2.0, 0.0) };
            for k in [-1.0f32, 0.0, 1.0] {
                let d = Rect::from_center_size(r.center() + step * k, Vec2::splat(t));
                fill_rect(p, d, pal().edge);
            }
        }
        if resp.dragged() {
            if let Some(pp) = resp.interact_pointer_pos() {
                out = Some(Some(if vertical { pp.x } else { pp.y }));
            }
        }
        if resp.double_clicked() {
            out = Some(None);
        }
        if hot {
            ui.ctx().set_cursor_icon(if vertical {
                egui::CursorIcon::ResizeHorizontal
            } else {
                egui::CursorIcon::ResizeVertical
            });
        }
    });
    out
}
