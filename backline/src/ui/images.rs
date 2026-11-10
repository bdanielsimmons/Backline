//! Pictures: covers and art from the web, kept on disk and loaded in the background (`Images`), and the ways they
//! are drawn (a card that tilts, a record, the page's art).

use super::*;

// -------------------------------------------------------------------- images
pub(crate) struct Images {
    /// texture (None = failed to load) and when it was last drawn
    pub(crate) map: HashMap<String, (Option<egui::TextureHandle>, Instant)>,
    pub(crate) requested: HashSet<String>,
    pub(crate) req: Sender<String>,
    /// the rows' covers ("thumb:...") waiting to load
    pub(crate) thumbs: Thumbs,
    /// average colour of the top and bottom half of each cover, for the art-view gradient
    pub(crate) avg: HashMap<String, [Color32; 2]>,
    /// per place a cover is shown (album view, player): the cover on show, the one it is fading from, and when
    /// the new one arrived. The old cover stays up while the next loads, so a track change never flashes.
    pub(crate) hold: HashMap<&'static str, (Option<egui::TextureId>, Option<egui::TextureId>, f64)>,
}

impl Images {
    /// A loaded picture's size in pixels.
    pub(crate) fn size(&self, url: &str) -> Option<[usize; 2]> {
        self.map.get(url).and_then(|(t, _)| t.as_ref()).map(|h| h.size())
    }

    pub(crate) fn get(&mut self, url: &str) -> Option<egui::TextureId> {
        if url.is_empty() {
            return None;
        }
        if let Some((tex, used)) = self.map.get_mut(url) {
            *used = Instant::now();
            return tex.as_ref().map(|h| h.id());
        }
        if self.requested.insert(url.to_string()) {
            if url.starts_with("thumb:") {
                let (m, cv) = &*self.thumbs;
                m.lock().unwrap_or_else(|e| e.into_inner()).push(url.to_string());
                cv.notify_one();
            } else {
                let _ = self.req.send(url.to_string());
            }
        }
        None
    }

    /// The cover to show at `slot` for `url`: (now showing, fading from, how far the fade is 0..1).
    /// While the new cover loads, the previous one stays; when it arrives it fades in over the old.
    pub(crate) fn held(
        &mut self,
        slot: &'static str,
        url: &str,
        gray: bool,
        now: f64,
    ) -> (Option<egui::TextureId>, Option<egui::TextureId>, f32) {
        let new = art_tex(self, url, gray);
        let gone = url.is_empty() || self.failed(url);
        let e = self.hold.entry(slot).or_insert((None, None, 0.0));
        match new {
            Some(n) if e.0 != Some(n) => *e = (Some(n), e.0, now),
            None if gone => *e = (None, None, 0.0),
            _ => {}
        }
        let f = ((now - e.2) / 0.35).clamp(0.0, 1.0) as f32;
        if f >= 1.0 {
            e.1 = None;
        }
        (e.0, e.1, f)
    }

    /// Back online: pictures that failed while there was no internet are tried again when next drawn.
    pub(crate) fn retry_failed(&mut self) {
        let failed: Vec<String> = self.map.iter().filter(|(_, (t, _))| t.is_none()).map(|(u, _)| u.clone()).collect();
        for u in failed {
            self.map.remove(&u);
            self.requested.remove(&u);
        }
        self.hold.clear();
    }

    /// Tried and failed (or no picture at all): nothing is coming.
    pub(crate) fn failed(&self, url: &str) -> bool {
        matches!(self.map.get(url), Some((None, _)))
    }

    /// Covers not drawn for two minutes are let go (a big one is ~1.6 MB), so long browsing doesn't keep growing.
    /// They load again if they come back on screen.
    pub(crate) fn trim(&mut self) {
        if self.map.len() < 150 {
            return;
        }
        let old: Vec<String> =
            self.map.iter().filter(|(_, (_, t))| t.elapsed() > Duration::from_secs(120)).map(|(u, _)| u.clone()).collect();
        for u in old {
            self.map.remove(&u);
            self.requested.remove(&u);
        }
    }
}

/// Where a web picture is kept on disk: a name made from its address (FNV-1a, the same every run).
pub(crate) fn cover_path(url: &str) -> std::path::PathBuf {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in url.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    api::config_dir().join("covers").join(format!("{:016x}", h))
}

