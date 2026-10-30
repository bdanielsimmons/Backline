//! Visuals: MilkDrop / Windows Media Player style pictures that move to the music, drawn on the graphics card, and
//! two old Windows screensavers (3D Pipes, Starfield) drawn by egui. Shown full screen, behind Now Playing, in the
//! deck's little visualizer, and as a screensaver when you step away.
//!
//! The feedback trick (most presets): every frame takes the last one, warps it a little (zoom, spin, swirl), fades
//! it, and draws the music on top. Each spot that shows visuals has its own pair of pictures to do that with.

use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use eframe::egui_glow;
use eframe::glow::{self, HasContext};
use crate::terrain::World;
use std::sync::{Arc, Mutex};

/// A preset: its group in the menu, its name, and how it's drawn.
pub(crate) struct Preset {
    pub group: &'static str,
    pub name: &'static str,
    pub kind: Kind,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Kind {
    /// A shader of its own, in full colour.
    Gpu(&'static str),
    /// The WMP-style engine: warp layers, an ink layer that draws the sound, and a palette (as Battery worked).
    Engine(Eng),
    /// Another preset every 20 seconds: from this group, or (None, Alchemy) from all of them.
    Random(Option<&'static str>),
    Pipes,
    Stars,
    /// 3D (ui/terrain.rs): a world (the terrain, a cavern, a canyon) in one of its looks, or (None) a different look
    /// for each song.
    Terrain(World, Option<usize>),
    /// 3D (ui/wave.rs): the wave, a sheet of light drifting across.
    Wave,
}

/// A palette: what the engine's brightness 0..1 becomes, dark to light.
pub(crate) type Pal = [[u8; 3]; 4];
const GOLD: Pal = [[24, 4, 32], [125, 62, 46], [228, 172, 74], [255, 248, 222]];
const FIRE: Pal = [[0, 0, 0], [150, 22, 0], [255, 140, 20], [255, 252, 200]];
const ICE: Pal = [[0, 4, 22], [24, 74, 156], [112, 202, 255], [240, 255, 255]];
const GREEN: Pal = [[0, 10, 0], [12, 92, 32], [112, 240, 112], [240, 255, 222]];
const VIOLET: Pal = [[10, 0, 22], [94, 22, 146], [222, 122, 255], [255, 242, 255]];
const MONO: Pal = [[0, 0, 0], [62, 60, 58], [172, 168, 160], [255, 252, 246]];
const BLUE: Pal = [[0, 0, 30], [0, 42, 146], [44, 144, 255], [224, 242, 255]];
const SUNSET: Pal = [[12, 0, 30], [164, 32, 92], [255, 122, 62], [255, 232, 152]];
const TEAL: Pal = [[0, 16, 16], [0, 92, 92], [62, 222, 192], [232, 255, 246]];
const AMBER: Pal = [[16, 8, 0], [112, 52, 0], [242, 152, 22], [255, 242, 182]];
const RED: Pal = [[10, 0, 4], [130, 0, 30], [255, 40, 80], [255, 220, 230]];

/// One engine preset. The warp (each frame takes the last one through it): `zoom` (<1 grows outward), `rot`,
/// `twist` (more spin near the middle), a radial wave (`rwa`, `rwf`), a drift (`dx`, `dy`), a `rip`ple, a `mir`ror
/// (n-fold kaleidoscope), `fade` and `blur`; bass adds to zoom (`bz`) and spin (`br`). Then the ink: 1 a filled
/// blob of the waveform, 2 a ring, 3 a scope line, 4 a filled wave, 5 a ring of dots, 6 spectrum rays, 7 bubbles,
/// 8 scribbles, 9 an X, 10 the spectrum as mountains, 11 spray, 12 bars from the top; `size`, `thick`, `gain`,
/// `count`, `spin`, `off` (its height). `hue` turns the palette round over time.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Eng {
    pub zoom: f32,
    pub rot: f32,
    pub twist: f32,
    pub rwa: f32,
    pub rwf: f32,
    pub dx: f32,
    pub dy: f32,
    pub rip: f32,
    pub mir: f32,
    pub fade: f32,
    pub blur: f32,
    pub ink: f32,
    pub size: f32,
    pub thick: f32,
    pub gain: f32,
    pub bz: f32,
    pub br: f32,
    pub count: f32,
    pub spin: f32,
    pub off: f32,
    /// grain in the warp: ragged, smoky edges
    pub jit: f32,
    pub pal: Pal,
    pub hue: f32,
}

const B: Eng = Eng {
    zoom: 1.0,
    rot: 0.0,
    twist: 0.0,
    rwa: 0.0,
    rwf: 8.0,
    dx: 0.0,
    dy: 0.0,
    rip: 0.0,
    mir: 0.0,
    fade: 0.97,
    blur: 0.0,
    ink: 2.0,
    size: 0.4,
    thick: 35.0,
    gain: 1.0,
    bz: 0.02,
    br: 0.0,
    count: 12.0,
    spin: 0.0,
    off: 0.0,
    jit: 0.003,
    pal: GOLD,
    hue: 0.004,
};

impl Eng {
    fn k(&self) -> [f32; 22] {
        [
            self.zoom, self.rot, self.twist, self.rwa, self.rwf, self.dx, self.dy, self.rip, self.mir, self.fade, self.blur,
            self.ink, self.size, self.thick, self.gain, self.bz, self.br, self.count, self.spin, self.off, self.jit, 0.0,
        ]
    }
}

const fn e(group: &'static str, name: &'static str, eng: Eng) -> Preset {
    Preset { group, name, kind: Kind::Engine(eng) }
}

pub(crate) const PRESETS: &[Preset] = &[
    Preset { group: "Alchemy", name: "Alchemy", kind: Kind::Random(None) },
    // ---- Ambience
    Preset { group: "Ambience", name: "Random", kind: Kind::Random(Some("Ambience")) },
    e("Ambience", "Swirl", Eng { zoom: 0.975, twist: 0.08, ink: 2.0, size: 0.35, fade: 0.975, blur: 0.5, ..B }),
    e("Ambience", "Warp", Eng { zoom: 0.955, rot: 0.01, rwa: 0.05, ink: 1.0, size: 0.45, fade: 0.978, blur: 0.5, pal: VIOLET, ..B }),
    e("Ambience", "Anon", Eng { zoom: 0.97, rot: 0.02, ink: 6.0, size: 1.0, fade: 0.965, pal: TEAL, ..B }),
    e("Ambience", "Falloff", Eng { dy: 0.015, ink: 3.0, size: 0.35, off: 0.6, fade: 0.985, blur: 1.0, jit: 0.004, pal: AMBER, ..B }),
    e("Ambience", "Water", Eng { zoom: 0.99, rip: 0.01, ink: 3.0, size: 0.5, fade: 0.975, blur: 0.5, pal: BLUE, ..B }),
    e("Ambience", "Bubble", Eng { zoom: 0.985, dy: -0.01, ink: 7.0, count: 8.0, size: 0.35, fade: 0.975, blur: 0.5, pal: ICE, ..B }),
    e("Ambience", "Dizzy", Eng { zoom: 0.97, rot: 0.1, ink: 3.0, size: 0.5, fade: 0.97, pal: SUNSET, hue: 0.02, ..B }),
    e("Ambience", "Windmill", Eng { zoom: 0.975, rot: 0.04, mir: 4.0, ink: 2.0, size: 0.5, fade: 0.97, pal: GREEN, ..B }),
    e("Ambience", "Niagara", Eng { dy: 0.02, rip: 0.002, ink: 12.0, size: 0.35, fade: 0.985, blur: 1.0, pal: BLUE, ..B }),
    e("Ambience", "Blender", Eng { zoom: 0.975, rot: 0.12, mir: 2.0, ink: 8.0, count: 4.0, size: 0.6, fade: 0.965, pal: VIOLET, hue: 0.01, ..B }),
    e("Ambience", "X Marks the Spot", Eng { zoom: 0.97, ink: 9.0, size: 0.4, fade: 0.97, pal: FIRE, ..B }),
    e("Ambience", "Down the Drain", Eng { zoom: 0.975, rot: 0.06, twist: 0.05, ink: 1.0, size: 0.6, fade: 0.975, blur: 1.0, jit: 0.004, ..B }),
    e("Ambience", "Thingus", Eng { zoom: 0.98, twist: 0.02, ink: 8.0, count: 5.0, size: 0.6, fade: 0.965, pal: TEAL, hue: 0.01, ..B }),
    // ---- Bars and Waves
    Preset { group: "Bars and Waves", name: "Bars", kind: Kind::Gpu(BARS) },
    e("Bars and Waves", "Ocean Mist", Eng { dy: -0.006, rip: 0.004, ink: 4.0, size: 0.3, off: -0.4, fade: 0.983, blur: 1.5, pal: ICE, bz: 0.0, ..B }),
    Preset { group: "Bars and Waves", name: "Fire Storm", kind: Kind::Gpu(FIRE_SH) },
    Preset { group: "Bars and Waves", name: "Scope", kind: Kind::Gpu(SCOPE) },
    // ---- Battery
    Preset { group: "Battery", name: "Randomization", kind: Kind::Random(Some("Battery")) },
    e("Battery", "BrightSphere", Eng { zoom: 0.97, rot: 0.02, ink: 1.0, size: 0.35, fade: 0.948, pal: AMBER, ..B }),
    e("Battery", "cominatya", Eng { zoom: 0.985, twist: 0.05, ink: 5.0, count: 18.0, size: 0.5, fade: 0.968, pal: VIOLET, ..B }),
    e("Battery", "dandelionaid", Eng { zoom: 0.98, rot: 0.01, ink: 6.0, size: 0.8, fade: 0.938, pal: GREEN, ..B }),
    e("Battery", "DrowningFlower", Eng { zoom: 0.975, mir: 6.0, ink: 2.0, size: 0.35, fade: 0.968, pal: SUNSET, ..B }),
    e("Battery", "EletriArnation", Eng { zoom: 0.98, rot: 0.03, mir: 2.0, ink: 3.0, size: 0.5, fade: 0.948, pal: ICE, hue: 0.04, ..B }),
    e("Battery", "cottonstar", Eng { zoom: 0.985, ink: 6.0, size: 0.6, fade: 0.978, blur: 1.5, pal: ICE, ..B }),
    e("Battery", "gemstone matrix", Eng { zoom: 0.975, mir: 8.0, ink: 5.0, count: 14.0, size: 0.55, fade: 0.965, pal: TEAL, hue: 0.01, ..B }),
    e("Battery", "GrooveSwirl", Eng { zoom: 0.99, twist: 0.08, ink: 2.0, size: 0.4, fade: 0.978, pal: VIOLET, hue: 0.08, ..B }),
    e("Battery", "eventhorizon", Eng { zoom: 1.04, rot: 0.03, ink: 2.0, size: 0.9, fade: 0.968, pal: FIRE, ..B }),
    e("Battery", "illuminator", Eng { zoom: 0.94, ink: 1.0, size: 0.3, fade: 0.965, blur: 1.0, pal: AMBER, ..B }),
    e("Battery", "ISeeTheTruth", Eng { zoom: 0.98, mir: 2.0, ink: 9.0, size: 0.4, fade: 0.958, pal: SUNSET, ..B }),
    e("Battery", "kaleidoscope", Eng { zoom: 0.985, rot: 0.02, mir: 6.0, ink: 8.0, count: 3.0, size: 0.5, fade: 0.968, pal: VIOLET, hue: 0.05, ..B }),
    e("Battery", "Nerds Are Cool", Eng { dy: -0.01, ink: 3.0, size: 0.3, fade: 0.978, pal: GREEN, bz: 0.0, ..B }),
    e("Battery", "Lotus", Eng { zoom: 0.99, rot: 0.01, mir: 5.0, ink: 6.0, size: 0.7, fade: 0.968, pal: VIOLET, ..B }),
    e("Battery", "relativelycalm", Eng { zoom: 0.995, rip: 0.004, ink: 4.0, size: 0.35, off: -0.2, fade: 0.98, blur: 1.0, pal: BLUE, bz: 0.0, ..B }),
    e("Battery", "sleepyspray", Eng { dy: -0.004, ink: 11.0, count: 30.0, fade: 0.973, blur: 0.5, pal: TEAL, ..B }),
    e("Battery", "Smoke or Water", Eng { zoom: 0.985, twist: 0.05, ink: 5.0, count: 24.0, size: 0.35, fade: 0.983, blur: 1.5, pal: MONO, ..B }),
    e("Battery", "what is an egab", Eng { rwa: 0.04, rwf: 10.0, ink: 2.0, size: 0.45, fade: 0.958, pal: SUNSET, hue: 0.03, ..B }),
    e("Battery", "SpidersLastMoment", Eng { ink: 8.0, count: 6.0, size: 0.8, thick: 45.0, fade: 0.918, pal: MONO, ..B }),
    e("Battery", "strawberryaid", Eng { zoom: 0.98, mir: 4.0, ink: 6.0, size: 0.8, fade: 0.948, pal: RED, ..B }),
    e("Battery", "the world", Eng { rot: 0.01, rip: 0.004, ink: 1.0, size: 0.5, fade: 0.978, pal: TEAL, ..B }),
    e("Battery", "circledance", Eng { zoom: 0.985, ink: 5.0, count: 14.0, size: 0.6, spin: 1.5, fade: 0.955, blur: 1.0, pal: BLUE, ..B }),
    e("Battery", "tornado", Eng { zoom: 0.98, twist: 0.1, dy: -0.008, ink: 5.0, count: 30.0, size: 0.6, fade: 0.97, pal: AMBER, ..B }),
    e("Battery", "Geeks Kick ASCII", Eng { dy: 0.015, ink: 12.0, size: 0.3, fade: 0.978, pal: MONO, bz: 0.0, ..B }),
    e("Battery", "khemicalnova", Eng { zoom: 0.96, rot: 0.04, ink: 2.0, size: 0.2, fade: 0.958, blur: 0.5, pal: VIOLET, hue: 0.1, ..B }),
    // ---- Musical Colors
    Preset { group: "Musical Colors", name: "Random", kind: Kind::Random(Some("Musical Colors")) },
    e("Musical Colors", "Night Lights", Eng { zoom: 0.99, ink: 5.0, count: 30.0, size: 0.6, spin: 0.3, fade: 0.985, pal: SUNSET, ..B }),
    e("Musical Colors", "Colors in Motion", Eng { dy: -0.006, ink: 10.0, size: 0.6, fade: 0.968, pal: VIOLET, hue: 0.1, ..B }),
    e("Musical Colors", "Aurora", Eng { dy: -0.004, rip: 0.003, ink: 4.0, size: 0.3, off: 0.3, fade: 0.985, blur: 1.5, pal: TEAL, ..B }),
    e("Musical Colors", "Rhythmic Colors", Eng { zoom: 0.975, mir: 4.0, ink: 6.0, size: 0.95, fade: 0.95, pal: SUNSET, hue: 0.03, ..B }),
    e("Musical Colors", "Star Power", Eng { zoom: 0.97, ink: 6.0, size: 0.9, fade: 0.948, pal: AMBER, ..B }),
    e("Musical Colors", "Electric Green", Eng { mir: 2.0, ink: 3.0, size: 0.45, fade: 0.948, pal: GREEN, ..B }),
    e("Musical Colors", "Soft Fire", Eng { dy: -0.008, ink: 10.0, size: 0.55, fade: 0.978, blur: 1.5, pal: FIRE, bz: 0.0, ..B }),
    Preset { group: "Musical Colors", name: "Silky Wave", kind: Kind::Gpu(SILK) },
    e("Musical Colors", "CutOut", Eng { ink: 10.0, size: 0.7, fade: 0.5, pal: MONO, bz: 0.0, ..B }),
    e("Musical Colors", "Rolling Fire", Eng { zoom: 0.985, rot: 0.01, ink: 10.0, size: 0.6, fade: 0.968, pal: FIRE, ..B }),
    e("Musical Colors", "Water Spray", Eng { ink: 11.0, count: 40.0, fade: 0.958, pal: BLUE, ..B }),
    e("Musical Colors", "Acid Rock", Eng { mir: 2.0, rip: 0.006, ink: 10.0, size: 0.6, fade: 0.968, pal: GREEN, hue: 0.15, ..B }),
    e("Musical Colors", "Hard Rock", Eng { ink: 10.0, size: 0.8, fade: 0.85, pal: SUNSET, bz: 0.0, ..B }),
    e("Musical Colors", "Hot Spray", Eng { ink: 11.0, count: 50.0, fade: 0.948, pal: FIRE, ..B }),
    e("Musical Colors", "Yellow Swirl", Eng { twist: 0.07, ink: 2.0, size: 0.4, fade: 0.978, pal: AMBER, hue: 0.0, ..B }),
    e("Musical Colors", "Blue Flame", Eng { dy: -0.012, ink: 10.0, size: 0.5, fade: 0.978, blur: 1.5, pal: BLUE, bz: 0.0, hue: 0.0, ..B }),
    e("Musical Colors", "Critter Rock", Eng { ink: 8.0, count: 4.0, size: 0.6, fade: 0.948, pal: GREEN, ..B }),
    e("Musical Colors", "Electric Rainbow", Eng { dy: -0.01, ink: 3.0, size: 0.4, fade: 0.978, pal: VIOLET, hue: 0.2, bz: 0.0, ..B }),
    e("Musical Colors", "Neon Highway", Eng { dx: 0.01, ink: 10.0, size: 0.5, fade: 0.985, pal: VIOLET, bz: 0.0, ..B }),
    e("Musical Colors", "Ice Crystals", Eng { zoom: 0.975, mir: 6.0, ink: 5.0, count: 18.0, size: 0.6, fade: 0.965, pal: ICE, hue: 0.0, ..B }),
    // ---- Particle
    e("Particle", "Particle", Eng { zoom: 0.985, ink: 5.0, count: 40.0, size: 0.8, spin: 0.5, fade: 0.93, pal: ICE, ..B }),
    e("Particle", "Rotating Particle", Eng { rot: 0.03, ink: 5.0, count: 40.0, size: 0.7, spin: 0.5, fade: 0.948, pal: SUNSET, ..B }),
    // ---- Plenoptic
    Preset { group: "Plenoptic", name: "Random", kind: Kind::Random(Some("Plenoptic")) },
    e("Plenoptic", "Smokey Circles", Eng { dy: -0.006, ink: 7.0, count: 5.0, size: 0.3, fade: 0.985, blur: 1.5, pal: MONO, ..B }),
    e("Plenoptic", "Smokey Lines", Eng { dy: -0.006, ink: 8.0, count: 4.0, size: 0.5, fade: 0.985, blur: 1.5, pal: VIOLET, ..B }),
    e("Plenoptic", "Vox", Eng { zoom: 0.98, mir: 2.0, ink: 1.0, size: 0.3, fade: 0.958, pal: SUNSET, ..B }),
    e("Plenoptic", "Flame", Eng { dy: -0.015, ink: 10.0, size: 0.5, fade: 0.978, blur: 1.5, pal: FIRE, bz: 0.0, hue: 0.0, ..B }),
    e("Plenoptic", "Fountain", Eng { ink: 11.0, count: 60.0, fade: 0.968, blur: 0.5, pal: ICE, ..B }),
    // ---- Spikes
    Preset { group: "Spikes", name: "Spike", kind: Kind::Gpu(SPIKE) },
    Preset { group: "Spikes", name: "Amoeba", kind: Kind::Gpu(AMOEBA) },
    // ---- MilkDrop style
    Preset { group: "Milk", name: "Warp Tunnel", kind: Kind::Gpu(TUNNEL) },
    Preset { group: "Milk", name: "Trails", kind: Kind::Gpu(TRAILS) },
    Preset { group: "Milk", name: "Warp Stars", kind: Kind::Gpu(STARS) },
    Preset { group: "Milk", name: "Kaleidoscope", kind: Kind::Gpu(KALEIDO) },
    Preset { group: "Milk", name: "Plasma", kind: Kind::Gpu(PLASMA) },
    Preset { group: "Milk", name: "Tornado", kind: Kind::Gpu(TORNADO) },
    Preset { group: "Milk", name: "Smoke Rings", kind: Kind::Gpu(SMOKE) },
    // ---- 3D (the terrain's looks in assets/terrain_looks.bin, in order)
    Preset { group: "3D", name: "Wave", kind: Kind::Wave },
    Preset { group: "3D", name: "Terrain", kind: Kind::Terrain(World::Exact, None) },
    Preset { group: "3D", name: "Experimental Terrain", kind: Kind::Terrain(World::Terrain, None) },
    Preset { group: "3D", name: "Cavern", kind: Kind::Terrain(World::Cavern, None) },
    Preset { group: "3D", name: "Canyon", kind: Kind::Terrain(World::Canyon, None) },
    Preset { group: "3D", name: "Rose Terrain", kind: Kind::Terrain(World::Terrain, Some(0)) },
    Preset { group: "3D", name: "Blue Terrain", kind: Kind::Terrain(World::Terrain, Some(1)) },
    Preset { group: "3D", name: "Meadow Terrain", kind: Kind::Terrain(World::Terrain, Some(2)) },
    Preset { group: "3D", name: "Night Terrain", kind: Kind::Terrain(World::Terrain, Some(3)) },
    Preset { group: "3D", name: "Sepia Terrain", kind: Kind::Terrain(World::Terrain, Some(4)) },
    Preset { group: "3D", name: "Amber Terrain", kind: Kind::Terrain(World::Terrain, Some(5)) },
    Preset { group: "3D", name: "Aurora Terrain", kind: Kind::Terrain(World::Terrain, Some(6)) },
    // ---- screensavers
    Preset { group: "Screensavers", name: "3D Pipes", kind: Kind::Pipes },
    Preset { group: "Screensavers", name: "Starfield", kind: Kind::Stars },
];

/// The menu: the 3D worlds first, then Windows Media Player's in its order, the MilkDrop-style ones, the screensavers,
/// and Alchemy (a mix of all of them) last.
pub(crate) const GROUPS: &[&str] =
    &["3D", "Ambience", "Bars and Waves", "Battery", "Musical Colors", "Particle", "Plenoptic", "Spikes", "Milk", "Screensavers", "Alchemy"];

/// A group's name in the menu.
pub(crate) fn group_title(g: &str) -> &str {
    match g {
        "Milk" => "MilkDrop style",
        _ => g,
    }
}

/// A preset by name, as saved ("Group: Name", or an older plain name); Down the Drain when unknown.
pub(crate) fn find(name: &str) -> usize {
    let (g, n) = name.split_once(": ").unwrap_or(("", name));
    PRESETS
        .iter()
        .position(|p| p.name.eq_ignore_ascii_case(n) && (g.is_empty() || p.group == g))
        .or_else(|| PRESETS.iter().position(|p| p.name.eq_ignore_ascii_case(n)))
        .unwrap_or_else(|| PRESETS.iter().position(|p| p.name == "Down the Drain").unwrap_or(0))
}

/// How a preset is saved: "Group: Name" (several groups have a Random).
pub(crate) fn save_name(i: usize) -> String {
    let p = &PRESETS[i.min(PRESETS.len() - 1)];
    format!("{}: {}", p.group, p.name)
}

/// The preset actually showing at time `t`: a Random one picks another every 20 seconds.
pub(crate) fn resolve(i: usize, t: f32) -> usize {
    let Kind::Random(g) = PRESETS[i].kind else { return i };
    let pool: Vec<usize> = (0..PRESETS.len())
        .filter(|&j| match PRESETS[j].kind {
            // (the terrain takes a while to build: never a twenty-second stop in a round)
            Kind::Random(_) | Kind::Pipes | Kind::Stars | Kind::Terrain(..) | Kind::Wave => false,
            _ => g.map_or(true, |g| PRESETS[j].group == g),
        })
        .collect();
    if pool.is_empty() {
        return i;
    }
    let slot = (t / 20.0).floor() as u64;
    let h = slot.wrapping_mul(0x9e3779b97f4a7c15).rotate_left(17).wrapping_mul(0xbf58476d1ce4e5b9);
    pool[(h % pool.len() as u64) as usize]
}

/// What the music is doing this frame, for the shaders.
#[derive(Clone, Copy)]
pub(crate) struct Frame {
    pub t: f32,
    pub travel: f32,
    pub bass: f32,
    pub mid: f32,
    pub high: f32,
    pub wave: [f32; 64],
    pub spec: [f32; 32],
    pub peak: [f32; 32],
}

/// Automatic gain and beats: slow averages of the bass and the loudest swing, so the visuals react as strongly to a
/// quiet jazz trio as to a loud record (as WMP's did), and a kick on each beat (bass well over its recent level).
#[derive(Default, Clone, Copy)]
pub(crate) struct Agc {
    bass_avg: f32,
    wave_peak: f32,
    spec_peak: f32,
    beat: f32,
}

impl Agc {
    pub(crate) fn apply(&mut self, f: &mut Frame, dt: f32) {
        let k_slow = 1.0 - (-dt / 1.5).exp();
        self.bass_avg += (f.bass - self.bass_avg) * k_slow;
        let wp = f.wave.iter().fold(0f32, |m, v| m.max(v.abs()));
        let sp = f.spec.iter().fold(0f32, |m, v| m.max(*v));
        // peaks: up at once, down over a few seconds
        self.wave_peak = if wp > self.wave_peak { wp } else { self.wave_peak + (wp - self.wave_peak) * (1.0 - (-dt / 4.0).exp()) };
        self.spec_peak = if sp > self.spec_peak { sp } else { self.spec_peak + (sp - self.spec_peak) * (1.0 - (-dt / 4.0).exp()) };
        let wg = (0.8 / self.wave_peak.max(0.08)).min(6.0);
        let sg = (0.9 / self.spec_peak.max(0.12)).min(4.0);
        for v in f.wave.iter_mut() {
            *v = (*v * wg).clamp(-1.0, 1.0);
        }
        for v in f.spec.iter_mut().chain(f.peak.iter_mut()) {
            *v = (*v * sg).min(1.0);
        }
        // a beat: the bass jumps well over its recent average; it rings out over a fifth of a second
        let rel = f.bass / self.bass_avg.max(0.02);
        if rel > 1.35 && f.bass > 0.03 {
            self.beat = self.beat.max(((rel - 1.0) * 0.6).min(1.0));
        }
        self.beat *= (-dt / 0.2).exp();
        let level = (f.bass / self.bass_avg.max(0.02) * 0.35).min(1.0);
        f.bass = (0.35 * level + 0.9 * self.beat).min(1.4);
        f.mid = (f.mid * sg).min(1.0);
        f.high = (f.high * sg).min(1.2);
    }
}

impl Frame {
    /// From the player's analysis: bands and peaks (low to high, 0..1) and the waveform (-1..1).
    pub(crate) fn from_audio(t: f32, travel: f32, bands: &[f32], peaks: &[f32], wave: &[f32]) -> Self {
        let pick = |src: &[f32], n: usize| -> Vec<f32> {
            (0..n).map(|i| if src.is_empty() { 0.0 } else { src[(i * src.len() / n).min(src.len() - 1)] }).collect()
        };
        let avg = |a: f32, b: f32| -> f32 {
            let n = bands.len();
            if n == 0 {
                return 0.0;
            }
            let (i, j) = (((a * n as f32) as usize).min(n - 1), ((b * n as f32) as usize).clamp(1, n));
            bands[i..j.max(i + 1)].iter().sum::<f32>() / (j.max(i + 1) - i) as f32
        };
        let mut f = Frame {
            t,
            travel,
            bass: avg(0.0, 0.15).powf(2.0) * 1.5,
            mid: avg(0.15, 0.55),
            high: avg(0.7, 1.0) * 1.3,
            wave: [0.0; 64],
            spec: [0.0; 32],
            peak: [0.0; 32],
        };
        f.wave.copy_from_slice(&pick(wave, 64));
        f.spec.copy_from_slice(&pick(bands, 32));
        f.peak.copy_from_slice(&pick(peaks, 32));
        f
    }
}

// ------------------------------------------------------------------------------------------------ the GPU side
const HEAD: &str = r#"
#ifdef GL_ES
precision highp float;
#endif
in vec2 v; out vec4 o;
uniform sampler2D prev; uniform vec2 res; uniform float t, bass, mid, high, travel;
uniform float wave[64]; uniform float spec[32]; uniform float peak[32];
float W(float x){ x = clamp(x, 0., 1.) * 63.; int i = int(x); return mix(wave[i], wave[min(i + 1, 63)], fract(x)); }
float S(float x){ x = clamp(x, 0., 1.) * 31.; int i = int(x); return mix(spec[i], spec[min(i + 1, 31)], fract(x)); }
vec3 pal(float x){ return .5 + .5 * cos(6.2831 * (x + vec3(0., .33, .67))); }
float asp(){ return res.x / res.y; }
vec2 P(){ vec2 p = (v - .5) * 2.; p.x *= asp(); return p; }
vec3 back(vec2 p){ p.x /= asp(); return texture(prev, p * .5 + .5).rgb; }
float hash(vec2 q){ return fract(sin(dot(q, vec2(127.1, 311.7))) * 43758.5453); }
mat2 rot(float a){ return mat2(cos(a), -sin(a), sin(a), cos(a)); }
vec3 fade(vec3 c, float k){ return max(c * k - 1.5 / 255., 0.); }
"#;

const VERT: &str = r#"
out vec2 v;
void main(){ vec2 a = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2)); v = a; gl_Position = vec4(a * 2. - 1., 0., 1.); }
"#;

