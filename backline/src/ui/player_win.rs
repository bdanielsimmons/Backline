//! The player window (the skinned deck with transport, seek and volume) and the queue under it.

use super::*;

impl App {
    pub(crate) fn player_window(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        // the deck, drawn from the skin worn (Backline's own looks are skins too: one is always on)
        if self.wa_art.is_some() {
            let inner = ui.max_rect();
            self.winamp_main(ui, inner, acts);
        }
    }

    pub(crate) fn playlist_ui(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        // a worn skin draws the queue as its own playlist window
        if !self.chart_in_queue() && self.wa_art.as_ref().is_some_and(|t| t.sheets.contains_key("pledit")) {
            let inner = ui.max_rect();
            let outer = crate::frame_outer(inner);
            crate::tour::note("QUEUE", outer);
            self.winamp_playlist(ui, outer, acts);
            return;
        }
        // the lead sheet in the queue's place (focus mode's LEAD SHEET tab)
        let inner = ui.max_rect();
        window_deco(ui, inner, "LEAD SHEET");
        // nothing in the window (the lead sheet's mixer at its foot) spills over the window under it
        let clip = crate::frame_outer(inner).intersect(ui.clip_rect());
        ui.set_clip_rect(Rect::from_min_max(clip.min, Pos2::new(clip.max.x, inner.max.y + 6.0)));
        // the page, clear of the frame all round (the panel's Ui keeps the title strip's chips)
        let mut page = crate::page_ui(ui, inner);
        self.right_header(ui, &mut page, acts);
    }
}

/// In a queue's right-click menu: move a song up to play next, or behind the songs you queued (not the one playing,
/// nor ones already played).
pub(crate) fn queue_moves(ui: &mut egui::Ui, acts: &mut Vec<Action>, i: usize, cur: Option<usize>) {
    if cur.is_some_and(|c| i > c) {
        if menu_item(ui, "Move: Play Next") {
            acts.push(Action::QueueNext(i));
            ui.close_menu();
        }
        if menu_item(ui, "Move: After Queued Songs") {
            acts.push(Action::QueueLater(i));
            ui.close_menu();
        }
    }
    if let Some(c) = cur.filter(|c| i != *c) {
        if menu_item(ui, "Move: Above Current Song") {
            acts.push(Action::MoveQueue(i, c));
            ui.close_menu();
        }
    }
}

impl App {
    /// A song you queued yourself (Play next / Add to queue), waiting to play.
    pub(crate) fn queued(&self, i: usize) -> bool {
        self.cur.is_some_and(|c| i > c && i <= c + self.q_manual)
    }
}