/// A picture, from its address: "thumb:" a row's cover (small and square), "gray:" in black and white, "file:" from a
/// song file (or its folder), "img:" a file on this computer, else the web (kept on disk once fetched, so covers show
/// offline too).
pub(crate) fn picture(api: &Api, url: &str) -> Option<egui::ColorImage> {
    let (thumb, url) = match url.strip_prefix("thumb:") {
        Some(u) => (true, u),
        None => (false, url),
    };
    let (gray, real) = match url.strip_prefix("gray:") {
        Some(u) => (true, u),
        None => (false, url),
    };
    let bytes = match real.strip_prefix("file:") {
        Some(path) => sources::file_art(std::path::Path::new(path)),
        None if real.starts_with("img:") => std::fs::read(&real[4..]).ok(),
        None => {
            let kept = cover_path(real);
            match std::fs::read(&kept).ok().filter(|b| b.len() > 100) {
                Some(b) => Some(b),
                None => {
                    let b = api.fetch(real).ok();
                    if let Some(b) = &b {
                        if let Some(d) = kept.parent() {
                            let _ = std::fs::create_dir_all(d);
                        }
                        let _ = std::fs::write(&kept, b);
                    }
                    b
                }
            }
        }
    };
    bytes.and_then(|b| image::load_from_memory(&b).ok()).map(|im| {
        let im = if thumb {
            // a row's cover: square and small. A video's picture has black bands above and below its 16:9 frame:
            // those go first, then the middle of it is taken
            let im = if real.contains("ytimg.com") && im.height() * 4 == im.width() * 3 {
                let band = im.width() * 9 / 16;
                im.crop_imm(0, (im.height() - band) / 2, im.width(), band)
            } else {
                im
            };
            let s = im.width().min(im.height());
            im.crop_imm((im.width() - s) / 2, (im.height() - s) / 2, s, s).thumbnail(96, 96)
        } else if im.width().max(im.height()) > 800 {
            // covers are only ever drawn up to a few hundred pixels: kept small, not at the file's full size
            im.thumbnail(800, 800)
        } else {
            im
        };
        let im = if gray { im.grayscale() } else { im };
        let rgba = im.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw())
    })
}

pub(crate) fn spawn_image_pool(api: Api, tx: Sender<Msg>, ctx: egui::Context) -> Sender<String> {
    let (req_tx, req_rx) = channel::<String>();
    let req_rx = Arc::new(Mutex::new(req_rx));
    for _ in 0..4 {
        let rx = req_rx.clone();
        let tx = tx.clone();
        let api = api.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || loop {
            let url = match rx.lock().unwrap().recv() {
                Ok(u) => u,
                Err(_) => break,
            };
            let img = picture(&api, &url);
            let _ = tx.send(Msg::Img(url, img));
            ctx.request_repaint();
        });
    }
    req_tx
}

/// The rows' covers waiting to load: the last asked for first (where the list is now, not where it was scrolled
/// past), on two threads of their own so the big covers never wait behind them.
pub(crate) type Thumbs = Arc<(Mutex<Vec<String>>, std::sync::Condvar)>;

pub(crate) fn spawn_thumb_pool(api: Api, tx: Sender<Msg>, ctx: egui::Context) -> Thumbs {
    let q: Thumbs = Arc::new((Mutex::new(Vec::new()), std::sync::Condvar::new()));
    for _ in 0..2 {
        let (q, tx, api, ctx) = (q.clone(), tx.clone(), api.clone(), ctx.clone());
        std::thread::spawn(move || loop {
            let url = {
                let (m, cv) = &*q;
                let mut v = m.lock().unwrap_or_else(|e| e.into_inner());
                loop {
                    match v.pop() {
                        Some(u) => break u,
                        None => v = cv.wait(v).unwrap_or_else(|e| e.into_inner()),
                    }
                }
            };
            let img = picture(&api, &url);
            if tx.send(Msg::Img(url, img)).is_err() {
                break;
            }
            ctx.request_repaint();
        });
    }
    q
}

/// Texture for a cover, optionally the black-and-white version (falls back to colour while it loads).
pub(crate) fn art_tex(images: &mut Images, url: &str, gray: bool) -> Option<egui::TextureId> {
    // (the Greyscale look: every cover in grey)
    if gray || crate::skin::mono() {
        if let Some(id) = images.get(&gray_url(url)) {
            return Some(id);
        }
    }
    images.get(url)
}

/// A picture's address in grey ("thumb:gray:..." for a row's small cover, as `picture` reads them).
pub(crate) fn gray_url(url: &str) -> String {
    match url.strip_prefix("thumb:") {
        Some(rest) => format!("thumb:gray:{}", rest),
        None => format!("gray:{}", url),
    }
}