const COPY: &str = r#"
#ifdef GL_ES
precision highp float;
#endif
in vec2 v; out vec4 o; uniform sampler2D prev;
uniform int mode; uniform vec3 c0, c1, c2, c3; uniform float hue;
vec3 turn(vec3 c, float a){
  // round the colour wheel, keeping brightness (YIQ)
  float y = dot(c, vec3(.299, .587, .114)), i = dot(c, vec3(.596, -.274, -.322)), q = dot(c, vec3(.211, -.523, .312));
  float ca = cos(a), sa = sin(a); float i2 = i * ca - q * sa, q2 = i * sa + q * ca;
  return clamp(vec3(y + .956 * i2 + .621 * q2, y - .272 * i2 - .647 * q2, y - 1.106 * i2 + 1.703 * q2), 0., 1.);
}
void main(){
  vec3 c = texture(prev, v).rgb;
  if (mode == 1) {
    float x = clamp(c.r, 0., 1.);
    vec3 col = x < .3333 ? mix(c0, c1, x * 3.) : x < .6667 ? mix(c1, c2, (x - .3333) * 3.) : mix(c2, c3, (x - .6667) * 3.);
    c = turn(col, hue);
  }
  o = vec4(c, 1.);
}
"#;

/// The engine: the last frame through the preset's warp, faded, with the music drawn on top as brightness (the
/// palette colours it on the way to the screen). `k` is `Eng::k`.
const ENGINE: &str = r#"
uniform float k[22];
float back1(vec2 q){ return back(q).r; }
void main(){
  vec2 p = P(); float r = length(p), a = atan(p.y, p.x);
  // ---- the warp
  float z = k[0] - k[15] * bass;
  float spin = k[1] + k[16] * bass + k[2] * max(0., 1.2 - r);
  vec2 q = rot(spin) * p * z;
  q *= 1. + k[3] * sin(r * k[4] - t * 2.);
  q += vec2(k[5], k[6]);
  q += k[7] * vec2(sin(p.y * 10. + t * 2.), cos(p.x * 9. + t * 1.7));
  q += (vec2(hash(p * res + t), hash(p.yx * res - t * 1.3)) - .5) * k[20];
  if (k[8] > .5) { float an = atan(q.y, q.x), seg = 6.2831 / k[8]; an = abs(mod(an, seg) - seg * .5); q = length(q) * vec2(cos(an), sin(an)); }
  float c;
  if (k[10] > 0.) { vec2 d = vec2(k[10]) / res.y * 2.;
    c = (back1(q + vec2(d.x, 0.)) + back1(q - vec2(d.x, 0.)) + back1(q + vec2(0., d.y)) + back1(q - vec2(0., d.y))) * .25;
  } else c = back1(q);
  c = max(c * k[9] - 1.5 / 255., 0.);
  // ---- the ink
  int ink = int(k[11] + .5); float sz = k[12], th = k[13], g = k[14], n = k[17], off = k[19];
  vec2 pp = rot(t * k[18]) * p; float aa = atan(pp.y, pp.x) / 6.2831 + .5;
  float v2 = 0.;
  if (ink == 1) {
    float R = sz * (1. + .3 * bass) + .28 * sz * W(abs(aa * 2. - 1.));
    v2 = smoothstep(R, R - .03, r) * (.95 - .35 * r / max(R, .01));
  } else if (ink == 2) {
    float R = sz + .15 * W(abs(aa * 2. - 1.)) + .08 * bass;
    v2 = exp(-abs(r - R) * th);
  } else if (ink == 3) {
    v2 = exp(-abs(p.y - (off + W(v.x) * sz * (1. + bass))) * th);
  } else if (ink == 4) {
    float y0 = off + W(v.x) * sz;
    v2 = max(step(p.y, y0) * .6 * smoothstep(-1., y0, p.y), exp(-abs(p.y - y0) * th));
  } else if (ink == 5) {
    for (int i = 0; i < 40; i++) { if (float(i) >= n) break; float f = float(i) / n;
      vec2 at = vec2(cos(f * 6.2831), sin(f * 6.2831)) * sz * (.45 + .7 * S(f));
      v2 = max(v2, smoothstep(.045, 0., length(pp - at))); }
  } else if (ink == 6) {
    float len = .1 + sz * S(abs(aa * 2. - 1.)), ray = smoothstep(.4, 0., abs(fract(aa * 48.) - .5));
    v2 = ray * step(r, len);
  } else if (ink == 7) {
    float slot = floor(t * 2.);
    for (int i = 0; i < 12; i++) { if (float(i) >= n) break; vec2 sd = vec2(float(i), slot);
      vec2 at = (vec2(hash(sd), hash(sd + 3.1)) * 2. - 1.) * vec2(asp(), 1.) * .75;
      float rr = .05 + sz * (.4 + bass) * hash(sd + 7.7);
      v2 = max(v2, exp(-abs(length(p - at) - rr) * th)); }
  } else if (ink == 8) {
    for (int i = 0; i < 8; i++) { if (float(i) >= n) break; float fi = float(i);
      float y = sin(p.x * (2. + fi) + t * (1. + .3 * fi)) * sz * (.35 + bass) * cos(p.x * .7 + fi) + (fi - n * .5) * .1;
      v2 = max(v2, exp(-abs(p.y - y) * th)); }
  } else if (ink == 9) {
    float m = sz * .25 * W(abs(p.x) / asp());
    v2 = exp(-min(abs(p.x - p.y + m), abs(p.x + p.y - m)) * th) * step(r, sz * 2.5);
  } else if (ink == 10) {
    float top = -1. + S(v.x) * sz * 2.2;
    v2 = max(step(p.y, top) * (.45 + .2 * (top - p.y)), exp(-abs(p.y - top) * th));
  } else if (ink == 11) {
    for (int i = 0; i < 60; i++) { if (float(i) >= n) break; float fi = float(i);
      float h = hash(vec2(fi, 1.7)), ph = fract(t * (.35 + .3 * h) + hash(vec2(fi, 9.1)));
      vec2 at = vec2((hash(vec2(fi, 4.2)) - .5) * 3.4 * ph * asp() * .6, -1. + (2.4 + bass) * ph - 1.7 * ph * ph);
      v2 = max(v2, smoothstep(.05 * (1. + bass), 0., length(p - at)) * (1. - ph * .5)); }
  } else if (ink == 12) {
    v2 = step(1. - S(v.x) * sz * 2., v.y * 2. - 1.);
  }
  c = max(c, v2 * g);
  o = vec4(vec3(clamp(c, 0., 1.)), 1.);
}
"#;

