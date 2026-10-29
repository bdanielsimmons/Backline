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
| LOOPS page: songs on the left, the one picked on the right (`loops_view`; narrow windows: list, then one song with < ALL SONGS); `spot_data` gathers songs, loops, bookmarks; `loops_detail` draws the map (`song_wave`) and rows; SHOW / ORDER / export in the "..." menu (`loops_controls`); `Action::PlayAt` plays from a bookmark | `ui/views.rs` |
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
| DIARY tabs ROUTINE / LISTENING: tune of the week from `data/canon.rs` (session tunes by kind, gospel by era; kinds take turns via `store.routine_count`), six steps that open tools set up (`Action::RoutineGo`), done-list per day, listening counts; TRAINER "UP A 4TH - 12 KEYS" (`trainer_keys`, `check_trainer`) | `ui/routine.rs`, `data/canon.rs`, `extras.rs` |
| DIARY tab MY PLAN (only when `plan.json` is in the config folder; private, never committed): weeks/days/blocks from the file, today's blocks on a timer (`PlanState`, `plan_tick` chimes and moves on), done blocks in `plan_log.json`; ahead/behind tips per block (`Block.ahead`/`behind`), GUIDE TONES AND ARPEGGIOS on the neck from each week's `changes` (`chord_parts`, `chord_dots`, walk from `arpeggios::tune_line`), RELOAD, KEEPERS (`Keeper`, `keepers_today`, private `plan_keepers.json`; fun slot adds, Review plays two a day), a song of the week (`Week.song`), PROGRESS (`plan_minutes`, `streak`; ticked blocks credit the diary as "My Plan: <block>" via `plan_credit`), UP A RUNG / NEXT STEP buttons that move a block along (`Moves` in private `plan_moves.json`, `fill_moved`); BACKLINE_OPEN=plan | `ui/plan.rs`, `extras.rs` |
| Files: "Show in Explorer" (`Action::ShowFile`, cached Tidal songs too via `cache::file_of`); a file gone from disk is dropped from every list (`file_gone`, at start-up and when it comes up to play) | `extras.rs`, `main.rs` |
| CHORDS: find, analyzer, voicings, passing chords, PROGRESSIONS (tabs in three groups, `GROUPS` in `chords_view`, PROGRESSIONS under HARMONY; `chords_tab` numbers stay fixed; 8 = PROGRESSIONS, `progressions_page` in `ui/lines.rs`, one row of dropdowns, family in `prog_fam`) | `ui/chords.rs`; triads/moves `ui/triads.rs`; clusters `ui/clusters.rs` |
| CHORDS tab SCALE HARMONY: the chords of a parent scale (major, melodic/harmonic minor, harmonic major) as cards, what to solo with over each (`si_auto` shapes, triad pairs), a vamp sent to the band via `prac_chart` | `ui/harmony.rs` |
| LINES: tabs in three groups (PATTERNS / SCALES / OVER CHORDS, `GROUPS` in `lines_view`; `ln_tab` numbers stay fixed); Slonimsky, scales, the line player (`line_player`: one row - play, loop, tempo, feel, OVER from the shared `ln_over`, favorite; `line_score`) | `ui/lines.rs` (UI), `data/lines.rs` (patterns) |
| LINES improvising tab PENTATONICS & MORE: pentatonic / hexatonic (triad pairs) / diminished / chromatic options families, `family_scales()`, exercises; PENTATONICS (boxes, `positions()` for any scale / notes per string, `neck_view`) | `ui/improv.rs` |
| LINES tab ATONAL (after Sandole): YOUR LINES from the private `atonal_lines.json` in Backline's folder (never in the repo; format documented on `AtFile`; notes as written, `unsure` marked in `ear::acc_col`, fixed by clicking a note: -/+ or the neck, delete, add; saved at once), GENERATED LINES (`alternate_triads`, `joined_scale`/`tetrachord_line`, `interval_cycle`), ANALYSIS (`interval_name`, `trichords`, `tetrachords`, `scales_holding` over `scale_list`, `syllables`); state in `App.at` (`AtState`); the score with marked notes and a click-to-select is `line_score_marked` in `ui/lines.rs` | `ui/atonal.rs` (the generators and fingering in `ui/atonal_core.rs`, shared with the phone) |
| LINES tab ARPEGGIOS: a chord's notes inside its scale's positions (the chords of a scale, or any chord), exercises, through a tune (guide tones, arpeggios, 3-5-7-9) | `ui/arpeggios.rs` |
| READING (sight reading) | `ui/reading.rs` |
| EAR (PRACTICE): ear-training drills from one menu (chord qualities, intervals, progressions, scale degrees, guide tones, scales & modes, bass note, play it back on a clickable neck); question builders `build`/`ask` (tested per drill), sounds mixed from `band::sequence_sound`/`line_sound`; per-drill record `store.ear` (`EarStats`: answers in play, misses, mix-ups, sessions, `best` run); chord qualities: `CHORDS` table in five `LEVELS`, look-alike sets `ALIKES`, your own set `store.ear_chords`, pool from `chord_pool`, voicings from `build_chord` (required notes, range rules, drop/rootless), wrong-answer hints `chord_hint`/`wrong_hint`, optional chime; finished sessions add "Ear training" time to the diary. The menu (`dropdown_grouped`) heads them TONAL and ATONAL (from `ATONAL` = 8): FREE INTERVALS (`FREE`, SING FIRST, a soft `wash_sound` between questions), CELLS (`TETRACHORDS`, `TRICHORDS` as drill `TRICHORD`, `prime_form`), SCALE IN INTERVALS (`EXOTICS`, `build_run`, syllables `degree_syllable`/`fixed_syllable`, your place in `store.ear_atonal`), VERTICALS (`stack`, `stack_symbol`), SIGHT-SING & NEXT NOTE (`atonal_line`, `build_sight`, the day's line); sung ones are self-marked (`ear_self`) | `ui/ear.rs` |
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
| Packs for a friend: plan, books, atonal lines, chord melodies, loops & playlists in one zip (`.backline`); made in PREFERENCES > SHARE WITH A FRIEND, opened by dropping it on the window (`handle_paths`) or OPEN A PACK; `pack_panel` ticks the pieces, `PackUnlock` puts them in place | `data/pack.rs` |
| Visuals: MilkDrop / WMP-style presets as GLSL shaders run through egui_glow paint callbacks (feedback ping-pong per slot: 0 full screen, 1 Now Playing, 2 deck, 3 playlist bar), 3D Pipes and Starfield drawn by egui; `PRESETS` / `GROUPS`, `vis_paint`, `vis_full_ui` (G, Esc), `vis_menu`, idle screensaver `vis_idle_check`; deck visualizer mode 3; `BACKLINE_OPEN=visuals` with `BACKLINE_VIS=<preset>` | `ui/visuals.rs` |
| Note view (the waveform's NOTES chip): the song by pitch, E1 to E6, 50 a second, from the stems switched on when split (`analyse`, `from_stems`); drawn by `notes_strip` in `ui/views.rs`, made by `start_notes` | `audio/notes.rs` |
| Stems: splitting (HT-Demucs), the split songs (`is_split` for the row mark set by `row_stems`, MY TRACKS > WITH STEMS), the time/size guess before a split (`time_guess`, `size_guess`) | `audio/stems.rs` |
| Guitar Pro files (.gp: bars, notes with string/fret, key and time, chord names and grids, the recording inside) | `data/gp.rs` |
| Transcriptions: Backline's own shareable format (JSON, one per tune, in `transcriptions/`), kept moves (`moves.json`) | `data/transcription.rs` |
| CHORD MELODIES (PRACTICE): SONGS - bars written out (`draw_bars`, `bar_items`: stems, beams, rests, ties, key and time signatures), moves spotted (`suggest`), the pinned panel (`cm_focus`, `move_steps`, `focus_dots`); MOVES - the ones kept | `ui/melodies.rs` |
| Files, YouTube/SoundCloud (yt-dlp), playlist files | `data/sources.rs` |
| Self-update | `data/update.rs` |

## Shared with the phone app

`android/src/lib.rs` pulls these desktop files in with `#[path]`: `data/api.rs`, `audio/band.rs`, `data/chart.rs`,
`audio/decode.rs`, `data/meta.rs`, `audio/player.rs`, `data/theory.rs`, `audio/tuning.rs`, `ui/font.rs`,
`ui/fretboard.rs`, `ui/icons.rs`, `ui/skin.rs`, `ui/vicon.rs`, `ui/viz.rs`, `ui/widgets.rs`, `data/genskin.rs`,
`data/winamp.rs`, `ui/winamp_draw.rs`, `data/cache.rs`, `data/lines.rs`, and the pure parts split out of desktop screens:
`ui/atonal_core.rs` (generated lines, fingering, line analysis), `ui/music_data.rs` (scales, intervals, cells, exotic
scales, syllables), `ui/triad_core.rs` (voice leading, bass under a grip, grip names, numerals), `ui/si_core.rs`
(Superimpose shapes, scales, suggestions), `ui/staff.rs` (clef, accidentals, chord staff). A change to one of these
must still build for Android.

The phone's screens: a bottom bar Music / Play / Practice / Chords / Lines (`bottom_bar`, `tab_icon`), no top bar (each
screen titled by `screen`, a gear opens `settings_ui`), the now-playing bar (`now_bar`) and Now Playing (`now_screen`:
cover, LCD, seek, shuffle / repeat / like / speed / sleep / volume) in `android/src/lib.rs`. Music (`library.rs`): My
tracks (liked, playlists on the phone and Tidal, albums, artists, kept), Home, Search, Tidal pages (`page_ui`), files
(`walk`, tags, covers, playlist files) and YouTube. The song menu, playlists on the phone, the queue's place
(`where.json`, not `session.json`, which is the Tidal sign-in) and edit info in `menu.rs`. Practice (`practice.rs`):
the wave (zoom, drag a loop), loop / trainer (12 keys) / pitch (tuning, WAV) / marks. Chords (`chords.rs`): picker
grid, Oolimo neck (`chord_neck_view`, grouped voicings, staff), analyzer (saved grips), triads (map, voice leading,
over a bass). Metronome (`metro.rs`, with the track: follow, lock). Lines (`lines_ui.rs`): atonal lines and
Superimpose. Plain text everywhere (`font::set_modern(true)`); the pixel font only on Now Playing's LCD.

## Adding things

- **A tab under LINES / CHORDS:** add it to a group in that file's `GROUPS` (a new number) and a branch for its number.
- **A text box:** reserve an `F_*` id in `ui/tools.rs`, call `views::field(ui, &mut self.ed, F_ID, &mut text, rect, hint, multi)`.
- **A saved per-song thing:** a field on the relevant `store` struct with `#[serde(default)]` so old files still load.

## Testing a build

- Test copies use a separate profile: `BACKLINE_HOME=<folder>`; they open off-screen, never take focus, and make no sound.
  Add `BACKLINE_SHOW=1` to open one in front of you, with sound, to try it by hand.
- `BACKLINE_OPEN=<name>` opens a screen at start: practice trainer pitch stems zoom lists loops loopsong groups prog news mini mini_eq_pl mine yt diary listening
  diarysong pent libfilter minefilter scales countoff eq buttonmap bookmarks triads cluster group export superimpose
  fingerings melodies analyzer voicings harmony arpeggios artist (BACKLINE_ARTIST=<id>) artists record routine ear
  (BACKLINE_EAR=<drill index 0-12> opens a drill with a question answered; the atonal ones are 8-12) atonal
  (LINES > ATONAL; BACKLINE_AT_VIEW=1 the generated lines, BACKLINE_AT_SEL=<note> a note open to fix). With
  `lines`, BACKLINE_LN_TAB=<0-5> opens that LINES tab.
- CHORD MELODIES only: `BACKLINE_CM_SCROLL=<px>` opens the bars scrolled down; `BACKLINE_CM_PICK=<kind>` opens the
  first move of that kind (its index in `KINDS`) in the panel.
- Unit tests: `cargo test --release` in `backline/`.