pub(crate) fn paint_art_g(ui: &egui::Ui, images: &mut Images, url: &str, rect: Rect, round: f32, gray: bool) {
    let round = round.min(0.0);
    ui.painter().rect_filled(rect, Rounding::same(round), pal().ink2);
    if let Some(id) = art_tex(images, url, gray) {
        egui::Image::new(egui::load::SizedTexture::new(id, rect.size())).rounding(Rounding::same(round)).paint_at(ui, rect);
    } else {
        // no cover (yet): the record, turning while it loads
        let loading = !url.is_empty() && !images.failed(url);
        paint_record(ui.painter(), rect, loading.then(|| ui.input(|i| i.time) as f32));
        if loading {
            ui.ctx().request_repaint();
        }
    }
}

/// The stand-in for a missing picture: a quiet square with the Backline gem, or (`spin` = seconds) a small
/// turning arc while it loads.
pub(crate) fn paint_record(p: &egui::Painter, rect: Rect, spin: Option<f32>) {
    let c = rect.center();
    let s = rect.width().min(rect.height());
    p.rect_filled(Rect::from_center_size(c, Vec2::splat(s)), 0.0, Color32::from_black_alpha(60));
    let col = pal().ink2.gamma_multiply(0.7);
    match spin {
        Some(t) => {
            // three quarters of a ring, turning
            let r = s * 0.12;
            let pts: Vec<Pos2> = (0..=24).map(|k| c + Vec2::angled(t * 4.0 + k as f32 / 24.0 * 4.7) * r).collect();
            p.add(egui::Shape::line(pts, Stroke::new((s * 0.025).max(2.0), col)));
        }
        None => {
            // the Backline gem, quietly
            let g = Rect::from_center_size(c, Vec2::splat(s * 0.36));
            vicon::gem(p, g, pal().ink2.gamma_multiply(0.75), pal().ink2.gamma_multiply(0.35));
        }
    }
}

/// Covers shown as records instead of squares (click a cover to switch). Saved in the settings.
pub(crate) static VINYL: AtomicBool = AtomicBool::new(false);

/// Covers beside the songs in track lists (the switch at the left of their header). Saved in the settings.
pub(crate) static ROW_ARTS: AtomicBool = AtomicBool::new(true);

pub(crate) fn row_arts_on() -> bool {
    ROW_ARTS.load(Ordering::Relaxed)
}

/// How far the record has turned: it spins up to 33 1/3 rpm times the playback speed (`speed`, 0 = stopped)
/// and coasts to a stop on pause.
/// Seconds since 1970 (for times kept across restarts).
pub(crate) fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

pub(crate) fn vinyl_angle(ui: &egui::Ui, speed: f32) -> f32 {
    let id = egui::Id::new("vinyl_angle");
    let dt = ui.input(|i| i.stable_dt).min(0.1);
    let (ang, vel): (f32, f32) = ui.ctx().data(|d| d.get_temp(id)).unwrap_or((0.0, 0.0));
    let target = if skin::calm() { 0.0 } else { 3.49 * speed }; // radians a second at 33 1/3 rpm
    let vel = vel + (target - vel) * (1.0 - (-dt * 2.0).exp());
    let ang = (ang + vel * dt) % std::f32::consts::TAU;
    ui.ctx().data_mut(|d| d.insert_temp(id, (ang, vel)));
    if vel > 0.005 {
        ui.ctx().request_repaint();
    }
    ang
}