const TUNNEL: &str = r#"void main(){ vec2 p = P();
  vec3 c = fade(back(rot(.012 + .03 * mid) * p * (.965 - .045 * bass)), .962);
  float ang = atan(p.y, p.x) / 6.2831 + .5; float r = .32 + .2 * W(abs(ang * 2. - 1.)) + .12 * bass;
  c += pal(t * .08 + ang) * exp(-abs(length(p) - r) * 85.) * (.6 + bass);
  o = vec4(c, 1.); }"#;

const TRAILS: &str = r#"void main(){ vec2 p = P();
  vec2 q = p * .985 + .012 * vec2(sin(p.y * 3. + t), cos(p.x * 3. + t * 1.3));
  vec3 c = fade(back(q), .955);
  float y = W(v.x) * .55; c += pal(t * .06 + v.x * .5) * exp(-abs(p.y - y) * 65.) * (.55 + mid);
  o = vec4(c, 1.); }"#;

const STARS: &str = r#"void main(){ vec2 p = P();
  vec3 c = max(back(p * (.95 - .05 * bass)) * .86 - 2. / 255., 0.);
  for (int i = 0; i < 4; i++) {
    float d = fract(travel * .12 + float(i) * .25), sc = mix(18., .7, d);
    vec2 uv = p * sc + float(i) * 7.3, id = floor(uv), f = fract(uv) - .5;
    float h = hash(id); vec2 off = (vec2(hash(id + 1.3), hash(id + 2.7)) - .5) * .6;
    float s = smoothstep(.07, 0., length(f - off)) * d * step(.55, h);
    c += s * mix(vec3(.75, .85, 1.), pal(h), .45) * (1.2 + high);
  }
  o = vec4(c, 1.); }"#;

