# Backline: how the code is laid out

A map for finding things fast. Every source file starts with a `//!` summary; this page ties them together.

## The shape of it

- **One app struct.** `App` (in `backline/src/main.rs`) holds all state. `App::new` (~line 2400) builds it and reads
  `settings.json`; `save_settings` (~3350) writes it back. A new setting = a field in `App`, its default in `new`,
  one line in `save_settings`, one line reading `st["name"]` in `new`.
- **Actions.** UI code never changes state directly for anything non-trivial: it pushes an `Action` (enum in
  `main.rs`) and `App::apply` (~4160) or `extras.rs` handles it. Practice-side actions live in `extras.rs`.
- **Messages.** Background threads send a `Msg` (enum in `main.rs`) on `app.tx`; the main loop drains them
  (`main.rs`, then `drain_extra` in `extras.rs`).
- **Saved data.** Settings (window, skin, toggles) in `settings.json`; everything you build up (tunes, loops,
  playlists, diary days, hearts, YouTube/SoundCloud lists) in `library.json` via `data/store.rs` (`Store`). Both under
  `%APPDATA%\Backline` (`api::config_dir()`; `BACKLINE_HOME` overrides it for test profiles).

## Where things live

| Area | Files |
|---|---|
| App state, actions, messages, library page, main loop | `main.rs` |
| Practice logic (sources, tunes, loops, diary, trainer, stems, metronome, playlists) | `extras.rs` |
| Practice screens (FILES / YT / SC / TUNES / DIARY / LOOPS lists, practice panel, lead sheet) | `ui/views.rs` |
| Library window, sign-in, Tidal search box, EQ dock, presets | `ui/library_win.rs` |
| Player window, title buttons, queue | `ui/player_win.rs` |
| Now Playing (big cover, lyrics) | `ui/album_view.rs` |
| Floating tools: focus timer, metronome, band, number boxes, field ids `F_*` | `ui/tools.rs` |
| CHORDS: find, analyzer, voicings, passing chords (tabs in three groups; `chords_tab` numbers stay fixed) | `ui/chords.rs`; triads/moves `ui/triads.rs`; clusters `ui/clusters.rs` |
| CHORDS tab SCALE HARMONY: the chords of a parent scale (major, melodic/harmonic minor, harmonic major) as cards, what to solo with over each (`si_auto` shapes, triad pairs), a vamp sent to the band via `prac_chart` | `ui/harmony.rs` |
| LINES: Slonimsky, scales, the line player (`line_player`: one row - play, loop, tempo, feel, OVER from the shared `ln_over`, favorite; `line_score`) | `ui/lines.rs` (UI), `data/lines.rs` (patterns) |
| LINES improvising tab PENTATONICS & MORE: pentatonic / hexatonic (triad pairs) / diminished / chromatic options families, `family_scales()`, exercises; PENTATONICS (boxes, `positions()` for any scale / notes per string, `neck_view`) | `ui/improv.rs` |
| LINES tab ARPEGGIOS: a chord's notes inside its scale's positions (the chords of a scale, or any chord), exercises, through a tune (guide tones, arpeggios, 3-5-7-9) | `ui/arpeggios.rs` |
| READING (sight reading) | `ui/reading.rs` |
| Music theory (chord notes, names, grips, scales) | `data/theory.rs` |
| Neck and piano drawings | `ui/fretboard.rs` (shared with the phone app) |
| Drawing kit (buttons, tabs, LCD, dropdowns, check boxes, list rows) | `ui/widgets.rs`; text `ui/font.rs` |
| Text box (select, Ctrl+A, copy/paste) | `field()` in `ui/views.rs` |
| Skins: palettes `ui/skin.rs`; Backline-drawn Winamp looks `data/genskin.rs`; real Winamp skins `data/winamp.rs` (load, fix-ups), `ui/winamp_draw.rs` (sprites, plates), `ui/winamp_ui.rs` (main window, EQ) |
| Audio engine (decode, loop, speed, pitch, EQ, output devices) | `audio/player.rs`, `audio/decode.rs` |
| Synth sounds (metronome, band, chords and lines played back, count-in) | `audio/band.rs` |
| Tidal API, offline switch, liked-songs copy | `data/api.rs` |
| Song cache (kept songs) | `data/cache.rs` |
| Guitar Pro files (.gp: bars, notes with string/fret, key and time, chord names and grids, the recording inside) | `data/gp.rs` |
| Transcriptions: Backline's own shareable format (JSON, one per tune, in `transcriptions/`), kept moves (`moves.json`) | `data/transcription.rs` |
| CHORD MELODIES (PRACTICE): SONGS - bars written out (`draw_bars`, `bar_items`: stems, beams, rests, ties, key and time signatures), moves spotted (`suggest`), the pinned panel (`cm_focus`, `move_steps`, `focus_dots`); MOVES - the ones kept | `ui/melodies.rs` |
| Files, YouTube/SoundCloud (yt-dlp), playlist files | `data/sources.rs` |
| Self-update | `data/update.rs` |

## Shared with the phone app

`android/src/lib.rs` pulls these desktop files in with `#[path]`: `data/api.rs`, `audio/band.rs`, `data/chart.rs`,
`audio/decode.rs`, `data/meta.rs`, `audio/player.rs`, `data/theory.rs`, `audio/tuning.rs`, `ui/font.rs`,
`ui/fretboard.rs`, `ui/icons.rs`, `ui/skin.rs`, `ui/vicon.rs`, `ui/viz.rs`, `ui/widgets.rs`, `data/genskin.rs`,
`data/winamp.rs`, `ui/winamp_draw.rs`. A change to one of these must still build for Android.

## Adding things

- **A tab under LINES / CHORDS:** add its name to the `tab_row` list in that file and a branch for its index.
- **A text box:** reserve an `F_*` id in `ui/tools.rs`, call `views::field(ui, &mut self.ed, F_ID, &mut text, rect, hint, multi)`.
- **A saved per-song thing:** a field on the relevant `store` struct with `#[serde(default)]` so old files still load.

## Testing a build

- Test copies use a separate profile: `BACKLINE_HOME=<folder>`; they open off-screen, never take focus, and make no sound.
  Add `BACKLINE_SHOW=1` to open one in front of you, with sound, to try it by hand.
- `BACKLINE_OPEN=<name>` opens a screen at start: practice zoom lists loops groups prog news mini mine yt diary
  diarysong pent libfilter minefilter scales countoff eq buttonmap bookmarks triads cluster group export superimpose
  fingerings melodies analyzer voicings harmony arpeggios.
- CHORD MELODIES only: `BACKLINE_CM_SCROLL=<px>` opens the bars scrolled down; `BACKLINE_CM_PICK=<kind>` opens the
  first move of that kind (its index in `KINDS`) in the panel.
- Unit tests: `cargo test --release` in `backline/`.