/// A record whose label is the cover, turned by `angle`. The light on the grooves stays put, like a real one.
/// `alpha` below 1 lets what is behind it (the visualizer) show through.
pub(crate) fn paint_vinyl(p: &egui::Painter, rect: Rect, tex: Option<egui::TextureId>, angle: f32, alpha: f32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.5;
    let k = |col: Color32| col.gamma_multiply(alpha);
    p.circle_filled(c, r, k(Color32::from_gray(14)));
    for g in 0..9 {
        let shade = Color32::from_gray(if g % 3 == 0 { 44 } else { 30 });
        p.circle_stroke(c, r * (0.47 + g as f32 * 0.058), Stroke::new(1.0_f32, k(shade)));
    }
    // the light: two soft wedges, fixed
    for (a0, a1) in [(-2.4f32, -1.9f32), (0.75, 1.25)] {
        let mut pts = vec![];
        for s in 0..=8 {
            pts.push(c + Vec2::angled(a0 + (a1 - a0) * s as f32 / 8.0) * r * 0.97);
        }
        for s in (0..=8).rev() {
            pts.push(c + Vec2::angled(a0 + (a1 - a0) * s as f32 / 8.0) * r * 0.46);
        }
        p.add(egui::Shape::Path(egui::epaint::PathShape::convex_polygon(pts, k(Color32::from_white_alpha(10)), Stroke::NONE)));
    }
    // the label: the cover cut to a circle, turning
    let lr = r * 0.42;
    match tex {
        Some(id) => {
            let tint = k(Color32::WHITE);
            let mut mesh = egui::Mesh::with_texture(id);
            mesh.vertices.push(egui::epaint::Vertex { pos: c, uv: Pos2::new(0.5, 0.5), color: tint });
            let n = 64;
            for s in 0..=n {
                let a = s as f32 / n as f32 * std::f32::consts::TAU;
                let uv = Vec2::angled(a - angle) * 0.5;
                let pos = c + Vec2::angled(a) * lr;
                mesh.vertices.push(egui::epaint::Vertex { pos, uv: Pos2::new(0.5 + uv.x, 0.5 + uv.y), color: tint });
                if s > 0 {
                    mesh.indices.extend_from_slice(&[0, s as u32, s as u32 + 1]);
                }
            }
            p.add(egui::Shape::mesh(mesh));
        }
        None => {
            p.circle_filled(c, lr, k(pal().red));
            let mark = [c + Vec2::angled(angle) * lr * 0.3, c + Vec2::angled(angle) * lr * 0.85];
            p.line_segment(mark, Stroke::new(2.0_f32, k(Color32::from_white_alpha(120))));
        }
    }
    p.circle_stroke(c, lr, Stroke::new(1.5_f32, k(Color32::from_black_alpha(160))));
    p.circle_filled(c, (r * 0.035).max(2.0), Color32::from_gray(8));
}

/// A page's own picture, round for an artist; a spinning record while it loads, a still one when there is none.
pub(crate) fn paint_page_art(ui: &egui::Ui, images: &mut Images, url: &str, rect: Rect, artist: bool) {
    match images.get(url) {
        Some(id) => {
            let round = if artist { rect.width() * 0.5 } else { 0.0 };
            egui::Image::new(egui::load::SizedTexture::new(id, rect.size())).rounding(Rounding::same(round)).paint_at(ui, rect);
            if artist {
                ui.painter().circle_stroke(rect.center(), rect.width() * 0.5, Stroke::new(2.0_f32, pal().accent));
            }
        }
        None if url.is_empty() || images.failed(url) => paint_record(ui.painter(), rect, None),
        None => {
            paint_record(ui.painter(), rect, Some(ui.input(|i| i.time) as f32));
            ui.ctx().request_repaint();
        }
    }
}

/// A cover drawn into four corners at `alpha`, with a hair cropped off each edge (where JPEG junk lives).
pub(crate) fn cover_quad(p: &egui::Painter, cs: [Pos2; 4], id: egui::TextureId, alpha: f32) {
    const E: f32 = 0.006;
    let uvs = [Pos2::new(E, E), Pos2::new(1.0 - E, E), Pos2::new(1.0 - E, 1.0 - E), Pos2::new(E, 1.0 - E)];
    let color = Color32::from_white_alpha((alpha.clamp(0.0, 1.0) * 255.0) as u8);
    let mut mesh = egui::Mesh::with_texture(id);
    for (pos, uv) in cs.iter().zip(uvs.iter()) {
        mesh.vertices.push(egui::epaint::Vertex { pos: *pos, uv: *uv, color });
    }
    mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
    p.add(egui::Shape::mesh(mesh));
}

/// A cover that changes smoothly: the old one stays until the new one is in, then the new fades in over it.
pub(crate) fn paint_held(ui: &egui::Ui, images: &mut Images, slot: &'static str, url: &str, rect: Rect, gray: bool) {
    let (tex, prev, f) = images.held(slot, url, gray, ui.input(|i| i.time));
    let cs = [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()];
    match tex {
        Some(id) => {
            if let Some(old) = prev {
                cover_quad(ui.painter(), cs, old, 1.0);
            }
            cover_quad(ui.painter(), cs, id, f);
            if f < 1.0 {
                ui.ctx().request_repaint();
            }
        }
        None => paint_art_g(ui, images, url, rect, 0.0, gray),
    }
}