const KALEIDO: &str = r#"void main(){ vec2 p = rot(t * .1) * P();
  float r = length(p), a = atan(p.y, p.x), seg = 6.2831 / 6.; a = abs(mod(a, seg) - seg * .5);
  vec2 k = r * vec2(cos(a), sin(a));
  float f = sin(k.x * 9. + t * 2.) * sin(k.y * 9. - t * 1.5) + sin(r * 12. - t * 3. - bass * 5.);
  vec3 col = pal(f * .3 + t * .05 + r * .5) * smoothstep(.25, 1., abs(f)) * (.35 + bass);
  o = vec4(fade(back(p * 1.012), .9) + col * .4, 1.); }"#;

const PLASMA: &str = r#"void main(){ vec2 p = P();
  float f = sin(p.x * 3. + t) + sin(p.y * 4. - t * 1.2) + sin((p.x + p.y) * 3. + t * .7) + sin(length(p) * 6. - t * 2. - bass * 6.);
  o = vec4(pal(f * .25 + t * .05) * (.5 + .6 * bass), 1.); }"#;

// Bars & Waves
const BARS: &str = r#"void main(){ float n = 32., i = floor(v.x * n), fx = fract(v.x * n);
  float h = spec[int(i)] * .62, pk = peak[int(i)] * .62, base = .28, gap = step(.12, fx) * step(fx, .88);
  vec3 c = vec3(.01, .01, .03);
  if (v.y > base) { float y = v.y - base;
    if (y < h) c = mix(vec3(.1, .85, .35), vec3(1., .85, .2), y / .62) * gap;
    if (abs(y - pk - .006) < .006) c = vec3(.95) * gap;
  } else { float y = (base - v.y) * 1.6;
    if (y < h) c = mix(vec3(.1, .85, .35), vec3(1., .85, .2), y / .62) * gap * .3 * (1. - y * 1.4); }
  o = vec4(c, 1.); }"#;

