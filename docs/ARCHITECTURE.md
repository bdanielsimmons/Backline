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
  Stored songs are `cache/<track id>.<ext>`; their title, artist, album and cover are written into the file's own tags
  in the background (`cache::know` / `start_tagging`, from the liked list, playlists and songs played), and offline
  the songs are listed from those tags (`cache::tagged`).

## Where things live

| Area | Files |
|---|---|
| App state, actions, messages, library page, main loop | `main.rs` |
| Practice logic (sources, tunes, loops, diary, trainer, stems, metronome, playlists) | `extras.rs` |
| Practice screens (FILES / YT / SC / TUNES / DIARY / LOOPS lists, practice panel, lead sheet) | `ui/views.rs` |
| Library window, sign-in, Tidal search box, EQ dock, presets | `ui/library_win.rs` |
| Window frames and title strips: `title_slot` / `title_right` (controls inside the strip), `page_rect` / `PAGE_PAD` (12 pt margin inside every window), `SKIN_TITLE_H` | `ui/widgets.rs`, `ui/winamp_draw.rs` (`draw_frame`) |
| Skin colours and missing pictures: `soften` (no glare), `backing` (panels stay on their side), borrowed pieces from the generated default skin in `load`; audit all saved skins with the ignored test `audit_skins` (BACKLINE_SKINS=folder) | `data/winamp.rs` |
| Chord grips: GRIPS library (degrees per string), movable placement (`grip_offsets`/`place_grip`), sections (`grip_sections`: must know / open / moveable), drop-voicing groups (`voicing_groups`), the signed-off rules as tests (`chord_audit`) | `data/theory.rs` |
| Chord spelling by letter (`spell_degree`, `scale_names`), parsing chord symbols | `data/theory.rs`, `audio/band.rs` (`parse_chord`) |
| With a skin: side list of places (`side_nav`), window switch icons (`window_icons`), now-playing bar (`now_bar`), page body (`library_body`) | `ui/library_win.rs` |
| Player window, title buttons, queue | `ui/player_win.rs` |
| Now Playing (big cover, lyrics) | `ui/album_view.rs` |
| Floating tools: focus timer, metronome, band, number boxes, field ids `F_*` | `ui/tools.rs` |
| Tidal artist pages: FOLLOW / RADIO (`Action::Follow`, `api.follow_artist`), the bio (`api::parse_bio`, `bio_view` in main.rs), NEW FROM ARTISTS YOU FOLLOW in the library's ARTISTS tab (`api.followed_new`, Tidal's v2 feed) | `data/api.rs`, `main.rs` |
| RECORD (practice panel tab): your input via cpal (`audio/record.rs`), mixed with what the player kept (`tour::mix`, `player::rec_start_heard`) into WAVs under takes/; takes live in `store.takes` (not as tune recordings), ALL TAKES panel; EDIT INFO for files has ALBUM and FILE NAME (rename on disk, `file_moved` migrates the id) | `audio/record.rs`, `ui/record.rs`, `extras.rs` (`rec_toggle`, `save_info`) |
| DIARY tabs ROUTINE / LISTENING / MASTERS: tune of the week from `data/canon.rs` (session tunes by kind, gospel by era; kinds take turns via `store.routine_count`), six steps that open tools set up (`Action::RoutineGo`), done-list per day, listening counts; TRAINER "UP A 4TH - 12 KEYS" (`trainer_keys`, `check_trainer`) | `ui/routine.rs`, `data/canon.rs`, `extras.rs` |
| Files: "Show in Explorer" (`Action::ShowFile`, cached Tidal songs too via `cache::file_of`); a file gone from disk is dropped from every list (`file_gone`, at start-up and when it comes up to play) | `extras.rs`, `main.rs` |
| CHORDS: find, analyzer, voicings, passing chords (tabs in three groups; `chords_tab` numbers stay fixed) | `ui/chords.rs`; triads/moves `ui/triads.rs`; clusters `ui/clusters.rs` |
| CHORDS tab SCALE HARMONY: the chords of a parent scale (major, melodic/harmonic minor, harmonic major) as cards, what to solo with over each (`si_auto` shapes, triad pairs), a vamp sent to the band via `prac_chart` | `ui/harmony.rs` |
| LINES: Slonimsky, scales, the line player (`line_player`: one row - play, loop, tempo, feel, OVER from the shared `ln_over`, favorite; `line_score`) | `ui/lines.rs` (UI), `data/lines.rs` (patterns) |
| LINES improvising tab PENTATONICS & MORE: pentatonic / hexatonic (triad pairs) / diminished / chromatic options families, `family_scales()`, exercises; PENTATONICS (boxes, `positions()` for any scale / notes per string, `neck_view`) | `ui/improv.rs` |
| LINES tab ARPEGGIOS: a chord's notes inside its scale's positions (the chords of a scale, or any chord), exercises, through a tune (guide tones, arpeggios, 3-5-7-9) | `ui/arpeggios.rs` |
| READING (sight reading) | `ui/reading.rs` |
| EAR (PRACTICE): ear-training drills from one menu (chord qualities, intervals, progressions, scale degrees, guide tones, scales & modes, bass note, play it back on a clickable neck); question builders `build`/`ask` (tested per drill), sounds mixed from `band::sequence_sound`/`line_sound`; per-drill record `store.ear` (`EarStats`: answers in play, misses, mix-ups, sessions, `best` run); chord qualities: `CHORDS` table in five `LEVELS`, look-alike sets `ALIKES`, your own set `store.ear_chords`, pool from `chord_pool`, voicings from `build_chord` (required notes, range rules, drop/rootless), wrong-answer hints `chord_hint`/`wrong_hint`, optional chime; finished sessions add "Ear training" time to the diary. The menu (`dropdown_grouped`) heads them TONAL and ATONAL (from `ATONAL` = 8): FREE INTERVALS (`FREE`, SING FIRST, a soft `wash_sound` between questions), CELLS (`TETRACHORDS`, `TRICHORDS` as drill `TRICHORD`), SCALE IN INTERVALS (`EXOTICS`, `build_run`, syllables `degree_syllable`/`fixed_syllable`, your place in `store.ear_atonal`), VERTICALS (`stack`, `stack_symbol`), SIGHT-SING & NEXT NOTE (`atonal_line`, `build_sight`, the day's line); sung ones are self-marked (`ear_self`) | `ui/ear.rs` |
| Music theory (chord notes, names, grips, scales) | `data/theory.rs` |
| Neck and piano drawings | `ui/fretboard.rs` (shared with the phone app) |
| Drawing kit (buttons, tabs, LCD, dropdowns, check boxes, list rows) | `ui/widgets.rs`; text `ui/font.rs` |
| Skinned look: pill buttons (`raised_h`), joined tabs (`segments`), spaced-capital headings (`spaced_caps`), one-line explanations with hover (`para` -> `about_line`); line icons by name `ui/vicon.rs` `named`; reading text in Inter (`assets/Inter-Regular.ttf`), playlist in the skin's font (family "skin", `smooth_fonts` in main.rs); casing `display_case` / `caps_words` in `ui/winamp_draw.rs` |
| Text box (select, Ctrl+A, copy/paste) | `field()` in `ui/views.rs` |
| Skins: palettes `ui/skin.rs`; Backline-drawn Winamp looks `data/genskin.rs`; real Winamp skins `data/winamp.rs` (load, fix-ups), `ui/winamp_draw.rs` (sprites, plates), `ui/winamp_ui.rs` (main window, EQ, playlist, the mini player's deck + EQ + PL stack) |
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
- `BACKLINE_OPEN=<name>` opens a screen at start: practice zoom lists loops groups prog news mini mini_eq_pl mine yt diary
  diarysong pent libfilter minefilter scales countoff eq buttonmap bookmarks triads cluster group export superimpose
  fingerings melodies analyzer voicings harmony arpeggios artist (BACKLINE_ARTIST=<id>) artists record routine ear
  (BACKLINE_EAR=<drill index 0-12> opens a drill with a question answered; the atonal ones are 8-12).
- CHORD MELODIES only: `BACKLINE_CM_SCROLL=<px>` opens the bars scrolled down; `BACKLINE_CM_PICK=<kind>` opens the
  first move of that kind (its index in `KINDS`) in the panel.
- Unit tests: `cargo test --release` in `backline/`.