const SCOPE: &str = r#"void main(){ vec2 p = P();
  vec3 c = fade(back(p), .72);
  float d = abs(p.y - W(v.x) * .7);
  c += vec3(.3, 1., .5) * exp(-d * 120.) * 1.2 + vec3(.1, .5, .3) * exp(-d * 22.) * .35;
  o = vec4(c, 1.); }"#;

const FIRE_SH: &str = r#"void main(){
  float sc = res.y / 720.;
  vec2 q = v - vec2((hash(v * res + t) - .5) * 4. * sc, (3. + hash(v.yx * res - t) * 3.) * sc) / res;
  vec3 c = texture(prev, q).rgb * vec3(.99, .962, .9) - .6 / 255.;
  if (v.y < .05) { float h = S(v.x) * 1.6 * (.55 + .7 * hash(vec2(floor(v.x * 90.), floor(t * 20.))));
    c = max(c, vec3(1., .78, .35) * h * (1. - v.y * 10.)); }
  o = vec4(max(c, 0.), 1.); }"#;

const SILK: &str = r#"void main(){ vec2 p = P();
  vec3 c = fade(back(vec2(p.x * .995, p.y * .99 - .004)), .955) * vec3(1., .8, .86);
  float top = -1. + S(v.x) * 1.7 + .05 * sin(v.x * 9. + t);
  if (p.y < top) { float k = (top - p.y);
    c = mix(vec3(.55, 1., 1.), vec3(.05, .55, .85), smoothstep(0., .9, k)); }
  c += vec3(1., .25, .55) * exp(-abs(p.y - top) * 60.) * .8;
  o = vec4(c, 1.); }"#;

// Ambience
const TORNADO: &str = r#"void main(){ vec2 p = P(); float r = length(p);
  vec3 c = fade(back(rot(.09 * exp(-r * 1.4)) * p * .985 + vec2(0., -.008)), .955);
  for (int i = 0; i < 24; i++) { float k = float(i) / 24.;
    vec2 at = vec2(cos(k * 6.2831 + t), sin(k * 6.2831 + t)) * (.3 + .3 * S(k));
    c += pal(t * .05 + k) * smoothstep(.035, 0., length(p - at)) * .8; }
  o = vec4(c, 1.); }"#;

// Spikes
const SPIKE: &str = r#"void main(){ vec2 p = P(); float r = length(p), a = atan(p.y, p.x) / 6.2831 + .5;
  vec3 c = fade(back(p * .97), .85);
  float len = .12 + .9 * S(abs(a * 2. - 1.)), ray = smoothstep(.35, 0., abs(fract(a * 48.) - .5));
  c += pal(t * .1 + r * .6) * ray * step(r, len) * (.4 + bass);
  o = vec4(c, 1.); }"#;

const AMOEBA: &str = r#"void main(){ vec2 p = P(); float r = length(p), a = atan(p.y, p.x);
  float R = .38 + .3 * S(abs(fract(a / 6.2831 + .5) * 2. - 1.)) + .05 * sin(a * 5. + t * 2.) + .1 * bass;
  vec3 c = fade(back(p * .99), .8);
  if (r < R) c = mix(c, pal(t * .04 + r) * (.35 + .4 * r / R), .6);
  c += pal(t * .04 + .5) * exp(-abs(r - R) * 50.);
  o = vec4(c, 1.); }"#;

const SMOKE: &str = r#"void main(){ vec2 p = P();
  vec2 q = p * .995 + vec2(.004 * sin(p.y * 5. + t), -.006);
  vec3 c = fade(back(q), .97) * vec3(.99, .98, 1.);
  float slot = floor(t * 2.);
  for (int i = 0; i < 5; i++) { vec2 seed = vec2(float(i), slot);
    vec2 at = (vec2(hash(seed), hash(seed + 3.1)) * 2. - 1.) * vec2(asp(), 1.) * .7;
    float rr = .08 + .3 * bass * hash(seed + 7.7);
    c += pal(float(i) * .2 + t * .05) * exp(-abs(length(p - at) - rr) * 80.) * .5; }
  o = vec4(c, 1.); }"#;

/// One spot's pair of pictures (feedback: draw the next from the last, then swap).
struct Slot {
    tex: [glow::Texture; 2],
    fbo: [glow::Framebuffer; 2],
    w: i32,
    h: i32,
    cur: usize,
}

#[derive(Default)]
pub(crate) struct Gl {
    head: String,
    progs: std::collections::HashMap<usize, glow::Program>,
    copy: Option<glow::Program>,
    vao: Option<glow::VertexArray>,
    slots: std::collections::HashMap<u8, Slot>,
    failed: bool,
    terrain: Option<Box<crate::terrain::Terrain>>,
    wave: Option<Box<crate::wave::Wave>>,
}

/// A program: the shared vertex shader and `frag`, each after the GL version line (`frag` brings its own header).
unsafe fn program(gl: &glow::Context, version: &str, frag: &str) -> Option<glow::Program> {
    let p = gl.create_program().ok()?;
    let mut shaders = Vec::new();
    for (kind, src) in
        [(glow::VERTEX_SHADER, format!("{}{}", version, VERT)), (glow::FRAGMENT_SHADER, format!("{}{}", version, frag))]
    {
        let s = gl.create_shader(kind).ok()?;
        gl.shader_source(s, &src);
        gl.compile_shader(s);
        if !gl.get_shader_compile_status(s) {
            crate::api::log(&format!("visuals: shader didn't compile: {}", gl.get_shader_info_log(s)));
            return None;
        }
        gl.attach_shader(p, s);
        shaders.push(s);
    }
    gl.link_program(p);
    for s in shaders {
        gl.detach_shader(p, s);
        gl.delete_shader(s);
    }
    if !gl.get_program_link_status(p) {
        crate::api::log(&format!("visuals: shader didn't link: {}", gl.get_program_info_log(p)));
        return None;
    }
    Some(p)
}

impl Gl {
    unsafe fn draw(&mut self, gl: &glow::Context, info: &egui::PaintCallbackInfo, slot_id: u8, preset: usize, f: &Frame) {
        if self.failed {
            return;
        }
        if self.vao.is_none() {
            self.head = egui_glow::ShaderVersion::get(gl).version_declaration().to_string();
            self.vao = gl.create_vertex_array().ok();
            self.copy = program(gl, &self.head, COPY);
            if self.vao.is_none() || self.copy.is_none() {
                self.failed = true;
                return;
            }
        }
        let (src, eng) = match PRESETS[preset].kind {
            Kind::Gpu(src) => (src, None),
            Kind::Engine(e) => (ENGINE, Some(e)),
            _ => return,
        };
        let key = src.as_ptr() as usize;
        if !self.progs.contains_key(&key) {
            match program(gl, &self.head, &format!("{}{}", HEAD, src)) {
                Some(p) => {
                    self.progs.insert(key, p);
                }
                None => {
                    self.failed = true;
                    return;
                }
            }
        }
        let prog = self.progs[&key];
        let vp = info.viewport_in_pixels();
        let (w, h) = (vp.width_px.max(2), vp.height_px.max(2));
        // the slot's pictures, made again when its size changes; a new preset carries on from the last one's picture
        // (the way MilkDrop blends from one into the next)
        let fresh = self.slots.get(&slot_id).map_or(true, |s| s.w != w || s.h != h);
        if fresh {
            if let Some(old) = self.slots.remove(&slot_id) {
                for i in 0..2 {
                    gl.delete_texture(old.tex[i]);
                    gl.delete_framebuffer(old.fbo[i]);
                }
            }
            let mut tex = Vec::new();
            let mut fbo = Vec::new();
            for _ in 0..2 {
                let (Ok(t), Ok(b)) = (gl.create_texture(), gl.create_framebuffer()) else {
                    self.failed = true;
                    return;
                };
                gl.bind_texture(glow::TEXTURE_2D, Some(t));
                gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, w, h, 0, glow::RGBA, glow::UNSIGNED_BYTE, None);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(b));
                gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(t), 0);
                gl.disable(glow::SCISSOR_TEST);
                gl.viewport(0, 0, w, h);
                gl.clear_color(0.0, 0.0, 0.0, 1.0);
                gl.clear(glow::COLOR_BUFFER_BIT);
                tex.push(t);
                fbo.push(b);
            }
            self.slots.insert(slot_id, Slot { tex: [tex[0], tex[1]], fbo: [fbo[0], fbo[1]], w, h, cur: 0 });
        }
        let s = self.slots.get_mut(&slot_id).unwrap();
        gl.disable(glow::BLEND);
        gl.bind_vertex_array(self.vao);
        gl.use_program(Some(prog));
        let u = |n: &str| gl.get_uniform_location(prog, n);
        gl.uniform_2_f32(u("res").as_ref(), w as f32, h as f32);
        gl.uniform_1_f32(u("t").as_ref(), f.t);
        gl.uniform_1_f32(u("travel").as_ref(), f.travel);
        gl.uniform_1_f32(u("bass").as_ref(), f.bass);
        gl.uniform_1_f32(u("mid").as_ref(), f.mid);
        gl.uniform_1_f32(u("high").as_ref(), f.high);
        gl.uniform_1_f32_slice(u("wave").as_ref(), &f.wave);
        gl.uniform_1_f32_slice(u("spec").as_ref(), &f.spec);
        gl.uniform_1_f32_slice(u("peak").as_ref(), &f.peak);
        gl.uniform_1_i32(u("prev").as_ref(), 0);
        if let Some(e) = eng {
            gl.uniform_1_f32_slice(u("k").as_ref(), &e.k());
        }
        gl.active_texture(glow::TEXTURE0);
        // the next picture, from the last (or straight from the music)
        gl.bind_texture(glow::TEXTURE_2D, Some(s.tex[s.cur]));
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(s.fbo[1 - s.cur]));
        gl.disable(glow::SCISSOR_TEST);
        gl.viewport(0, 0, w, h);
        gl.draw_arrays(glow::TRIANGLES, 0, 3);
        s.cur = 1 - s.cur;
        // onto the screen, inside egui's clip
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        gl.viewport(vp.left_px, vp.from_bottom_px, vp.width_px, vp.height_px);
        let clip = info.clip_rect_in_pixels();
        gl.enable(glow::SCISSOR_TEST);
        gl.scissor(clip.left_px, clip.from_bottom_px, clip.width_px, clip.height_px);
        gl.use_program(self.copy);
        let cp = self.copy.unwrap();
        let cu = |n: &str| gl.get_uniform_location(cp, n);
        gl.uniform_1_i32(cu("prev").as_ref(), 0);
        gl.uniform_1_i32(cu("mode").as_ref(), eng.is_some() as i32);
        if let Some(e) = eng {
            for (i, c) in e.pal.iter().enumerate() {
                gl.uniform_3_f32(cu(&format!("c{}", i)).as_ref(), c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0);
            }
            gl.uniform_1_f32(cu("hue").as_ref(), f.t * e.hue * std::f32::consts::TAU);
        }
        gl.bind_texture(glow::TEXTURE_2D, Some(s.tex[s.cur]));
        gl.draw_arrays(glow::TRIANGLES, 0, 3);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.bind_vertex_array(None);
        gl.use_program(None);
        gl.enable(glow::BLEND);
    }
}

/// Draw a GPU preset into `rect` (slot: 0 full screen, 1 Now Playing, 2 the deck, 3 the deck's playlist bar).
pub(crate) fn paint_gpu(
    p: &egui::Painter,
    rect: Rect,
    slot: u8,
    preset: usize,
    f: Frame,
    state: &Arc<Mutex<Gl>>,
    terrain: Option<Arc<crate::terrain::Input>>,
) {
    let state = state.clone();
    let cb = egui_glow::CallbackFn::new(move |info, painter| {
        if let Ok(mut g) = state.lock() {
            match &terrain {
                Some(inp) => {
                    let ok = unsafe { g.terrain.get_or_insert_with(Default::default).paint(painter.gl(), &info, slot, inp) };
                    // (graphics too old for the 3D worlds: a plain visual instead of nothing)
                    if !ok {
                        unsafe { g.draw(painter.gl(), &info, slot, find("Down the Drain"), &f) };
                    }
                }
                None => unsafe { g.draw(painter.gl(), &info, slot, preset, &f) },
            }
        }
    });
    p.add(egui::PaintCallback { rect, callback: Arc::new(cb) });
}

/// The wave in `rect` (a plain visual instead if the graphics are too old for it).
fn paint_wave(p: &egui::Painter, rect: Rect, slot: u8, f: Frame, t: f64, state: &Arc<Mutex<Gl>>) {
    let state = state.clone();
    let cb = egui_glow::CallbackFn::new(move |info, painter| {
        if let Ok(mut g) = state.lock() {
            let ok = unsafe { g.wave.get_or_insert_with(Default::default).paint(painter.gl(), &info, slot, t) };
            if !ok {
                unsafe { g.draw(painter.gl(), &info, slot, find("Down the Drain"), &f) };
            }
        }
    });
    p.add(egui::PaintCallback { rect, callback: Arc::new(cb) });
}

// ------------------------------------------------------------------------------------------------ the screensavers
/// 3D Pipes: pipes growing through a box of cells, a ball at every bend, clearing and starting over when full.
pub(crate) struct Pipes {
    used: std::collections::HashSet<(i32, i32, i32)>,
    segs: Vec<([i32; 3], [i32; 3], usize, bool)>,
    heads: Vec<([i32; 3], usize, usize, bool)>,
    acc: f32,
    fade: f32,
    seed: u64,
}

const NX: i32 = 12;
const NY: i32 = 8;
const NZ: i32 = 8;
const PIPE_COLS: [[u8; 3]; 7] =
    [[216, 52, 44], [47, 191, 74], [47, 111, 224], [232, 198, 42], [42, 200, 208], [200, 74, 216], [232, 232, 232]];
const DIRS: [[i32; 3]; 6] = [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]];

impl Default for Pipes {
    fn default() -> Self {
        let mut p = Pipes {
            used: Default::default(),
            segs: Vec::new(),
            heads: Vec::new(),
            acc: 0.0,
            fade: 0.0,
            seed: 0x9e3779b97f4a7c15,
        };
        p.reset();
        p
    }
}

impl Pipes {
    fn rnd(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed >> 40) as f32 / 16777216.0
    }
    fn reset(&mut self) {
        self.used.clear();
        self.segs.clear();
        self.heads.clear();
        for _ in 0..3 {
            self.new_pipe();
        }
    }
    fn free(&self, c: [i32; 3]) -> bool {
        c[0] >= 0 && c[1] >= 0 && c[2] >= 0 && c[0] < NX && c[1] < NY && c[2] < NZ && !self.used.contains(&(c[0], c[1], c[2]))
    }
    fn new_pipe(&mut self) {
        for _ in 0..40 {
            let c = [(self.rnd() * NX as f32) as i32, (self.rnd() * NY as f32) as i32, (self.rnd() * NZ as f32) as i32];
            if self.free(c) {
                self.used.insert((c[0], c[1], c[2]));
                // a colour not already on screen, when there is one left
                let taken: Vec<usize> = self.heads.iter().map(|h| h.2).collect();
                let mut col = (self.rnd() * PIPE_COLS.len() as f32) as usize % PIPE_COLS.len();
                for _ in 0..PIPE_COLS.len() {
                    if !taken.contains(&col) {
                        break;
                    }
                    col = (col + 1) % PIPE_COLS.len();
                }
                let d = (self.rnd() * 6.0) as usize % 6;
                self.heads.push((c, d, col, true));
                self.segs.push((c, c, col, true));
                return;
            }
        }
    }
    fn grow(&mut self) {
        for k in 0..self.heads.len() {
            let (c, d, col, alive) = self.heads[k];
            if !alive {
                continue;
            }
            let step = |d: usize| [c[0] + DIRS[d][0], c[1] + DIRS[d][1], c[2] + DIRS[d][2]];
            let mut nd = d;
            if self.rnd() < 0.25 || !self.free(step(d)) {
                let opts: Vec<usize> = (0..6).filter(|&o| self.free(step(o))).collect();
                if opts.is_empty() {
                    self.heads[k].3 = false;
                    continue;
                }
                nd = opts[(self.rnd() * opts.len() as f32) as usize % opts.len()];
            }
            let n = step(nd);
            self.used.insert((n[0], n[1], n[2]));
            self.segs.push((c, n, col, nd != d));
            self.heads[k] = (n, nd, col, true);
        }
        if self.heads.iter().filter(|h| h.3).count() < 3 && self.heads.len() < 8 {
            self.new_pipe();
        }
        if self.segs.len() > 260 || self.heads.iter().all(|h| !h.3) {
            self.fade = 1.0;
        }
    }

    pub(crate) fn draw(&mut self, p: &egui::Painter, r: Rect, dt: f32, bass: f32) {
        p.rect_filled(r, 0.0, Color32::BLACK);
        self.acc += dt * (1.0 + bass * 2.5);
        while self.acc > 0.1 {
            self.acc -= 0.1;
            if self.fade <= 0.0 {
                self.grow();
            }
        }
        let mut alpha = 1.0;
        if self.fade > 0.0 {
            self.fade -= dt * 0.8;
            alpha = self.fade.max(0.0);
            if self.fade <= 0.0 {
                self.fade = 0.0;
                self.reset();
            }
        }
        // a fixed camera a little above and to the side, as the screensaver had
        let (ay, ax) = (0.5f32, 0.35f32);
        let scale = r.height().min(r.width() * 0.62) * 1.35;
        let proj = |c: [i32; 3]| -> (Pos2, f32, f32) {
            let (mut x, mut y, mut z) =
                (c[0] as f32 - (NX - 1) as f32 / 2.0, c[1] as f32 - (NY - 1) as f32 / 2.0, c[2] as f32 - (NZ - 1) as f32 / 2.0);
            (x, z) = (x * ay.cos() + z * ay.sin(), -x * ay.sin() + z * ay.cos());
            (y, z) = (y * ax.cos() - z * ax.sin(), y * ax.sin() + z * ax.cos());
            let f = scale / (z + 16.0);
            (Pos2::new(r.center().x + x * f, r.center().y - y * f), f, z)
        };
        // tubes and balls, far first so near ones cover them; a ball sits a little in front of the tubes it joins,
        // so their cut ends hide inside it, and every pipe's tip is rounded off with a ball of its own width
        enum It {
            Tube(Pos2, f32, Pos2, f32, usize),
            Ball(Pos2, f32, usize),
        }
        let mut items: Vec<(f32, It)> = Vec::new();
        // the nearest tube touching each cell: a ball there is drawn just in front of it, never under a tube's cut end
        let mut near: std::collections::HashMap<[i32; 3], f32> = Default::default();
        for &(a, b, _, _) in &self.segs {
            if a != b {
                let z = (proj(a).2 + proj(b).2) / 2.0;
                for c in [a, b] {
                    let e = near.entry(c).or_insert(f32::MAX);
                    *e = e.min(z);
                }
            }
        }
        let ball_z = |c: [i32; 3], z: f32| near.get(&c).copied().unwrap_or(z).min(z) - 0.05;
        for &(a, b, col, joint) in &self.segs {
            let ((pa, fa, za), (pb, fb, zb)) = (proj(a), proj(b));
            if a != b {
                items.push(((za + zb) / 2.0, It::Tube(pa, 0.3 * fa, pb, 0.3 * fb, col)));
            }
            if joint {
                items.push((ball_z(a, za), It::Ball(pa, 0.3 * fa * 1.3, col)));
            }
        }
        for &(c, _, col, _) in &self.heads {
            let (pc, fc, zc) = proj(c);
            items.push((ball_z(c, zc), It::Ball(pc, 0.3 * fc * 1.06, col)));
        }
        items.sort_by(|a, b| b.0.total_cmp(&a.0));
        let shade = |c: [u8; 3], k: f32| {
            let m = |v: u8| ((v as f32 * k).min(255.0) * alpha) as u8;
            Color32::from_rgb(m(c[0]), m(c[1]), m(c[2]))
        };
        // light from the top left: across a tube (lit side first), a soft rim, a bright streak, the colour, a dark rim
        const ACROSS: [(f32, f32); 7] = [(-1.0, 0.45), (-0.75, 0.95), (-0.4, 1.5), (-0.2, 1.75), (0.15, 1.0), (0.6, 0.6), (1.0, 0.28)];
        let light = Vec2::new(-0.7, -0.7);
        let mut mesh = egui::Mesh::default();
        for (_, it) in items {
            match it {
                It::Tube(a, ra, b, rb, col) => {
                    let c = PIPE_COLS[col];
                    let d = b - a;
                    if d.length() < 0.5 {
                        continue;
                    }
                    let mut n = Vec2::new(-d.y, d.x).normalized();
                    if n.x + n.y > 0.0 {
                        n = -n;
                    }
                    let base = mesh.vertices.len() as u32;
                    for (t, k) in ACROSS {
                        mesh.colored_vertex(a - n * t * ra, shade(c, k));
                        mesh.colored_vertex(b - n * t * rb, shade(c, k));
                    }
                    for i in 0..ACROSS.len() as u32 - 1 {
                        let (p0, p1, q0, q1) = (base + 2 * i, base + 2 * i + 1, base + 2 * i + 2, base + 2 * i + 3);
                        mesh.add_triangle(p0, p1, q1);
                        mesh.add_triangle(p0, q1, q0);
                    }
                }
                It::Ball(at, rr, col) => {
                    // bright off-centre towards the light, dark at its rim
                    let c = PIPE_COLS[col];
                    let ctr = mesh.vertices.len() as u32;
                    mesh.colored_vertex(at + light * rr * 0.35, shade(c, 1.9));
                    const SEG: u32 = 32;
                    for ring in [(0.55f32, 1.15f32), (1.0, 0.3)] {
                        for k in 0..SEG {
                            let ang = k as f32 / SEG as f32 * std::f32::consts::TAU;
                            let dir = Vec2::new(ang.cos(), ang.sin());
                            let towards = (dir.dot(light.normalized()) * 0.5 + 0.5) * 0.5;
                            mesh.colored_vertex(at + dir * rr * ring.0 + light * rr * 0.35 * (1.0 - ring.0), shade(c, ring.1 + towards));
                        }
                    }
                    for k in 0..SEG {
                        let (i0, i1) = (ctr + 1 + k, ctr + 1 + (k + 1) % SEG);
                        mesh.add_triangle(ctr, i0, i1);
                        let (o0, o1) = (i0 + SEG, i1 + SEG);
                        mesh.add_triangle(i0, o0, o1);
                        mesh.add_triangle(i0, o1, i1);
                    }
                }
            }
        }
        p.with_clip_rect(r).add(egui::Shape::mesh(mesh));
    }
}

/// The Windows 3.1 starfield: white stars flying out of the middle, faster when the music hits.
pub(crate) struct Stars {
    s: Vec<[f32; 3]>,
    seed: u64,
}

impl Default for Stars {
    fn default() -> Self {
        let mut st = Stars { s: Vec::new(), seed: 0x2545f4914f6cdd1d };
        for _ in 0..320 {
            let v = [st.rnd() * 2.0 - 1.0, st.rnd() * 2.0 - 1.0, st.rnd()];
            st.s.push(v);
        }
        st
    }
}

impl Stars {
    fn rnd(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed >> 40) as f32 / 16777216.0
    }
    pub(crate) fn draw(&mut self, p: &egui::Painter, r: Rect, dt: f32, bass: f32) {
        p.rect_filled(r, 0.0, Color32::BLACK);
        let sp = 0.25 + bass * 0.6;
        let k = r.width() * 0.25;
        for i in 0..self.s.len() {
            self.s[i][2] -= dt * sp;
            let [x, y, z] = self.s[i];
            let at = Pos2::new(r.center().x + x / z.max(0.01) * k, r.center().y + y / z.max(0.01) * k);
            if z <= 0.02 || !r.contains(at) {
                self.s[i] = [self.rnd() * 2.0 - 1.0, self.rnd() * 2.0 - 1.0, 1.0];
                continue;
            }
            let size = ((1.0 - z) * r.width() / 260.0).max(0.8);
            let a = ((1.0 - z) * 1.4).min(1.0);
            p.rect_filled(Rect::from_center_size(at, Vec2::splat(size)), 0.0, Color32::from_white_alpha((a * 255.0) as u8));
        }
    }
}

// ------------------------------------------------------------------------------------------------ in the app
use crate::{Action, App};

impl App {
    /// This frame's music for the visuals (made once a frame, however many spots show them), and the time since the
    /// last frame.
    fn vis_frame(&mut self, ctx: &egui::Context) -> (Frame, f32) {
        let nr = ctx.cumulative_pass_nr();
        if let Some((n, f, dt)) = self.vis_cache {
            if n == nr {
                return (f, dt);
            }
        }
        let now = ctx.input(|i| i.time);
        let dt = ((now - self.vis_last) as f32).clamp(0.0, 0.1);
        self.vis_last = now;
        let bass = self.vis_cache.map_or(0.0, |c| c.1.bass);
        self.vis_travel += dt * (0.4 + bass * 1.6);
        let mut f = Frame::from_audio(now as f32, self.vis_travel, &self.bands, &self.peaks, &self.viz_wave);
        self.vis_agc.apply(&mut f, dt);
        self.vis_cache = Some((nr, f, dt));
        (f, dt)
    }

    /// The chosen visuals in `r` (slot: 0 full screen, 1 Now Playing, 2 the deck, 3 the playlist bar's).
    pub(crate) fn vis_paint(&mut self, ui: &egui::Ui, r: Rect, slot: u8) {
        let (f, dt) = self.vis_frame(ui.ctx());
        let preset = resolve(self.vis_preset.min(PRESETS.len() - 1), f.t);
        let p = ui.painter().with_clip_rect(r.intersect(ui.clip_rect()));
        match PRESETS[preset].kind {
            Kind::Gpu(..) | Kind::Engine(..) | Kind::Random(..) => paint_gpu(&p, r, slot, preset, f, &self.vis_gl, None),
            Kind::Terrain(world, look) => {
                let inp = self.terrain_input(ui.ctx(), world, look);
                paint_gpu(&p, r, slot, preset, f, &self.vis_gl, Some(Arc::new(inp)))
            }
            Kind::Wave => paint_wave(&p, r, slot, f, ui.ctx().input(|i| i.time), &self.vis_gl),
            Kind::Pipes => self.vis_pipes.draw(&p, r, dt, f.bass),
            Kind::Stars => self.vis_stars.draw(&p, r, dt, f.bass),
        }
        ui.ctx().request_repaint();
    }

    /// What the terrain hears and sees this frame.
    fn terrain_input(&self, ctx: &egui::Context, world: World, theme: Option<usize>) -> crate::terrain::Input {
        let (samples, first, rate, file) = match self.player.viz.lock() {
            Ok(v) => (v.samples.clone(), v.first.clone(), v.rate as f32, v.song.clone()),
            Err(_) => (Vec::new(), Vec::new(), 44100.0, None),
        };
        let ctl = &self.player.ctl;
        let music = file.map(|file| crate::terrain::Music { file, at: ctl.pos() as f64, speed: ctl.speed(), ab: ctl.ab() });
        let song = self.cur_track().map_or(0, |t| t.id as u64);
        // (test switches BACKLINE_3D_LOOK=<n>: that look of the world; BACKLINE_3D_EVERY=<seconds>: a new phase that
        // often, as if the song changed)
        let theme = theme.or_else(|| std::env::var("BACKLINE_3D_LOOK").ok().and_then(|v| v.parse().ok()));
        let song = match std::env::var("BACKLINE_3D_EVERY").ok().and_then(|v| v.parse::<f64>().ok()) {
            Some(s) if s > 0.0 => song.wrapping_add((ctx.input(|i| i.time) / s) as u64),
            _ => song,
        };
        crate::terrain::Input { t: ctx.input(|i| i.time), samples, first, rate, song, world, theme, music }
    }

    /// What the visuals are set to, as "Group: Name" (a random one also says what it is showing now).
    pub(crate) fn vis_name(&self) -> String {
        let p = &PRESETS[self.vis_preset];
        let group = group_title(p.group);
        let mut name = if p.group == p.name { p.name.to_string() } else { format!("{}: {}", group, p.name) };
        let shown = resolve(self.vis_preset, self.vis_cache.map_or(0.0, |c| c.1.t));
        if shown != self.vis_preset {
            name = format!("{}  (now {})", name, PRESETS[shown].name);
        }
        name
    }

    pub(crate) fn vis_menu(&mut self, ui: &mut egui::Ui) {
        // what is picked, first; then the groups (the one holding it marked)
        crate::views::dim_line(ui, "NOW SHOWING", 1.0, crate::pal().ink2);
        crate::views::dim_line(ui, &self.vis_name(), 1.6, crate::pal().ink);
        ui.separator();
        for g in GROUPS {
            let only: Vec<usize> = (0..PRESETS.len()).filter(|&i| PRESETS[i].group == *g).collect();
            if only.len() == 1 {
                let i = only[0];
                let mark = if i == self.vis_preset { "* " } else { "  " };
                if crate::menu_item(ui, &format!("{}{}", mark, PRESETS[i].name)) {
                    self.vis_preset = i;
                    self.dirty = true;
                    ui.close_menu();
                }
                continue;
            }
            let title = group_title(g);
            let mark = if PRESETS[self.vis_preset].group == *g { "* " } else { "  " };
            crate::sub_menu(ui, &format!("{}{}", mark, title), |ui| {
                for (i, p) in PRESETS.iter().enumerate().filter(|(_, p)| p.group == *g) {
                    let mark = if i == self.vis_preset { "* " } else { "  " };
                    if crate::menu_item(ui, &format!("{}{}", mark, p.name)) {
                        self.vis_preset = i;
                        self.dirty = true;
                        ui.close_menu();
                    }
                }
            });
        }
        ui.separator();
        if crate::menu_item(ui, "  Full screen  (G)") {
            self.vis_set_full(!self.vis_full, false);
            ui.close_menu();
        }
        crate::views::dim_line(ui, "SCREENSAVER (NOT WHILE PRACTISING)", 1.0, crate::pal().ink2);
        let idle = [
            (0u32, "Off"),
            (2, "After 2 minutes"),
            (5, "After 5 minutes"),
            (10, "After 10 minutes"),
        ];
        for (m, label) in idle {
            let mark = if self.vis_idle == m { "* " } else { "  " };
            if crate::menu_item(ui, &format!("{}{}", mark, label)) {
                self.vis_idle = m;
                self.dirty = true;
                ui.close_menu();
            }
        }
    }

    /// Full screen visuals on or off (`saver`: started by being idle, so any touch ends it).
    pub(crate) fn vis_set_full(&mut self, on: bool, saver: bool) {
        if on == self.vis_full {
            return;
        }
        self.vis_full = on;
        self.vis_saver = on && saver;
        // (a hidden test copy fills its own window instead: full screen would bring it out in front of you)
        let hidden = crate::hidden_test_copy();
        if on {
            self.vis_was_fs = self.ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            if !hidden {
                self.ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
            }
            self.vis_shown_at = self.ctx.input(|i| i.time);
        } else if !self.vis_was_fs && !hidden {
            self.ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        }
        self.vis_idle_since = std::time::Instant::now();
    }

    /// Going idle while music plays starts the screensaver; any key, click or mouse move counts as not idle.
    pub(crate) fn vis_idle_check(&mut self, ctx: &egui::Context) {
        let busy = ctx.input(|i| {
            i.pointer.delta() != Vec2::ZERO || i.pointer.any_down() || !i.keys_down.is_empty() || i.raw_scroll_delta != Vec2::ZERO
        });
        if busy {
            self.vis_idle_since = std::time::Instant::now();
            // the screensaver ends at a touch (not in its first moment, so the move that started nothing doesn't)
            if self.vis_full && self.vis_saver && ctx.input(|i| i.time) - self.vis_shown_at > 0.5 {
                self.vis_set_full(false, false);
            }
            return;
        }
        // (never while practising: playing along with a loop for minutes isn't stepping away)
        let playing = self.cur.is_some() && !self.paused && !self.stopped && !(crate::PRACTICE && self.practice);
        if self.vis_idle > 0 && playing && !self.vis_full && self.vis_idle_since.elapsed().as_secs() >= self.vis_idle as u64 * 60
        {
            self.vis_set_full(true, true);
        }
        if self.vis_idle > 0 && playing && !self.vis_full {
            ctx.request_repaint_after(std::time::Duration::from_secs(5));
        }
    }

    /// Full screen: the visuals, and when the mouse moves, the song and the preset with arrows to change it.
    pub(crate) fn vis_full_ui(&mut self, ui: &mut egui::Ui, acts: &mut Vec<Action>) {
        let r = ui.max_rect();
        let resp = ui.allocate_rect(r, egui::Sense::click());
        self.vis_paint(ui, r, 0);
        let now = ui.input(|i| i.time);
        if ui.input(|i| i.pointer.delta() != Vec2::ZERO) {
            self.vis_ui_at = now;
        }
        let n = PRESETS.len();
        let (left, right, esc) = ui.input(|i| {
            (i.key_pressed(egui::Key::ArrowLeft), i.key_pressed(egui::Key::ArrowRight), i.key_pressed(egui::Key::Escape))
        });
        if esc || resp.double_clicked() {
            self.vis_set_full(false, false);
            return;
        }
        if left {
            self.vis_preset = (self.vis_preset + n - 1) % n;
            self.vis_ui_at = now;
            self.dirty = true;
        }
        if right {
            self.vis_preset = (self.vis_preset + 1) % n;
            self.vis_ui_at = now;
            self.dirty = true;
        }
        resp.context_menu(|ui| self.vis_menu(ui));
        if self.vis_saver || now - self.vis_ui_at > 3.0 {
            return;
        }
        // the song, top left
        let p = ui.painter();
        if let Some(t) = self.cur_track() {
            let line = format!("{}  -  {}", t.artists_text(), t.title);
            crate::ptext(p, r.min + Vec2::new(28.0, 30.0), egui::Align::Min, &line, 1.6, Color32::from_white_alpha(230));
        }
        // the preset and arrows, bottom middle
        let name = self.vis_name();
        let w = crate::text_w(&name, 1.2) + 30.0;
        let mid = Pos2::new(r.center().x, r.max.y - 44.0);
        let bar = Rect::from_center_size(mid, Vec2::new(w + 120.0, 36.0));
        p.rect_filled(bar, 18.0, Color32::from_black_alpha(170));
        crate::ptext(p, mid, egui::Align::Center, &name, 1.2, Color32::from_white_alpha(235));
        let lr = Rect::from_center_size(Pos2::new(bar.min.x + 24.0, mid.y), Vec2::splat(30.0));
        let rr = Rect::from_center_size(Pos2::new(bar.max.x - 24.0, mid.y), Vec2::splat(30.0));
        for (rect, txt, by) in [(lr, "<", n - 1), (rr, ">", 1)] {
            let h = ui.interact(rect, ui.id().with(("vis_arrow", txt)), egui::Sense::click());
            crate::ptext(
                p,
                rect.center(),
                egui::Align::Center,
                txt,
                1.6,
                if h.hovered() { Color32::WHITE } else { Color32::from_white_alpha(170) },
            );
            if h.clicked() {
                self.vis_preset = (self.vis_preset + by) % n;
                self.dirty = true;
            }
        }
        crate::ptext(
            p,
            Pos2::new(r.center().x, r.max.y - 14.0),
            egui::Align::Center,
            "Arrows change it  ·  right-click for all  ·  Esc or double-click to leave",
            0.9,
            Color32::from_white_alpha(140),
        );
        let _ = acts;
    }
}
