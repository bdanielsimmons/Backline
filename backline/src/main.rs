#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]
#![recursion_limit = "512"]

// Folders: audio/ = playback and sound analysis, data/ = Tidal, files, library and updates, ui/ = everything drawn.
// The modules keep flat names (crate::player, crate::views...) so the folders are purely for reading.
// Here: the app's state (`App`), the enums everything speaks in (`Action`, `Msg`, `Sec`), the frame (`update`),
// playing and the queue. Beside it: settings.rs, actions.rs (actions carried out), messages.rs (what comes back from
// the background), extras.rs (the practice logic), test_hooks.rs. docs/ARCHITECTURE.md maps the rest.

mod actions;
mod extras;
mod messages;
mod settings;
mod test_hooks;

#[path = "audio/band.rs"]
mod band;
#[path = "audio/decode.rs"]
mod decode;
#[path = "audio/intro.rs"]
mod intro;
#[path = "audio/media.rs"]
mod media;
#[path = "audio/notes.rs"]
mod notes;
#[path = "audio/player.rs"]
mod player;
#[path = "audio/record.rs"]
mod record;
#[path = "audio/stems.rs"]
mod stems;
#[path = "audio/tour_song.rs"]
mod tour_song;
#[path = "audio/tuning.rs"]
mod tuning;

#[path = "data/api.rs"]
mod api;
#[path = "data/cache.rs"]
mod cache;
#[path = "data/canon.rs"]
mod canon;
#[path = "data/chart.rs"]
mod chart;
#[path = "data/genskin.rs"]
mod genskin;
#[path = "data/lines.rs"]
mod lines;
#[path = "data/meta.rs"]
mod meta;
#[path = "data/pack.rs"]
mod pack;
#[path = "data/search.rs"]
mod search;
#[path = "data/sources.rs"]
mod sources;
#[path = "data/store.rs"]
mod store;
#[path = "data/sync.rs"]
mod sync;
#[path = "data/theory.rs"]
mod theory;
#[path = "data/update.rs"]
mod update;
#[path = "data/winamp.rs"]
mod winamp;

#[path = "ui/frame_win.rs"]
mod frame_win;
#[path = "ui/fretboard.rs"]
mod fretboard;
#[path = "ui/widgets.rs"]
mod widgets;
#[path = "ui/winamp_draw.rs"]
mod winamp_draw;
#[allow(unused_imports)]
pub(crate) use widgets::*;
#[path = "ui/album_view.rs"]
mod album_view;
#[path = "ui/app_look.rs"]
mod app_look;
#[path = "ui/arpeggios.rs"]
mod arpeggios;
#[path = "ui/atonal.rs"]
mod atonal;
#[path = "ui/atonal_core.rs"]
mod atonal_core;
#[path = "ui/chrome.rs"]
mod chrome;
#[path = "ui/chords.rs"]
mod chords;
#[path = "ui/clusters.rs"]
mod clusters;
#[path = "ui/credits.rs"]
mod credits;
#[path = "ui/peer_files.rs"]
mod peer_files;
#[path = "ui/new_playlist.rs"]
mod new_playlist;
#[path = "ui/ear.rs"]
mod ear;
#[path = "ui/font.rs"]
mod font;
#[path = "data/gp.rs"]
#[allow(dead_code)]
mod gp;
#[path = "ui/harmony.rs"]
mod harmony;
#[path = "ui/help.rs"]
mod help;
#[path = "ui/history.rs"]
mod history;
#[path = "ui/icons.rs"]
mod icons;
#[path = "ui/images.rs"]
mod images;
#[path = "ui/improv.rs"]
mod improv;
#[path = "ui/layout.rs"]
mod layout;
#[path = "ui/library_win.rs"]
mod library_win;
#[path = "ui/lines.rs"]
mod lines_ui;
#[path = "ui/looks.rs"]
mod looks;
#[cfg(target_os = "macos")]
#[path = "ui/macmenu.rs"]
mod macmenu;
#[path = "ui/melodies.rs"]
mod melodies;
#[path = "ui/music_data.rs"]
mod music_data;
#[path = "ui/onscreen.rs"]
mod onscreen;
#[path = "ui/palette.rs"]
mod palette;
#[path = "ui/plan.rs"]
mod plan;
#[path = "ui/player_win.rs"]
mod player_win;
#[path = "ui/popout.rs"]
mod popout;
#[path = "ui/prefs.rs"]
mod prefs;
#[path = "ui/reading.rs"]
mod reading;
#[path = "ui/record.rs"]
mod record_ui;
#[path = "ui/routine.rs"]
mod routine;
#[path = "ui/si_core.rs"]
mod si_core;
#[path = "ui/tp_core.rs"]
mod tp_core;
#[path = "ui/notes_fmt.rs"]
mod notes_fmt;
#[path = "ui/pairs.rs"]
mod pairs;
#[path = "ui/skin.rs"]
mod skin;
#[path = "ui/staff.rs"]
mod staff;
#[path = "ui/tools.rs"]
mod tools;
#[path = "ui/tracks.rs"]
mod tracks;
#[path = "ui/tour.rs"]
mod tour;
#[path = "data/transcription.rs"]
mod transcription;
#[path = "ui/triad_core.rs"]
mod triad_core;
#[path = "ui/triads.rs"]
mod triads;
#[path = "ui/vicon.rs"]
mod vicon;
#[path = "ui/views.rs"]
mod views;
#[path = "ui/visuals.rs"]
mod visuals;
#[path = "ui/terrain.rs"]
mod terrain;
#[path = "ui/wave.rs"]
mod wave;
#[path = "ui/viz.rs"]
mod viz;
#[path = "ui/winamp_ui.rs"]
mod winamp_ui;

use api::{cover_url, Api, Card, GoTo, Kind, Page, Track};
use eframe::egui::{self, Align, Color32, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use font::{ptext, ptext_fit, snap, spx, text_w, thick, wrap};
use icons::*;
use app_look::*;
use chrome::*;
use images::*;
use looks::{backline_look, custom_worn};
use settings::*;
use test_hooks::*;
use search::*;
use tracks::*;
use player::{Cmd, Player};
use skin::*;
use sources::Src;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use viz::viz_draw;
use winamp_ui::skin_menu;

const EQ_PRESETS: [(&str, [f32; 10]); 4] = [
    ("FLAT", [0.0; 10]),
    ("BASS", [6.0, 5.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
    ("TREBLE", [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 2.0, 4.0, 5.0, 6.0]),
    ("ROCK", [4.0, 3.0, 2.0, 0.0, -1.0, -1.0, 1.0, 3.0, 4.0, 5.0]),
];

/// Shown in the player and the log so you can tell which build you are running.
const VERSION: &str = if PRACTICE { "v9 STUDIO" } else { "v9" };
/// Built with `--no-default-features` the practice tools (files, tunes, diary, loops, timer) are left out.
const PRACTICE: bool = cfg!(feature = "practice");

/// Points in the waveform visualizer.
const WAVE_N: usize = 128;

const ROW_H: f32 = 24.0;
/// The space between things on a page, a skin worn (the rows of a table stay flush).
const GAP: f32 = 8.0;
const HEAD_H: f32 = 18.0;

// ------------------------------------------------------------------ messages
enum Msg {
    /// an album's or playlist's songs, fetched to queue them: (its name, the songs, next or at the end)
    CardTracks(String, Result<Vec<Track>, String>, bool),
    /// LISTENING: a record's songs found ("artist - album"), to play
    RecordFound(String, Result<Vec<Track>, String>),
    Code(String, String),
    LoggedIn,
    NeedLogin(String),
    /// Tidal couldn't be reached at all (no internet): carry on with what is on this computer
    NoInternet,
    /// transcriptions brought in: the names, or what went wrong
    CmImported(Result<Vec<String>, String>),
    Page(Page, bool),
    /// an artist followed or unfollowed (their id, followed now?, how it went)
    Followed(String, bool, Result<(), String>),
    /// an album or playlist saved to (true) or removed from your Tidal albums / playlists
    CardSaved(Card, bool, Result<(), String>),
    MoreTracks(u64, Vec<Track>),
    /// the quick look at the library: its newest liked songs, how many in all, and its lists (or why not)
    LibraryHead(Result<(Vec<Track>, usize, Vec<(String, Vec<Card>)>), String>),
    /// the whole library, read again quietly
    LibraryWhole(Page),
    /// all of them read (true), or the reading stopped (false)
    MoreTracksDone(u64, bool),
    Err(String),
    Audio(u64, i64, Vec<u8>),
    PlayErr(u64, String),
    Prefetched(i64, Vec<u8>),
    Img(String, Option<egui::ColorImage>),
    Lyrics(i64, Result<(Vec<(f32, String)>, bool), String>),
    Media(u8),
    LikeFailed(i64, bool, String),
    /// release notes for the WHAT'S NEW panel: (tag, notes), newest first
    News(Result<Vec<(String, String)>, String>),
    /// the sound outputs changed (something plugged in or out, a Bluetooth device connected or gone): the default
    /// one, all of them, and the Bluetooth ones
    Outputs(String, Vec<String>, Vec<String>),
    /// a share file written (where), or read (its contents)
    ShareSaved(Result<String, String>),
    ShareIn(Result<store::Share, String>),
    /// a pack read (to tick what to bring in), and one saved (where)
    PackIn(Result<pack::Opened, String>),
    PackSaved(Result<String, String>),
    /// SYNC: files that came from your other computer (read them again), and library parts (put them in)
    SyncFiles(Vec<String>),
    SyncLibrary(Vec<(String, serde_json::Value)>),
    /// your own Tidal playlists, for adding to: (id, name)
    TidalLists(Result<Vec<(String, String)>, String>),
    /// a song put in a Tidal playlist: what to say
    TidalListDone(Result<String, String>),
    /// a song taken out of one of your Tidal playlists: (the playlist, the song, its nth time there), or why not
    TidalListRemoved(String, i64, usize, Result<(), String>),
    /// a new playlist made on your Tidal account (what to say), shown once the library is loaded again
    TidalListMade(Result<String, String>),
    /// one of your Tidal playlists renamed: (its id, the new name), or why not
    TidalListRenamed(String, String, Result<(), String>),
    /// a song is in that Tidal playlist already: (its id, its name, the song), to ask before adding it again
    TidalListDupe(String, String, Track),
    /// the other computer's song files (or why not); a song copied from it (or why not); the copying done (how many)
    PeerFiles(Result<Vec<sync::Shared>, String>),
    Copied(peer_files::Job, Result<PathBuf, String>),
    CopyEnd(usize),
    /// files / folders chosen or dropped: the audio found (and the folders to watch)
    /// files added: the songs, the folders, and playlist files read (their name and songs)
    Added(Vec<store::Ext>, Vec<String>, Vec<(String, Vec<i64>)>),
    /// a rescan of the watched folders
    Scanned(Vec<store::Ext>),
    YtAdded(Result<store::Ext, String>),
    YtTool(Result<PathBuf, String>),
    Sc(Result<Vec<store::Ext>, String>),
    ScSets(Result<Vec<(String, String)>, String>),
    UpdateFound(Result<Option<update::Info>, String>),
    /// a click in the macOS menu bar
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Menu(String),
    Tuning(i64, Result<Option<(i32, f32)>, String>),
    UpdateStaged(Result<(), String>),
    YtList(String, Result<(String, Vec<store::Ext>), String>),
    ScMeta(store::Ext),
    Wave(i64, Option<Vec<u8>>),
    /// The waveform's fine outline (`sources::wave_data`), for zooming in.
    WaveFine(i64, Vec<[i8; 2]>),
    /// The note view's map for a song and the stems it was made from (0: the whole song).
    Notes(i64, u8, Option<notes::NoteMap>),
    Exported(Result<String, String>),
    /// LOOK UP result: tune name, who wrote it, other versions found on Tidal
    /// A tune looked up: who wrote it, and recordings (from Tidal, or YouTube without it: those come with their links).
    Lookup(String, Result<sources::Work, String>, Vec<(Track, u32)>, Vec<store::Ext>),
    /// who wrote a tune and when, looked up on its own when the tune is opened
    TuneDetails(String, Result<sources::Work, String>),
    Chart(String, Result<(String, String, String), (String, Vec<String>)>),
    Live(String, Result<(String, String, String), (String, Vec<String>)>),
    Beats(i64, Option<band::BeatInfo>),
    /// the beat worked out in the background as a song starts (kept for FIND BEAT)
    BeatsBg(i64, Option<band::BeatInfo>),
    /// tempo / key detected from a track's audio
    Meta(i64, meta::Info),
    /// Winamp skins found (the list, whether they add to what is shown)
    WaList(Result<Vec<winamp::WaSkin>, String>, bool),
    /// a Winamp skin fetched and turned into colours
    WaSkin(Result<(winamp::WaSkin, skin::Pal, winamp::Art), String>),
    /// a free playlist font fetched (which one)
    FontReady(usize, Result<Vec<u8>, String>),
    /// the file picked for the opening sound
    IntroFile(Option<String>),
    /// the tour's song, fetched
    TourSong(Result<std::path::PathBuf, String>),
    /// the backing track was saved (its path) or not
    BandSaved(Result<String, String>),
    /// a file's own tags were written (or why not)
    TagsSaved(Result<String, String>),
    /// a track's credits: (role, names)
    Credits(Result<Vec<(String, String)>, String>),
    /// stem separation finished for this track id
    Stems(#[allow(dead_code)] i64, Result<(), String>),
    /// the stem tool (runtime + model) finished downloading
    StemTool(Result<(), String>),
}

/// A page of Backline's own, shown over the library (one at a time; going anywhere in the library leaves it).
#[derive(Clone, Copy, PartialEq)]
enum Side {
    /// BACKLINE: skins and settings
    Backline,
    /// the Winamp skins
    Skins,
    /// the log
    Log,
}

/// Which list the library shows.
#[derive(PartialEq, Clone, Copy)]
enum Sec {
    Tidal,
    Files,
    Yt,
    Sc,
    Tunes,
    Diary,
    Loops,
    Reading,
    Chords,
    Lines,
    Lists,
    /// MY TRACKS: every song you liked, from every source
    Mine,
    /// CHORD MELODIES: transcriptions (SONGS) and the moves in them (MOVES)
    Melodies,
    /// EAR: ear training drills
    Ear,
    /// CHART: the lead sheet (the tune open, or the song playing) and the band, a page of its own
    Chart,
}

impl Sec {
    /// A PRACTICE page (the rest are music: the sources and your lists).
    pub(crate) fn practice(self) -> bool {
        matches!(
            self,
            Sec::Tunes | Sec::Diary | Sec::Loops | Sec::Reading | Sec::Chords | Sec::Lines | Sec::Melodies | Sec::Ear | Sec::Chart
        )
    }
}

/// The sources Backline plays from: (key, name). A song's source is "tidal" for Tidal, otherwise its kind
/// ("file", "sc", "yt"). Adding a source is one more line here.
pub(crate) const SOURCES: [(&str, &str); 4] = [("tidal", "TIDAL"), ("file", "FILES"), ("sc", "SOUNDCLOUD"), ("yt", "YOUTUBE")];

/// Which text field has the keyboard (0 = none), where its caret is, and where a selection starts (it runs from
/// there to the caret).
#[derive(Default)]
struct Ed {
    id: u32,
    cur: usize,
    anchor: Option<usize>,
    /// picked from the box's right-click menu (cut, copy, paste): done as if typed, the next frame
    menu: Option<egui::Event>,
    /// a notes box whose tools (or tag suggestions) were under the pointer last frame: a press on them leaves it being
    /// written in; and this frame's, set while they're drawn
    hold: u32,
    hold_next: u32,
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Tracks,
    /// what you played lately in Backline (cached songs too, which Tidal never hears about)
    Recent,
    Lists,
    Albums,
    Artists,
}

enum Action {
    Open(Card),
    /// follow (true) or unfollow an artist on Tidal: their id
    Follow(String, bool),
    /// keep (true) or drop (false) an album or a playlist in your Tidal albums / playlists
    SaveCard(Card, bool),
    /// RECORD: start a take, or stop and keep it
    RecToggle,
    /// the last take, not listened to yet: keep it (true) or delete it, then start the new one
    RecLast(bool),
    /// play a take back (its id; you alone rather than the mix)
    RecPlay(i64, bool),
    /// a tune's chart on the lead sheet: yours if it is in TUNES, else looked up and shown without adding it
    PreviewTune(String),
    /// LISTENING: play a record (artist, album) from Tidal
    PlayRecord(String, String),
    /// a recommended tune into your TUNES (its chart looked up)
    AddRecommended(String),
    /// ROUTINE: this tune is the week's; a step opened; a step done (or undone) today
    /// ROUTINE: the tune you are working on for a kind (empty: none)
    SetTune(String, String),
    RoutineDone(u8),
    /// TRAINER: through 12 keys (true) or faster
    TrainerKeys(bool),
    /// the song's file, selected in the file manager
    ShowFile(i64),
    /// a file of yours that is no longer on disk: out of every list
    FileGone(i64),
    /// ALL TAKES: open or close it; delete a take (asked first)
    TakesPanel(bool),
    TakeDelete(i64),
    /// an artist's bio opened in full (true) or back to its first lines: their id
    BioOpen(String, bool),
    /// a track's album, artist or radio
    GoTo(i64, GoTo),
    /// show who made a track (id, "artist - title")
    Credits(i64, String),
    /// a track with several artists: ask which one to open
    PickArtist(Vec<(i64, String)>),
    Home,
    Library,
    Search(String),
    Back,
    Forward,
    /// put the windows back the classic way
    /// the queue: 0 shuffle, 1 sort by artist, 2 sort by title, 3 reverse
    QueueOp(u8),
    /// pick the playlist font (index into winamp_ui::PL_FONTS)
    PlFont(usize),
    /// left / right balance, -100 to 100
    Balance(i32),
    /// the Winamp skin browser: open / close, search, load more, wear one
    WaOpen,
    WaSearch,
    WaMore,
    WaApply(winamp::WaSkin),
    /// wear Backline's look n (skin::LOOKS), drawn as a skin
    Look(usize),
    /// your Custom look: its editor (two colours), worn as you pick
    EditLook,
    /// the opening sound: 0 off, 1 Backline's chime, 2 your file; 3 pick a file; 4 play it now
    Intro(u8),
    /// take a skin off the list of ones worn lately
    WaForget(String),
    /// a Winamp skin into your favourites, or out again: (md5, name)
    WaFav(String, String),
    /// play through another sound output (None = the system default)
    SetDevice(Option<String>),
    /// covers as spinning records, or square
    ToggleVinyl,
    Tab(Tab),
    Play(Vec<Track>, usize),
    PlayShuffled(Vec<Track>),
    PlayIndex(usize),
    PlayNext(Track),
    /// every song of an album or playlist, played next (true) or added to the end of the queue
    QueueCard(Card, bool),
    /// these songs (a list's name for the note) into the queue: next (true) or at the end
    QueueTracks(String, Vec<Track>, bool),
    Enqueue(Track),
    Remove(usize),
    ClearQueue,
    PlayBtn,
    PauseBtn,
    StopBtn,
    Toggle,
    Next,
    Prev,
    Seek(f32),
    SeekRel(f32),
    Volume(f32),
    Shuffle,
    Repeat,
    ClearStatus,
    CopyLog,
    ToggleArt,
    ToggleSpec,
    /// Full screen visuals on or off.
    Visuals,
    ToggleLyrics,
    ToggleFullscreen,
    ToggleLike(Track),
    /// move a song in the queue to play right after the one playing / after the songs you queued
    QueueNext(usize),
    QueueLater(usize),
    MoveQueue(usize, usize),
    /// the next of Backline's looks
    Skin,
    Sleep,
    ToggleMini,
    TogglePin,
    ToggleEq,
    TogglePractice,
    /// move the whole A-B loop by its own length: on to the next phrase (true) or back to the one before
    ShiftLoop(bool),
    SetAAt(f32),
    SetBAt(f32),
    LoopToggle,
    LoopClear,
    Speed(u32),
    ToggleCacheView,
    /// the BACKLINE tab: skins, preferences, help, storage, the log
    BacklinePage,
    /// sound these notes (MIDI) as a chord
    PlayNotes(Vec<i32>),
    /// play a kept song (from LOOPS or the diary), and go to this loop of it
    PlaySaved(store::Version, Option<(f32, f32)>),
    /// play a kept song from this point (a bookmark, or a click on its strip in LOOPS)
    PlayAt(store::Version, f32),
    /// forget saved loop `usize` of this song
    ForgetLoop(i64, usize),
    /// rename a saved loop and set its note: (song, which, name, note)
    /// a loop's name, note and group: (song, which, name, note, group)
    EditLoop(i64, usize, String, String, String),
    /// put a loop in a group (empty: no group)
    LoopGroup(i64, usize, String),
    /// chords one after another, this many seconds apart
    PlaySequenceAt(Vec<Vec<i32>>, f32, String),
    /// stop a sequence started with PlaySequenceAt
    StopSequence,
    /// open the editor for a song's title and artist (not Tidal's), and save what was typed
    EditInfo(i64, String, String),
    SaveInfo(i64, String, String),
    /// load your Tidal library again
    RefreshLibrary,
    /// look at a picture big: its link, its title, and whether it is an artist's
    ArtPreview(String, String, bool),
    /// the log, under the BACKLINE tab
    ShowLog,
    ClearCache,
    OpenCache,
    ToggleKeep,
    Logout,
    StartLogin,
    // ---- sources
    Section(Sec),
    AddFiles,
    AddFolder,
    /// the old storage page (kept so its code still has a way in; nothing opens it now)
    #[allow(dead_code)]
    Rescan,
    ForgetFolder(usize),
    AddYt,
    ScGo,
    ScSets,
    ScOpenSet(String),
    CopyLink(String),
    /// a song's "Artist - Title", copied (to paste into a search elsewhere)
    CopyName(String),
    /// a song off this computer: your own file to the Recycle Bin, or the saved copy of a streamed song
    DeleteSong(Track),
    ApplyUpdate,
    ToggleHelp,
    SetMode(u8),
    QuitApp,
    TogglePrefs,
    FindTuning,
    /// save your loops and playlists to a file / bring in a file from another computer
    /// open the WHAT'S NEW panel (notes of the latest versions)
    News,
    /// a bookmark on the song where it is now
    Bookmark,
    ShareExport,
    ShareImport,
    /// the preview's go-ahead: save the export / bring the import in
    ShareSave,
    /// save a pack of the ticked pieces; open one (a file window); put the ticked pieces of the open pack in place
    PackMake,
    /// open a pack from a file (false) or from a folder (true: one unzipped, or the pieces put together by hand)
    PackOpen(bool),
    PackUnlock,
    ShareTake,
    /// put a song in one of your Tidal playlists: (playlist id, its name, the song, again: even if it's in it
    /// already); an empty id makes a new playlist with that name
    TidalListAdd(String, String, Track, bool),
    /// take a song out of one of your own Tidal playlists: (playlist id, the song, its nth time in the list)
    TidalListRemove(String, i64, usize),
    /// open the panel to rename one of your own Tidal playlists: (its id, its name now)
    TidalListRename(String, String),
    /// put a song, or a loop of it (from, to, the loop's name), in one of your Backline playlists (by its place);
    /// again: even if it's in it already (otherwise that is asked first)
    PlaylistAdd(usize, Track, Option<(f32, f32, String)>, bool),
    /// open the New Playlist panel (with this song in it)
    PlaylistNew(Option<Track>),
    /// open the New Playlist panel with these songs in it, named (a radio, a mix): you can change the name;
    /// on Tidal (true) or in Backline
    PlaylistNewFrom(String, Vec<Track>, bool),
    /// open ADD TO PLAYLIST for a song, or a loop of it (from, to, the loop's name)
    PlaylistPicker(Track, Option<(f32, f32, String)>),
    /// copy songs from the other computer to this one (into your files)
    CopyHere(Vec<sync::Shared>),
    /// play a song from the other computer (a copy for now, not added to your files)
    PlayFromThere(sync::Shared),
    /// make the playlist the New Playlist panel describes
    PlaylistMake(new_playlist::NewPl),
    /// open the panel on one of your playlists: its name, description and cover
    PlaylistEdit(usize),
    PlaylistFromQueue,
    PlaylistRemove(usize, usize),
    PlaylistMove(usize, usize, i32),
    PlaylistDelete(usize),
    PlaylistOpen(Option<usize>),
    RemoveList(usize),
    YtKeep(i64),
    ScBrowser,
    ScKeep(i64),
    Offline(bool),
    GetYtDlp,
    RemoveExt(i64),
    /// a song out of a saved SoundCloud / YouTube playlist (by its link), kept out when it's read again
    RemoveFromList(String, i64),
    // ---- tunes
    PickTune(Track),
    AddToTune(usize, Track),
    NewTuneFrom(Track),
    NewTune,
    /// look up who wrote this tune and when (MusicBrainz only, no Tidal search)
    TuneDetails(usize),
    OpenTune(Option<usize>),
    RemoveTune(usize),
    Status(usize, u8),
    PlayTune(usize, bool),
    AddCurrentToTune(usize),
    LookUp(usize),
    // ---- practice
    SaveSection,
    GoSection(usize),
    DeleteSection(usize),
    ToggleLibrary,
    RemoveVersion(usize, usize),
    Pomo,
    /// hold the focus timer, or carry on with it
    PomoHold,
    /// bring in transcriptions: Guitar Pro files (false) or Backline transcription files (true)
    CmImport(bool),
    /// offline mode on (nothing goes out to the internet) or off
    GoOffline(bool),
    TimerPanel,
    FindChart(usize, Option<String>),
    LiveChart(String, Option<String>),
    MetroPanel,
    PlayAlong(String),
    SaveLive,
    Opt(extras::Opt),
    SetKnob(extras::Knob, String),
    ToggleNumerals,
    Knob(extras::Knob, i32),
    StemGet,
    StemSplit,
    StemCancel,
    StemToggle(usize),
    StemSolo(usize),
    StemAll,
    StemClear,
    /// Delete one song's stems.
    StemDelete(i64),
    /// Show one song's stems in Explorer / Finder.
    StemOpen(i64),
    Continue,
    Trainer,
    TapTempo,
    CountIn,
    Chan,
    Transpose(i32),
    Export,
    // ---- diary / chart
    SaveEntry,
    RightTab(u8),
    /// the lead sheet: its page in the library (in focus mode, the queue's window)
    OpenChart,
    ImportIreal(Option<usize>),
}

#[derive(PartialEq, Clone, Copy)]
enum Auth {
    LoggedOut,
    LoggingIn,
    In,
}

#[derive(PartialEq, Clone, Copy)]
enum Repeat {
    Off,
    All,
    One,
}

#[derive(PartialEq, Clone, Copy)]
enum RowState {
    Normal,
    Playing,
    Played,
}

// ---------------------------------------------------------------- random
struct Rng(u64);

impl Rng {
    fn new() -> Rng {
        let n = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
        Rng(n ^ 0x9E37_79B9_7F4A_7C15)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

fn shuffle_vec<T>(v: &mut Vec<T>) {
    let mut r = Rng::new();
    for i in (1..v.len()).rev() {
        let j = (r.next() % (i as u64 + 1)) as usize;
        v.swap(i, j);
    }
}

/// `tracks` reordered randomly, with tracks[first] moved to the front.
fn shuffled_from(tracks: &[Track], first: usize) -> Vec<Track> {
    let mut rest: Vec<Track> = tracks.to_vec();
    if rest.is_empty() {
        return rest;
    }
    let f = rest.remove(first.min(rest.len() - 1));
    shuffle_vec(&mut rest);
    let mut v = Vec::with_capacity(rest.len() + 1);
    v.push(f);
    v.extend(rest);
    v
}

/// A list longer than this is asked about before all of it goes in the queue (an album or a playlist isn't).
const QUEUE_ASK_OVER: usize = 100;

/// A count with commas between the thousands: 5,428.
pub(crate) fn commas(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

// -------------------------------------------------------------------- lyrics
struct Lyrics {
    id: i64,
    /// 0 = loading, 1 = ready, 2 = none available
    status: u8,
    lines: Vec<(f32, String)>,
    synced: bool,
}

// ----------------------------------------------------------------------- app
struct App {
    api: Api,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    ctx: egui::Context,
    images: Images,
    _font_tex: egui::TextureHandle,

    auth: Auth,
    login_code: Option<(String, String)>,
    login_err: String,

    page: Option<Arc<Page>>,
    /// your library as last loaded: MY LIBRARY shows it at once (REFRESH loads it again)
    lib_page: Option<Arc<Page>>,
    /// when the library was last loaded or looked over, and a look going on (it's looked over again, quietly, when
    /// you come back to it after a while: see `library_look`)
    lib_at: Option<Instant>,
    lib_looking: bool,
    /// songs being kept for practice (a download each, once)
    keeping: HashSet<i64>,
    /// MY TRACKS' songs as last made, and what they were made from (made again when that changes)
    #[allow(clippy::type_complexity)]
    mine_cache: Option<((usize, usize, u64, (bool, bool, bool), String, String), Arc<(Vec<Track>, usize, bool, usize)>)>,
    /// the window had the keyboard last frame (coming back to it looks the library over)
    was_focused: bool,
    /// a Winamp skin being fetched and put on (its name), for the little "putting it on" panel
    wa_loading: Option<String>,
    /// the picture being looked at big (link, title, an artist's?)
    art_preview: Option<(String, String, bool)>,
    back: Vec<Arc<Page>>,
    /// pages left with Back, for Forward (cleared when you open something new)
    fwd: Vec<Arc<Page>>,
    /// the credits panel: which track, and its credits once they arrive
    credits: Option<(String, Option<Result<Vec<(String, String)>, String>>)>,
    /// the "which artist?" panel
    artist_pick: Option<Vec<(i64, String)>>,
    serial: u64,
    lib_gen: u64,
    lib_tab: Tab,
    loading: bool,
    search: String,
    search_focus: bool,
    status: String,
    status_err: bool,

    queue: Arc<Vec<Track>>,
    /// Original (un-shuffled) order, kept while shuffle is on so it can be restored.
    orig_queue: Option<Vec<Track>>,
    cur: Option<usize>,
    q_scroll: bool,
    play_gen: u64,
    buffering: bool,
    paused: bool,
    stopped: bool,
    prefer_lossless: bool,
    kbps: u32,
    prefetched: Option<(i64, Vec<u8>)>,
    prefetching: i64,
    player: Player,
    volume: f32,
    shuffle: bool,
    repeat: Repeat,
    seek_drag: Option<f32>,
    /// the practice waveform's zoom (1 = the whole song) and the second its view starts at
    wv_zoom: f32,
    wv_start: f32,
    /// you scrolled the zoomed view away from the playhead: it stays put until you click, zoom or FIT
    wv_free: bool,
    /// the practice waveform's height (1 = normal; you drag it), kept with your settings
    wv_h: f32,
    /// The waveform shows the song by pitch (the note view) instead of loudness.
    wv_notes: bool,
    /// The note view's map: the song, the stems it was made from (0: the whole song), the map once made.
    notes: (i64, u8, Option<Arc<notes::NoteMap>>),
    notes_busy: bool,
    /// The map as pictures, 2048 columns each (made when first drawn).
    notes_tex: Vec<egui::TextureHandle>,
    bands: Vec<f32>,
    peaks: Vec<f32>,
    audio_err_shown: bool,

    art_view: bool,
    show_spec: bool,
    // ---- visuals (ui/visuals.rs)
    vis_gl: Arc<Mutex<visuals::Gl>>,
    vis_preset: usize,
    /// Visuals in Now Playing, in place of the cover.
    vis_np: bool,
    /// Automatic gain and beats for the visuals.
    vis_agc: visuals::Agc,
    vis_full: bool,
    vis_saver: bool,
    vis_was_fs: bool,
    vis_shown_at: f64,
    vis_ui_at: f64,
    /// the visuals' name last shown in Now Playing (a change shows it again)
    vis_seen: String,
    /// Minutes idle (music playing) before the screensaver; 0 off.
    vis_idle: u32,
    vis_idle_since: Instant,
    vis_last: f64,
    vis_travel: f32,
    vis_cache: Option<(u64, visuals::Frame, f32)>,
    vis_pipes: visuals::Pipes,
    vis_stars: visuals::Stars,
    /// The visuals in a window of their own (ui/popout.rs), while it is open: what it shares with the app
    pop: Option<Arc<Mutex<popout::Pop>>>,
    /// where that window was last (screen pixels: left, top, width, height), and whether it stays on top of other
    /// windows
    pop_px: Option<[i32; 4]>,
    pop_pin: bool,
    spec_op: f32,
    spec_h: f32,
    spec_w: f32,
    /// the album view's visualizer in a box: 0 none, 1 a themed box, 2 a thin line
    spec_frame: u8,
    /// with lyrics up, the visualizer stays as wide as the whole view (0: only under the cover, 1: full width)
    spec_lyr: u8,
    /// the visualizer settings panel, open on 0 (the player's screen) or 1 (the album view)
    viz_panel: Option<usize>,
    /// the Winamp skin browser: open, what was searched, the skins found, still loading, no more to load,
    /// and the skin being worn (md5, name)
    /// the BACKLINE page (skins, preferences, help, storage, the log, signing out)
    /// a page of Backline's own over the library, when one is open (the BACKLINE page, Winamp skins, the log)
    side: Option<Side>,
    /// where you have been in the library, for back / forward (see history.rs)
    hist_back: Vec<history::Spot>,
    hist_fwd: Vec<history::Spot>,
    hist_at: Option<history::Spot>,
    wa_q: String,
    wa_list: Vec<winamp::WaSkin>,
    wa_busy: bool,
    wa_end: bool,
    wa_worn: Option<(String, String)>,
    /// Winamp skins worn lately (md5, name), newest first, to go back to one in a click
    wa_recent: Vec<(String, String)>,
    /// your favourite Winamp skins (md5, name), kept until you remove them (their files stay on disk)
    wa_favs: Vec<(String, String)>,
    /// the skins page's tab: 0 browse the museum, 1 favorites, 2 worn lately
    wa_tab: u8,
    /// a skin shown big, before it goes on
    wa_preview: Option<winamp::WaSkin>,
    /// the worn Winamp skin's own pictures, drawn for the player and the equalizer
    wa_art: Option<winamp_ui::WaTex>,
    /// the album view cover's border (right-click the cover): 0 none, 1 subtle, 2 the skin's colour
    art_border: u8,
    /// art view caption: size (0 small, 1 medium, 2 large) and soft black text on a light plate instead of soft white
    cap_size: u8,
    cap_dark: bool,
    /// what Now Playing shows of the song: 0 art and info (with visuals in place of the cover, a small cover fades in
    /// over the words when the mouse moves or a song starts), 1 info only, 2 nothing
    cap_show: u8,
    /// that small cover: how far in (0..1), when it was last called up, for which song; when the choice was changed
    cap_art: f32,
    cap_art_at: f64,
    cap_song: i64,
    cap_mode_at: f64,
    /// the album view's background colours as shown, easing toward the new cover's
    bg_now: Option<[Color32; 2]>,
    /// the sound output chosen in Preferences (None = the system default), and the outputs last found
    out_device: Option<String>,
    /// the system's default output when last looked at (Backline follows it unless you picked one)
    sys_output: String,
    out_list: Vec<String>,
    /// the outputs that are Bluetooth (one of these going away pauses the music)
    out_bt: Vec<String>,
    /// the queue panel open over the album view
    art_queue: bool,
    /// visualizer settings: [0] the small one in the player, [1] the one in the album view
    viz: [viz::VizCfg; 2],
    viz_wave: Vec<f32>,
    show_lyrics: bool,
    fullscreen: bool,
    /// fullscreen was turned on inside the album view, so leaving the view also leaves fullscreen
    fs_by_art: bool,
    art_tilt: Vec2,
    lyrics: Option<Lyrics>,
    lyric_scroll: f32,
    lyric_free: f32,

    liked: HashSet<i64>,
    q_drag: Option<usize>,
    /// how many songs right after the one playing you queued yourself (Play next / Add to queue): they play before
    /// the rest of the album, playlist or radio (staying next in line when you start another), and new ones you add
    /// go after them
    q_manual: usize,
    /// the copies you added (PLAY NEXT / ADD TO QUEUE), by their own number: they leave the queue once playing
    /// moves on past them; the last number given out
    q_added: HashSet<u64>,
    q_next: u64,
    /// a song you queued leaves the queue once playing moves on past it (Preferences)
    q_drop: bool,
    /// MY TRACKS: the sources turned off (by SOURCES key)
    mine_off: Vec<String>,
    /// MY TRACKS shows the songs split into stems instead of the liked ones
    mine_stems: bool,
    /// the metronome's own volume (0..1), apart from the main volume
    mt_vol: f32,
    sleep_at: Option<Instant>,
    sleep_mins: u32,
    mini: bool,
    /// the mini window's size, asked for again a moment after going mini (the title bar coming off can change it)
    mini_size_at: Option<(Instant, Vec2)>,
    /// the full window's equalizer and playlist (show_eq, queue_hidden) while the mini player is on: the mini starts
    /// with both closed and opens its own, and leaving it brings these back
    mini_saved: (bool, bool),
    /// the skin stack the mini window was last sized for: (equalizer, playlist) under the deck
    mini_stack: (bool, bool),
    pinned: bool,
    saved_size: Vec2,
    /// the window was maximized when the mini player came on
    saved_max: bool,
    /// the song is changing because the last one finished or was skipped (a queued copy then leaves the queue)
    advancing: bool,
    show_eq: bool,
    eq_gains: [f32; 10],
    /// the equalizer's preamp, in dB (-12 to +12)
    eq_pre: f32,
    eq_on: bool,
    dirty: bool,
    last_save: Instant,
    last_title: String,

    practice: bool,
    loop_a: Option<f32>,
    loop_b: Option<f32>,
    loop_on: bool,
    speed: u32,
    loops: HashMap<i64, (f32, f32)>,
    menu_t: f32,
    /// the colour new bookmarks get: None for a random one each time
    mark_col: Option<[u8; 3]>,
    keep_cache: bool,
    /// look over your YouTube and SoundCloud songs for ones to keep (at start, and after anything is added)
    keep_sweep: bool,
    cache_stats: (usize, u64),
    cache_at: Instant,

    // ---- sources, tunes, diary (see extras.rs / views.rs)
    store: store::Store,
    store_dirty: bool,
    store_saved: Instant,
    /// SYNC: the code typed to link with another computer; songs brought in here this run (while linked, only these
    /// are kept on their own: ones that came in the sync download when they're played)
    sync_in: String,
    here: HashSet<i64>,
    /// DIARY: the notes being written, and the day they're for
    diary_note: (String, String),
    srcmap: HashMap<i64, Src>,
    sec: Sec,
    /// the list last open in each group (MUSIC / PRACTICE), to come back to it
    last_music: Sec,
    last_practice: Sec,
    ed: Ed,
    files_busy: bool,
    yt_busy: bool,
    yt_msg: String,
    yt_in: String,
    sc_in: String,
    marker_drag: u8,
    pan_off: f32,
    pan_len: f32,
    sc_msg: String,
    sc_busy: bool,
    sc_results: Vec<store::Ext>,
    sc_sets: Vec<(String, String)>,
    sc_cur: Option<String>,
    /// a playlist (by its link) asked about before it is taken off your playlists
    list_ask: Option<String>,
    upd: Option<update::Info>,
    upd_note: String,
    /// when the last update check found you up to date (seconds since 1970; 0: not yet), for "Checked 2 mins ago"
    upd_at: u64,
    upd_state: u8,
    last_active: Instant,
    show_help: bool,
    palette_open: bool,
    palette_q: String,
    /// the palette's text selected whole (Ctrl+A): typing replaces it
    palette_all: bool,
    /// the diary's song opened (the day, the song): the loops worked on, how long
    diary_song: Option<(String, i64)>,
    /// the diary's summary under the day: 0 the week, 1 the month, 2 the year
    diary_span: u8,
    /// reading time counts in the diary's charts and totals
    diary_reading: bool,
    /// the saved loops: the words they're filtered by, how they're sorted (0 newest, 1 name, 2 time spent, 3 song),
    /// and only the starred ones
    loops_filter: String,
    loops_sort: u8,
    palette_sel: usize,
    palette_frame: u32,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    menu_keep: Option<Box<dyn std::any::Any>>,
    fs_cmd_at: Option<Instant>,
    art_op: f32,
    show_prefs: bool,
    auto_restart: bool,
    /// the opening sound: 0 off, 1 Backline's chime, 2 the file below
    intro: u8,
    intro_file: Option<String>,
    /// which of Backline's opening sounds (intro::SOUNDS)
    intro_sound: u8,
    binds: Vec<Option<egui::Key>>,
    rebinding: Option<usize>,
    bind_note: String,
    /// the running tour: (0 general / 1 practice, step)
    tour: Option<(u8, usize)>,
    /// what was showing when the tour began (album view, practice mode, MORE and its tab), put back after
    tour_restore: Option<(bool, bool, bool, u8)>,
    /// what was playing before the tour (it plays its own song), put back when it ends
    tour_play: Option<tour::TourPlay>,
    /// recording a demo of a tour (BACKLINE_DEMO), and the tour's song, the demo library's only track
    demo_rec: Option<tour::DemoRec>,
    demo_track: Option<Track>,
    /// Windows' media controls, showing the song and whether it plays
    media: Option<media::Controls>,
    /// previous / play-pause / next under the taskbar thumbnail (put on a moment after start, once its button exists)
    thumb: Option<media::ThumbBar>,
    thumb_tried: bool,
    tour_seen: bool,
    /// the names under the window icons at the top of the side list
    icon_names: bool,
    /// the very first run: Backline's sleek look goes on once the window is up
    first_look: bool,
    /// the library's own zoom (ctrl + wheel or ctrl +/- over it)
    lib_zoom: f32,
    /// where the library is (ctrl +/- over it zoom it, not the app)
    lib_rect: Rect,
    help_tab: usize,
    /// (track id, cents from A440, confidence) from the tuning check
    tuning: Option<(i64, i32, f32)>,
    tuning_busy: bool,
    upd_checked: Option<Instant>,
    new_pl: String,
    pl_open: Option<usize>,
    /// what the queue was started from (a page's name, as a radio's), to name a playlist made of it
    queue_from: String,
    /// a short message over the library ("Added to ..."), and when it came
    toast: Option<(String, f64)>,
    /// a song about to go in a playlist it's in already: asked about first
    pl_dupe: Option<new_playlist::PlDupe>,
    /// a whole long list about to go in the queue (its name, its songs, next or at the end): asked about first
    queue_ask: Option<(String, Vec<Track>, bool)>,
    /// saving the queue as a playlist when it holds songs you added and a list you were playing: which one
    queue_save: Option<new_playlist::QueueSave>,
    /// ADD TO PLAYLIST, while it's open; what's typed to find a playlist; the side picked last (Tidal or Backline)
    pl_picker: Option<new_playlist::Picker>,
    pl_find: String,
    pl_last_tidal: bool,
    /// songs from the other computer: the copying, its list as shown here, a song whose file is there (asked about),
    /// what to do with such a song (0 ask, 1 play from there, 2 save a copy), whether ALWAYS is ticked, how many files
    /// were last shared with it, whether the last copy was only to play now, and FILES' sources (this computer's
    /// songs, that computer's: either or both)
    copying: Arc<peer_files::Copying>,
    peer: peer_files::PeerView,
    peer_ask: Option<(Track, String)>,
    peer_play: u8,
    peer_always: bool,
    shared_n: usize,
    copying_last_for_now: bool,
    files_there: bool,
    files_here: bool,
    /// your own Tidal playlists (None until asked for), and whether they are being fetched
    tidal_lists: Option<Vec<(String, String)>>,
    /// the New Playlist panel, while open
    new_pl_dlg: Option<new_playlist::NewPl>,
    tidal_lists_busy: bool,
    /// the name typed for a new Tidal playlist (MY LIBRARY > LISTS)
    tidal_new_name: String,
    /// the words your Tidal library is filtered by (empty: all of it)
    lib_filter: String,
    yt_results: Vec<store::Ext>,
    /// the opened playlist's name, and whether its songs are shown (it starts folded up)
    yt_title: String,
    yt_cur: Option<String>,
    offline: bool,
    new_tune: String,
    tune_open: Option<usize>,
    edit_ver: Option<usize>,
    del_arm: bool,
    pick: Option<Track>,
    look_busy: bool,
    look_for: String,
    look_tracks: Vec<(Track, u32)>,
    /// the tune's proper name, guessed from what LOOK UP found: (for which name, how many found, the guess)
    name_guess: (String, usize, Option<String>),
    /// tunes whose guessed name you said KEEP to (their names as kept)
    name_kept: Vec<String>,
    f_tune: String,
    f_mins: String,
    f_bpm: String,
    f_note: String,
    show_form: bool,

    // ---- practice tools
    wave: (i64, Option<Vec<u8>>),
    /// The playing song's fine outline (a low and high per millisecond), held only while it plays.
    wave_fine: (i64, Option<Vec<[i8; 2]>>),
    /// the library hidden: just the player's column (with a skin, the window narrowed to it)
    lib_hidden: bool,
    /// the window's width (and whether it filled the screen) before the library was hidden, to come back to
    lib_shown_w: Option<(f32, bool)>,
    more: bool,
    /// stems: 0 idle, 1 downloading the tool, 2 splitting a track
    stem_busy: u8,
    stem_on: [bool; 4],
    /// focus timer: 0 idle, 1 focus block, 2 break, 3 break over, 4 held (paused, or brought back after a restart)
    pomo: u8,
    /// held: the seconds left and which part it was (1 focus, 2 break)
    pomo_left: u64,
    pomo_was: u8,
    pomo_end: Option<Instant>,
    pomo_focus: u32,
    pomo_break: u32,
    pomo_cycles: u32,
    pomo_n: u32,
    pomo_flash: Option<Instant>,
    timer_open: bool,
    metro_open: bool,
    bpm: u32,
    count_in: u32,
    taps: Vec<Instant>,
    trainer: bool,
    trainer_step: u32,
    trainer_n: u32,
    passes: u32,
    last_wraps: u32,
    /// when the loop last went round (practice time counts only while it keeps doing so)
    last_wrap_at: Option<Instant>,
    /// pitch shift in cents (100 = a semitone)
    semis: i32,
    /// the pitch as typed in its box
    pitch_text: String,
    /// the A you play at (Hz: 440, 442, 432...), and what's typed for it and for the record's A
    my_a: f32,
    my_a_text: String,
    rec_a_text: String,
    chan: u32,
    acc: f32,
    acc_at: Instant,
    last_cap: Instant,
    /// the queue kept with where you are last time (its address: the same queue isn't made again)
    last_q: usize,
    /// the song and stems NOTES last tried (one that couldn't be read isn't tried again each frame)
    notes_tried: (i64, u8),
    sec_name: String,
    pending_seek: Option<f32>,
    /// LOOPS shows every song with a loop (else just this week's)
    loops_all: bool,
    /// LOOPS grouped under their songs (else every loop on its own, newest first)
    /// LOOPS shows: 0 loops and bookmarks, 1 loops, 2 bookmarks
    loops_show: u8,
    /// the song open on the LOOPS page (None: the one playing, else the first)
    loops_pick: Option<i64>,
    /// the loop being renamed / noted in LOOPS (song id, which), and what is typed
    clip_edit: Option<(i64, usize)>,
    /// a bookmark being renamed or given a note on the LOOPS page: (song, which one); its words in clip_name/clip_note
    mark_edit: Option<(i64, usize)>,
    /// where that loop's or bookmark's editor is open: the practice panel (true) or the LOOPS page
    edit_in_practice: bool,
    clip_name: String,
    clip_note: String,
    clip_group: String,
    /// a loop waiting to be deleted from the practice panel (song, which), asked first
    loop_ask: Option<(i64, usize)>,
    /// LOOPS > GROUPS: the group being renamed (and the name typed), the one asked to delete, a new one's name
    grp_edit: Option<String>,
    grp_name: String,
    grp_ask: Option<String>,
    grp_new: String,
    /// LOOPS: shown as each loop (0), by song (1) or by group (2); the new group name box open
    loops_mode: u8,
    /// export / import waiting in its preview panel: (importing, the file's contents)
    share_view: Option<(bool, store::Share)>,
    /// what's ticked in that preview: songs (their loops) and playlists (by name); None until the panel opens
    share_pick: Option<(HashSet<i64>, HashSet<String>)>,
    /// PREFERENCES > SHARE WITH A FRIEND: the pieces left out of a pack (all in to begin with), and loops and playlists in it
    pack_off: Vec<String>,
    pack_loops: bool,
    /// a pack being opened: its pieces, ticked or not
    pack_in: Option<pack::Opened>,
    /// the WHAT'S NEW panel's notes (open while Some), and the last version you saw them for
    news: Option<Vec<(String, String)>>,
    seen_build: u64,
    grp_adding: bool,
    /// LOOPS > GROUPS: the group whose page is open ("" for the loops in no group)
    grp_open: Option<String>,
    sel_anchor: Option<f32>,
    exporting: bool,

    // ---- lead sheet
    rtab: u8,
    chart_tr: i32,
    /// bars picked on the lead sheet by dragging across them: the band loops just these (first, last)
    chart_sel: Option<(usize, usize)>,
    /// the bar a drag across the lead sheet started on
    chart_drag: Option<usize>,
    chart_edit: bool,
    /// the CHORDS tab: which side (find / name), the chord typed, the notes clicked on the neck and the piano,
    /// and guitar shapes already worked out
    chords_tab: u8,
    /// the chord stepped to in WALK THE CHANGES
    vc_step: usize,
    /// the chord finder's funnel: the root and the kind of chord picked
    cf_root: Option<i32>,
    cf_fam: Option<usize>,
    /// BUILD YOUR OWN cluster: how many notes, each step (true = a semitone), the note it starts on
    cb_size: u8,
    cb_steps: [bool; 4],
    cb_lo: i32,
    chord_q: String,
    an_frets: [Option<u8>; 6],
    /// your grip waiting to be deleted (chord key, which), asked to confirm first
    grip_ask: Option<(String, usize)>,
    /// laying down a grip for this chord (key, name as shown), from the finder
    grip_for: Option<(String, String)>,
    /// the analyzer's SAVE GRIP AS panel: open, the name, and the funnel's root / kind / type
    gs_open: bool,
    gs_name: String,
    gs_pick: (usize, usize, usize),
    /// how chord names are written, the details panel, the neck's size, how strong the hover rings are
    chord_sp: theory::Spelling,
    chord_details: bool,
    /// the CLUSTERS tab: key, parent scale, notes in the cluster, string set, the degree on show
    cl_key: i32,
    cl_scale: usize,
    cl_view: u8,
    cl_semi: bool,
    cl_low: i32,
    cl_bass: i32,
    /// FIXED CLUSTER's own bass, and FIXED BASS's own cluster (its lower note), so walking one tab leaves the other be
    fc_bass: i32,
    fb_low: i32,
    cl_changes: String,
    cl_chord: usize,
    /// CLUSTERS: the mode on show in SCALES; the voicing on the neck in BUILD and its name
    cl_mode: usize,
    /// TRIADS / MOVES: the page, key, triad quality, strings (3 and 4 note), the shape picked, start position,
    /// typed changes, the bass under a triad; the move, its dip, mode and stack; a move playing (start, step times)
    tr_page: u8,
    tr_root: i32,
    tr_q: usize,
    tr_set: usize,
    tr_set4: usize,
    tr_sel: usize,
    /// TRIADS > MAP: every note of the triad lit on the whole neck, under the shapes
    tr_whole: bool,
    tr_pos: u8,
    tr_prog: String,
    tr_bass: i32,
    mv_page: u8,
    mv_move: usize,
    mv_dip: bool,
    mv_parent: usize,
    mv_mode: usize,
    mv_stack: u8,
    mv_motion: u8,
    mv_pedal: bool,
    mv_play: Option<(Instant, Vec<f32>)>,
    /// VOICINGS: SHAPES or WALK THE CHANGES
    vc_page: u8,
    /// READING: key, position, rhythm level, bars, time (0 4/4, 1 3/4), tempo, the helps shown, the line, and a
    /// play-through (start, note times, only clicks?, length)
    rd_key: usize,
    rd_pos: usize,
    rd_level: usize,
    rd_bars: usize,
    rd_time: usize,
    rd_bpm: u32,
    rd_tab: bool,
    rd_names: bool,
    rd_map: bool,
    rd_line: Option<crate::reading::Reading>,
    rd_play: Option<(Instant, Vec<(f32, usize)>, bool, f32)>,
    /// reading time: when you last started or finished a line (pauses up to two minutes between lines count), and
    /// the part of a second not yet written to the diary
    rd_active: Option<Instant>,
    rd_acc: f32,
    /// READING > THE BOOK: which tab, the book (read once), what could not be read, the lesson, piece and part on
    /// show, and which piece the tempo was set for
    rd_mode: u8,
    rd_book_loaded: bool,
    rd_book: Vec<crate::reading::Lesson>,
    rd_errs: Vec<String>,
    rd_lesson: usize,
    rd_piece: usize,
    rd_part: usize,
    rd_tempo_for: (usize, usize),
    /// what is playing, to carry on at a new tempo: (part, other part, only clicks, from beat, count-in length, secs a beat)
    rd_ctx: Option<(crate::reading::Reading, Option<crate::reading::Reading>, bool, f32, f32, f32)>,
    rd_bpm_text: String,
    /// a duet shown as both staves
    rd_both: bool,
    /// how far the music is magnified
    rd_zoom: f32,
    /// which reading guide is open
    rd_guide: u8,
    /// the book's markings on the music (fingers, strings, pick strokes...)
    rd_marks: bool,
    /// made-up lines: the position's fingering written over the notes
    rd_fingering: bool,
    cb_frets: [Option<u8>; 6],
    cb_name: String,
    vc_family: u8,
    vc_root: i32,
    vc_type: usize,
    vc_set: usize,
    pass_pair: usize,
    /// LINES: which tab, the Slonimsky division / kind / pattern / first note / direction / octave up
    ln_tab: u8,
    /// PENTATONICS: key, major or minor, the box shown (5 = the whole neck), the blue note and in-between notes,
    /// labels (degrees, notes, fingers), the exercise, its string, and what it is played over
    pn_root: i32,
    pn_box: u8,
    pn_blue: bool,
    pn_between: bool,
    pn_label: u8,
    pn_ex: u8,
    pn_string: u8,
    /// which box counts as box 1 (0: the one with its root on the 6th string, first finger)
    /// NUMBERING, kept for each tab by where position 1's root sits (empty: the usual one)
    pn_num: String,
    /// SCALES: the family the scale list shows (empty: the scale's own)
    pf_fam: String,
    /// SCALES, playing a position: all of it (0), or 1 or 2 octaves from its lowest root
    pf_range: u8,
    pf_num: String,
    ar_num: String,
    /// PENTATONICS & MORE: the family (0 pentatonic, 1 hexatonic, 2 diminished), the scale in it, notes per string
    pn_fam: u8,
    pn_scale: u8,
    pn_nps: u8,
    /// SUPERIMPOSE: the chord (root, quality), suggestions or any shape, the suggestion picked, the shape and its
    /// root (from the chord's root), and what the line is played over
    si_root: i32,
    /// SUPERIMPOSE: the chord's kind and which chord (as in FIND A CHORD), and the scale it is heard in
    si_fam: u8,
    si_chord: u8,
    si_scale: u8,
    /// what a line is played over (LINES and SCALE HARMONY): nothing, a drone, the chord, the band
    ln_over: u8,
    hm_parent: u8,
    hm_key: i32,
    hm_stack: u8,
    hm_deg: u8,
    hm_solo: u8,
    hm_vamp: Vec<u8>,
    /// EAR: the drill, the question, this session
    ear: ear::Ear,
    /// LINES > ATONAL: your lines (from the private file) or generated ones, the note being fixed, the generator's
    /// choices
    at: atonal::AtState,
    /// DIARY > MY PLAN: your own plan (from the private file), the week looked at, today's session timer
    plan: plan::PlanState,
    si_any: bool,
    si_pick: u8,
    si_kind: u8,
    si_kroot: u8,
    /// TRIAD PAIRS: over a chord or through changes; a suggested pair or any two triads; the suggestion picked;
    /// the two triads picked (root, kind); the strings (0 anywhere, then the sets from the top down); the exercise;
    /// the changes (0 the open tune when there is one, then the set ones) and their key
    tp_mode: u8,
    tp_any: bool,
    tp_pick: u8,
    tp_a: (i32, u8),
    tp_b: (i32, u8),
    tp_strings: u8,
    tp_ex: u8,
    tp_prog: u8,
    tp_key: i32,
    /// SCALE FINGERINGS: the scale (by name), key, system (0 five positions, else notes a string), the position
    /// shown (its count = the whole neck), labels, which shape is position 1 (255: the one rooted on the 5th
    /// string), the starred scales, and shifting through them all
    pf_scale: String,
    pf_root: i32,
    pf_sys: u8,
    pf_pos: u8,
    pf_label: u8,
    pf_stars: Vec<String>,
    /// what SCALES plays: 0 this position up and down, 1 every position in turn, 2.. a pattern (3rds, 4ths...)
    pf_play: u8,
    /// CHORD MELODIES: SONGS or MOVES; the transcriptions listed; the moves kept (and read yet); the tune open and
    /// read; its tempo; what is shown (chord names, grips, staff, tab, note names); playing (since, from, to in
    /// quarter notes); looping; the bars to play; the beat picked (and the one shift-clicked); the moves' filters
    /// and their key
    cm_tab: u8,
    cm_list: Option<Vec<(std::path::PathBuf, String)>>,
    /// when the transcriptions folder was last read
    cm_list_at: Instant,
    cm_moves: Vec<transcription::Move>,
    cm_moves_read: bool,
    cm_file: Option<std::path::PathBuf>,
    cm_loaded: Option<(std::path::PathBuf, Arc<transcription::Tune>)>,
    /// the moves spotted in the tune open (worked out once per tune)
    cm_found: Option<(std::path::PathBuf, Vec<(usize, (usize, usize), (usize, usize))>)>,
    cm_bpm: u32,
    cm_show: [bool; 5],
    cm_play: Option<(Instant, f32, f32)>,
    cm_loop: bool,
    cm_from: usize,
    cm_to: usize,
    cm_sel: Option<(usize, usize)>,
    cm_sel2: Option<(usize, usize)>,
    cm_kind: u8,
    cm_star_only: bool,
    cm_shift: i32,
    /// the move open under the list
    cm_open: Option<usize>,
    /// the moves spotted, marked over the bars (all kinds, or one); the one clicked
    cm_marks: bool,
    cm_mark_kind: u8,
    cm_mark: Option<usize>,
    /// CHORD MELODIES: the beats being dragged over (from, to); the tune open has its chord names worked out
    cm_drag: Option<((usize, usize), (usize, usize))>,
    cm_named: bool,
    /// COLTRANE CELLS THROUGH A TUNE: which cell
    ct_cell: u8,
    /// the current loop: the diary key it was last kept under (its ends move with it until it is cleared or another is
    /// picked), and the saved loop it is (song, which), if one
    loop_ident: Option<String>,
    loop_saved: Option<(i64, usize)>,
    /// CHORD MELODIES: the spotted moves you removed
    cm_hidden: Vec<transcription::Hidden>,
    /// MOVES: a move playing (since, seconds a beat, which)
    cm_mplay: Option<(Instant, f32, usize)>,
    /// the panel's step cursor: (what is looked at (its first beat, the saved move), the beat)
    cm_cursor: Option<(((usize, usize), Option<usize>), (usize, usize))>,
    /// ARPEGGIOS: from a scale's chords or any chord; the key (or root); the scale; its degree; the chord kind; the
    /// position shown; what is played; what it is played over; the line through a tune
    ar_src: u8,
    ar_root: i32,
    ar_scale: u8,
    ar_deg: u8,
    ar_q: u8,
    ar_pos: u8,
    ar_ex: u8,
    ar_tune: u8,
    /// the grip clicked in the panel (the panel's first beat, which grip)
    cm_step: Option<((usize, usize), usize)>,
    /// a line played over something (0 nothing, 1 drone, 2 chord, 3 band) and that chord's notes; None for plain lines
    ln_backing: Option<(u8, Vec<i32>)>,
    /// a line over changes (TRIAD PAIRS): the chart the band plays (its text, its bars in play order), and each
    /// chord in turn - its notes, voiced low, and how many beats it lasts - for the drone and the strummed chord
    ln_prog: Option<(String, Vec<usize>, Vec<(Vec<i32>, f32)>)>,
    /// SLONIMSKY'S DIARY: the book's pattern open, the note it starts on (0 = C, as written), played backwards
    ln_at: usize,
    ln_root: i32,
    ln_back: bool,
    /// what SHUFFLE picks from: 0 the whole book, 1 this chapter, 2 favorites
    ln_shuffle: u8,
    /// the tempo and notes-a-beat being typed
    ln_bpm_text: String,
    ln_sub_text: String,
    /// the ABOUT THIS PATTERN panel open
    ln_about: bool,
    /// the HOW THESE PATTERNS WORK panel open
    ln_how: bool,
    /// with no pattern book: the octave division, the way notes are added, the pattern
    sl_div: usize,
    sl_kind: usize,
    sl_pat: usize,
    /// tempo, notes to a beat, round and round, and the line being played (for the diary's count)
    ln_bpm: u32,
    ln_sub: u8,
    ln_loop: bool,
    ln_playing: String,
    /// SCALES: the range and direction of its patterns
    sc_oct: usize,
    sc_dir: usize,
    /// the last sequence played, to go round again
    seq_last: Option<Vec<Vec<i32>>>,
    /// CHORDS > PROGRESSIONS: the family shown, key, minor, 7th chords, all twelve keys, one being typed
    prog_fam: usize,
    prog_key: i32,
    prog_minor: bool,
    prog_7: bool,
    /// the progression clicked, asking whether it goes on the lead sheet (group, row)
    prog_ask: Option<(usize, usize)>,
    prog_new: String,
    /// a practice progression on the lead sheet instead of a tune: (title, chart)
    prac_chart: Option<(String, String)>,
    /// a chord sequence playing: (started, seconds a chord, which list, chords)
    seq_play: Option<(Instant, f32, String, usize)>,
    /// the song whose info is being edited: its id, the title and the artist as typed
    edit_info: Option<(i64, String, String)>,
    /// EDIT INFO for a file: its album and its file name (renamed on disk when changed)
    edit_extra: (String, String),
    /// DIARY: which page (0 diary, 1 routine, 2 listening, 3 masters); LISTENING on gospel
    diary_tab: u8,
    /// TUNES: yours (0) or the recommended ones (1), jazz or gospel; a recommended tune being looked at (its chart
    /// goes on the lead sheet without becoming one of yours)
    tunes_tab: u8,
    tunes_gospel: bool,
    /// LISTENING: guitarists only
    listen_guitar: bool,
    preview_tune: Option<String>,
    listen_gospel: bool,
    /// TRAINER: after the passes, up a 4th (through 12 keys) rather than faster; keys done so far
    trainer_keys: bool,
    keys_done: u32,
    /// ALL TAKES open; a take asked about before it is deleted
    takes_open: bool,
    take_ask: Option<i64>,
    /// key and BPM being typed in the edit info panel
    edit_kb: (String, String),
    /// which skin groups are open on the BACKLINE page (ORIGINAL, RETRO ORIGINAL, WINAMP)
    skins_open: [bool; 3],
    /// under WINAMP in the skin list: FAVOURITES and WORN LATELY folded open or not
    wa_groups_open: [bool; 2],
    neck_zoom: f32,
    ghost_alpha: f32,
    /// the analyzer's reading: the one picked (click), the one hovered
    an_pick: Option<String>,
    an_hover: Option<String>,
    shape_cache: HashMap<String, Vec<[Option<u8>; 6]>>,
    /// the bar being edited by click in the chart: which bar, and its chords as typed
    bar_edit: Option<(usize, String)>,
    chart_busy: bool,
    chart_sugg: Vec<String>,
    chart_q: String,
    live_q: String,
    live: Option<(String, String, String)>,
    chart_live: bool,
    chart_pick: Option<usize>,
    mtab: u8,
    /// RECORD: the sound coming in while a take runs, when it started, the input picked, the inputs found
    rec_buf: Option<Arc<std::sync::Mutex<record::InBuf>>>,
    rec_t0: Option<Instant>,
    rec_dev: Option<String>,
    rec_devs: Vec<String>,
    /// the tune the take goes with (0 = free play, else the tune's index + 1); the band starts with it
    rec_tune: u32,
    rec_band: bool,
    /// the last take (its mixed version's id) and whether it has been listened to; KEEP / DELETE asked
    rec_last: Option<(i64, bool)>,
    rec_ask: bool,
    /// Diary: month shown (year, month; 0 = this month) and the day picked (days since 1970; 0 = today)
    diary_ym: (i32, u32),
    diary_sel: i64,
    /// DIARY: the picked day's times can be changed, and practice added by hand (what, minutes)
    diary_edit: bool,
    /// DIARY: ALL NOTES open, and how many days' notes it shows (0 = the day's notes box)
    diary_all: usize,
    /// ALL NOTES: the search; the filter ("*" the starred days, or a #tag); a year, a month or a day at a time (0, 1,
    /// 2) and a day inside the one shown (0: today); all the tags shown (not only the five used most); the calendar
    /// page turning (when, which way, the page before)
    notes_q: String,
    notes_filter: Option<String>,
    notes_mode: u8,
    notes_tags_all: bool,
    notes_flip: Option<(f64, i8, [String; 3])>,
    /// the diary calendar's page turning to another month: when, which way, the page it turns from
    diary_cal_flip: Option<(f64, i8, [String; 3])>,
    /// how many days fit the search and the filter (for ALL NOTES' heading)
    notes_fit: usize,
    /// DIARY: the notes box sliding to another day (when it started, whether to a later day, the old day's words)
    diary_flip: Option<(f64, bool, String)>,
    diary_add: (String, String),
    /// DIARY: the journal entry being changed (its place in the store's entries)
    entry_edit: Option<usize>,
    mt: band::Metro,
    mt_bpm: u32,
    mt_add: f32,
    mt_step_at: Instant,
    mt_sig: u64,
    mt_group: usize,
    vol_before: f32,
    look_style: u8,
    last_ppp: f32,
    last_size: Vec2,
    frames: u32,
    mt_vis: Option<(Instant, f32, usize)>,
    beat_failed: HashSet<i64>,
    /// each played song's beat, worked out as it starts (None: no steady beat), and the song FIND BEAT waits on
    beat_known: HashMap<i64, Option<band::BeatInfo>>,
    beat_wait: Option<i64>,
    mt_gen: u32,
    mt_pos: (f32, Instant),
    mt_sync_at: Instant,
    /// locked to a song's beats: the song time its laid-out clicks run to (rebuilt before then)
    mt_lead_end: f32,
    beat: Option<(i64, f32, f32)>,
    /// every beat of that song through it (followed as the band drifts), and its usual beat length
    beat_grid: Option<(i64, std::sync::Arc<Vec<f32>>, f32)>,
    beat_busy: bool,
    band_on: bool,
    band_style: u8,
    band_parts: [bool; 3],
    /// how loud each part plays (bass, chords, drums), 0 to 1.5
    band_lv: [f32; 3],
    /// times through the chart before the band stops (0 = until you stop it)
    band_loops: u32,
    /// the band's parts as last rendered (for which chart, tempo and style), so a level change only remixes
    band_stems: Option<(u64, std::sync::Arc<[Vec<f32>; 3]>)>,
    band_t0: Instant,
    band_lead: f32,
    band_bar: f32,
    /// seconds a beat, and where each bar of band_order starts (in beats, the total last)
    band_beat: f32,
    band_starts: Vec<f32>,
    band_order: Vec<usize>,
    band_sig: u64,
    band_chart: String,
    pomo_sound: bool,
    /// the focus timer's extras (auto-start, long rest, music, diary) and its pills
    tm: tools::TimerMore,
    lib_frac: f32,
    /// the chord finder's voicing group on show (drop 2, drop 3...)
    voicing_tab: u8,
    /// the stack's width with a skin worn, dragged (0: as wide as fits at a whole-pixel size)
    stack_w: f32,
    stack_frac: f32,
    /// the Winamp stack's size: points per skin pixel (a whole number of screen pixels)
    stack_k: f32,
    /// the practice panel's height (drag the edge above it)
    prac_h: f32,
    /// Tidal can't be reached (no internet): when it was last tried (it is tried again now and then)
    net_down: bool,
    net_try: Instant,
    /// the wait before the next try after the internet went: longer each time it fails (30 s up to 10 min)
    net_wait: u64,
    /// offline mode: you chose it - nothing goes out, only what is on this computer shows and plays
    go_offline: bool,
    /// how many liked songs were last kept on disk
    lib_saved_n: usize,
    /// how many liked songs are in, and since when (they are kept once the count stops changing)
    lib_seen: (usize, Instant),
    /// the equalizer's height, dragged (None: it follows the player's size)
    eq_h_user: Option<f32>,
    /// the skinned deck's size in the player window (skin pixels to screen points; None: as large as fits)
    deck_zoom: Option<f32>,
    /// the skinned equalizer's size in its window (None: as large as fits)
    eq_zoom: Option<f32>,
    /// the player was sized in this session (until then, the cover beside it always shows)
    deck_free: bool,
    /// the box for naming a new equalizer curve is open
    eq_naming: bool,
    /// until when the skinned deck shows where its buttons are (just after a skin is put on)
    wa_map_until: f64,
    /// the skinned deck's guide (each button outlined and named) stays on
    wa_guide: bool,
    /// thin outlines round a skin's real buttons while the pointer is over the player (off: the skin as drawn)
    wa_outline: bool,
    /// your Custom look's two colours (background, accent), and whether it's the look you wore last
    custom_cols: (Color32, Color32),
    look_custom: bool,
    /// the Custom look's editor, while open
    look_edit: Option<looks::LookEdit>,
    /// whether the system's title bar is on (None: not set yet this window state; see `app_bar`)
    deco_now: Option<bool>,
    /// the queue hidden (the PL button), the player taking the whole column
    queue_hidden: bool,
    /// where each floating window sits (outer rectangle), by name
    /// the first row shown in the Winamp playlist (a fraction while scrolling)
    wa_pl_top: f32,
    /// the playlist font picked (index into winamp_ui::PL_FONTS; 0 = the skin's own)
    pl_font: usize,
    /// equalizer AUTO: remember an EQ per song, and the ones remembered
    eq_auto: bool,
    eq_songs: HashMap<i64, [f32; 10]>,
    /// your own equalizer curves, by name
    eq_mine: Vec<(String, [f32; 10])>,
    /// the name typed for the curve about to be saved
    eq_name: String,
    /// left / right balance, -100 to 100
    balance: i32,
    ebuf: String,
    ebuf_id: u32,
    chart_tried: std::collections::HashSet<String>,
    /// tunes whose writer and year were looked up this session
    details_tried: std::collections::HashSet<String>,
    chart_rn: bool,
    /// the lead sheet stays put (LOCKED) instead of following the song playing (the default)
    chart_lock: bool,
    ireal_in: String,
    /// a tune's page: its written-by / year / from / style boxes open
    tune_more: bool,
    look_q: String,
    chart_cache: (String, chart::Chart),
}

/// Audio bytes for a track: a file on this computer, from the local cache if it's stored,
/// a YouTube clip (downloaded once with yt-dlp), or otherwise downloaded from Tidal
/// (and then stored, unless caching is switched off).
pub(crate) fn fetch_audio(api: &Api, id: i64, lossless: bool, keep: bool, src: Option<Src>) -> Result<Vec<u8>, String> {
    match src {
        Some(Src::File(p)) => {
            return std::fs::read(&p).map_err(|e| format!("can't open {}: {}", p.display(), e));
        }
        Some(Src::Yt(vid)) => {
            if let Some(b) = cache::get(id) {
                crate::api::log(&format!("youtube {}: playing from disk ({} bytes)", vid, b.len()));
                return Ok(b);
            }
            crate::api::guard()?;
            let t0 = Instant::now();
            let b = sources::yt_fetch(&vid)?;
            crate::api::log(&format!("youtube {}: {} bytes in {:.1} s", vid, b.len(), t0.elapsed().as_secs_f32()));
            if keep {
                if let Some(p) = cache::put(id, &b) {
                    crate::api::log(&format!("youtube {}: saved to {}", vid, p.display()));
                }
            }
            return Ok(b);
        }
        None => {}
    }
    if let Some(b) = cache::get(id) {
        crate::api::log(&format!("track {}: playing from disk ({} bytes)", id, b.len()));
        return Ok(b);
    }
    let b = api.stream_url(id, lossless).and_then(|u| api.fetch(&u))?;
    if keep {
        match cache::put(id, &b) {
            Some(p) => crate::api::log(&format!("track {}: saved to {}", id, p.display())),
            None => crate::api::log(&format!("track {}: could not save to {}", id, cache::dir().display())),
        }
    }
    Ok(b)
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> App {
        let st = load_settings();
        // the look worn last (an older version's "skin" number: where that palette went)
        let look = match (st["look"].as_u64(), st["skin"].as_u64()) {
            (Some(n), _) => n as usize % LOOKS.len(),
            (None, Some(old)) => look_from_old(old as usize),
            _ => FIRST_LOOK,
        };
        SKIN.store(look, Ordering::Relaxed);
        font::set_ppp(cc.egui_ctx.pixels_per_point());
        let font_tex = font::init(&cc.egui_ctx);
        // the installed-font list (Winamp skins name their font) is read in the background, never on screen
        std::thread::spawn(winamp_ui::warm_fonts);
        // smooth text (the sleek skins, tooltips) in Chakra Petch: retro and techy without being pixel text
        // (SIL Open Font License, see assets/ChakraPetch-OFL.txt)
        set_fonts(&cc.egui_ctx, smooth_fonts(None));
        setup_style(&cc.egui_ctx);
        let api = Api::new();
        let (tx, rx) = channel();
        let ctx = cc.egui_ctx.clone();
        let req = spawn_image_pool(api.clone(), tx.clone(), ctx.clone());
        let thumbs = spawn_thumb_pool(api.clone(), tx.clone(), ctx.clone());
        let mut app = App {
            api,
            tx,
            rx,
            ctx,
            images: Images { map: HashMap::new(), requested: HashSet::new(), req, thumbs, avg: HashMap::new(), hold: HashMap::new() },
            _font_tex: font_tex,
            auth: Auth::LoggedOut,
            login_code: None,
            login_err: String::new(),
            page: None,
            lib_page: None,
            lib_at: None,
            lib_looking: false,
            keeping: HashSet::new(),
            mine_cache: None,
            was_focused: true,
            art_preview: None,
            wa_loading: None,
            back: Vec::new(),
            fwd: Vec::new(),
            credits: None,
            artist_pick: None,
            serial: 0,
            lib_gen: 0,
            lib_tab: Tab::Tracks,
            loading: false,
            search: String::new(),
            search_focus: false,
            status: String::new(),
            status_err: false,
            queue: Arc::new(Vec::new()),
            orig_queue: None,
            cur: None,
            q_scroll: false,
            play_gen: 0,
            buffering: false,
            paused: false,
            stopped: true,
            prefer_lossless: true,
            kbps: 0,
            prefetched: None,
            prefetching: 0,
            player: Player::new(),
            volume: 0.8,
            shuffle: false,
            repeat: Repeat::Off,
            seek_drag: None,
            wv_zoom: 1.0,
            wv_start: 0.0,
            wv_free: false,
            wv_h: 1.0,
            wv_notes: false,
            notes: (0, 0, None),
            notes_busy: false,
            notes_tex: Vec::new(),
            bands: vec![0.0; 32],
            peaks: vec![0.0; 32],
            audio_err_shown: false,
            art_view: false,
            show_spec: true,
            vis_gl: Arc::new(Mutex::new(visuals::Gl::default())),
            vis_preset: visuals::find("Down the Drain"),
            vis_np: false,
            vis_agc: Default::default(),
            vis_full: false,
            vis_saver: false,
            vis_was_fs: false,
            vis_shown_at: 0.0,
            vis_ui_at: 0.0,
            vis_seen: String::new(),
            vis_idle: 5,
            vis_idle_since: Instant::now(),
            vis_last: 0.0,
            vis_travel: 0.0,
            vis_cache: None,
            vis_pipes: Default::default(),
            vis_stars: Default::default(),
            pop: None,
            pop_px: None,
            pop_pin: true,
            spec_op: 0.2,
            spec_h: 0.42,
            spec_w: 0.6,
            spec_frame: 1,
            spec_lyr: 0,
            viz_panel: None,
            side: None,
            hist_back: Vec::new(),
            hist_fwd: Vec::new(),
            hist_at: None,
            wa_q: String::new(),
            wa_list: Vec::new(),
            wa_busy: false,
            wa_end: false,
            wa_worn: None,
            wa_recent: Vec::new(),
            wa_favs: Vec::new(),
            wa_tab: 0,
            wa_preview: None,
            wa_art: None,
            art_border: 1,
            cap_size: 1,
            cap_dark: false,
            cap_show: 0,
            cap_art: 0.0,
            cap_art_at: 0.0,
            cap_song: 0,
            cap_mode_at: -10.0,
            bg_now: None,
            out_device: None,
            sys_output: String::new(),
            out_list: Vec::new(),
            out_bt: Vec::new(),
            art_queue: false,
            viz: [viz::VizCfg::player(), viz::VizCfg::art()],
            viz_wave: vec![0.0; WAVE_N],
            show_lyrics: true,
            fullscreen: false,
            fs_by_art: false,
            art_tilt: Vec2::ZERO,
            lyrics: None,
            lyric_scroll: 0.0,
            lyric_free: 0.0,
            liked: HashSet::new(),
            q_drag: None,
            q_manual: 0,
            q_drop: true,
            mine_off: Vec::new(),
            mine_stems: false,
            mt_vol: 0.8,
            q_added: HashSet::new(),
            q_next: 0,
            sleep_at: None,
            sleep_mins: 0,
            mini: false,
            mini_size_at: None,
            mini_saved: (false, false),
            mini_stack: (false, false),
            pinned: false,
            saved_size: Vec2::ZERO,
            saved_max: false,
            advancing: false,
            show_eq: false,
            eq_gains: [0.0; 10],
            eq_pre: 0.0,
            eq_on: false,
            dirty: false,
            last_save: Instant::now(),
            last_title: String::new(),
            practice: false,
            loop_a: None,
            loop_b: None,
            loop_on: false,
            speed: 100,
            loops: HashMap::new(),
            menu_t: 0.0,
            mark_col: None,
            // tracks are streamed, not kept; saving them to disk is switched on by hand
            keep_cache: false,
            keep_sweep: true,
            cache_stats: (0, 0),
            cache_at: Instant::now(),
            store: store::Store::load(),
            store_dirty: false,
            store_saved: Instant::now(),
            sync_in: String::new(),
            here: HashSet::new(),
            diary_note: (String::new(), String::new()),
            srcmap: HashMap::new(),
            sec: Sec::Tidal,
            last_music: Sec::Tidal,
            last_practice: Sec::Tunes,
            ed: Ed::default(),
            files_busy: false,
            yt_busy: false,
            yt_msg: String::new(),
            yt_in: String::new(),
            sc_in: String::new(),
            marker_drag: 0,
            pan_off: 0.0,
            pan_len: 0.0,
            sc_msg: String::new(),
            sc_busy: false,
            sc_results: Vec::new(),
            sc_sets: Vec::new(),
            sc_cur: None,
            list_ask: None,
            upd: None,
            upd_note: String::new(),
            upd_at: 0,
            upd_state: 0,
            last_active: Instant::now(),
            show_help: false,
            palette_open: false,
            palette_q: String::new(),
            palette_all: false,
            diary_song: None,
            diary_span: 0,
            diary_reading: true,
            loops_filter: String::new(),
            loops_sort: 0,
            palette_sel: 0,
            palette_frame: 0,
            menu_keep: None,
            fs_cmd_at: None,
            art_op: 0.9,
            show_prefs: false,
            auto_restart: false,
            intro: 1,
            intro_file: None,
            intro_sound: 0,
            binds: prefs::default_binds(),
            rebinding: None,
            bind_note: String::new(),
            tour: None,
            tour_restore: None,
            tour_play: None,
            demo_rec: tour::demo_kind().map(tour::DemoRec::new),
            demo_track: None,
            media: None,
            thumb: None,
            thumb_tried: false,
            tour_seen: false,
            icon_names: false,
            first_look: false,
            lib_zoom: 1.0,
            lib_rect: Rect::NOTHING,
            help_tab: 0,
            tuning: None,
            tuning_busy: false,
            upd_checked: None,
            new_pl: String::new(),
            pl_open: None,
            queue_from: String::new(),
            toast: None,
            pl_dupe: None,
            queue_ask: None,
            queue_save: None,
            pl_picker: None,
            pl_find: String::new(),
            pl_last_tidal: true,
            copying: Arc::new(peer_files::Copying::default()),
            peer: peer_files::PeerView::default(),
            peer_ask: None,
            peer_play: 0,
            peer_always: false,
            shared_n: usize::MAX,
            copying_last_for_now: false,
            files_there: false,
            files_here: true,
            tidal_lists: None,
            new_pl_dlg: None,
            tidal_lists_busy: false,
            tidal_new_name: String::new(),
            lib_filter: String::new(),
            yt_results: Vec::new(),
            yt_title: String::new(),
            yt_cur: None,
            offline: false,
            new_tune: String::new(),
            tune_open: None,
            edit_ver: None,
            del_arm: false,
            pick: None,
            look_busy: false,
            look_for: String::new(),
            look_tracks: Vec::new(),
            name_guess: (String::new(), 0, None),
            name_kept: Vec::new(),
            f_tune: String::new(),
            f_mins: String::new(),
            f_bpm: String::new(),
            f_note: String::new(),
            show_form: false,
            wave: (0, None),
            wave_fine: (0, None),
            lib_hidden: false,
            lib_shown_w: None,
            more: false,
            stem_busy: 0,
            stem_on: [true; 4],
            pomo: 0,
            pomo_left: 0,
            pomo_was: 1,
            pomo_end: None,
            pomo_focus: 25,
            pomo_break: 5,
            pomo_cycles: 4,
            pomo_n: 1,
            pomo_flash: None,
            timer_open: false,
            metro_open: false,
            bpm: 0,
            count_in: 0,
            taps: Vec::new(),
            trainer: false,
            trainer_step: 5,
            trainer_n: 4,
            passes: 0,
            last_wraps: 0,
            last_wrap_at: None,
            semis: 0,
            pitch_text: String::new(),
            my_a: 440.0,
            my_a_text: String::new(),
            rec_a_text: String::new(),
            chan: 0,
            acc: 0.0,
            acc_at: Instant::now(),
            last_cap: Instant::now(),
            last_q: 0,
            notes_tried: (0, 0),
            sec_name: String::new(),
            pending_seek: None,
            loops_all: false,
            loops_show: 0,
            loops_pick: None,
            clip_edit: None,
            mark_edit: None,
            edit_in_practice: false,
            clip_name: String::new(),
            clip_note: String::new(),
            clip_group: String::new(),
            loop_ask: None,
            grp_edit: None,
            grp_name: String::new(),
            grp_ask: None,
            grp_new: String::new(),
            loops_mode: 0,
            share_view: None,
            share_pick: None,
            pack_off: Vec::new(),
            pack_loops: true,
            pack_in: None,
            news: None,
            seen_build: 0,
            grp_adding: false,
            grp_open: None,
            sel_anchor: None,
            exporting: false,
            rtab: 0,
            chart_tr: 0,
            chart_sel: None,
            chart_drag: None,
            chart_edit: false,
            bar_edit: None,
            chords_tab: 0,
            vc_step: 0,
            cf_root: None,
            cf_fam: None,
            cb_size: 3,
            cb_steps: [true, false, true, false],
            cb_lo: 0,
            chord_q: "Dm7".to_string(),
            an_frets: [None; 6],
            grip_ask: None,
            grip_for: None,
            gs_open: false,
            gs_name: String::new(),
            gs_pick: (0, 2, 0),
            chord_sp: theory::Spelling { slash: true, ..theory::Spelling::default() },
            chord_details: false,
            cl_key: 0,
            cl_scale: 0,
            cl_view: 0,
            cl_semi: true,
            cl_low: 11,
            cl_bass: 0,
            fc_bass: 0,
            fb_low: 11,
            cl_changes: "Dm7b5 G7 Cmaj7 Gbm7b5 Fm7 Em7 Ebdim7 Dm7 G7 Cmaj7".to_string(),
            cl_chord: 0,
            cl_mode: 0,
            tr_page: 0,
            tr_root: 0,
            tr_q: 0,
            tr_set: 0,
            tr_set4: 0,
            tr_sel: 0,
            tr_whole: true,
            tr_pos: 1,
            tr_prog: "1 4 5 1".to_string(),
            tr_bass: 2,
            mv_page: 0,
            mv_move: 0,
            mv_dip: false,
            mv_parent: 0,
            mv_mode: 1,
            mv_stack: 0,
            mv_motion: 0,
            mv_pedal: true,
            mv_play: None,
            vc_page: 0,
            rd_key: 0,
            rd_pos: 1,
            rd_level: 0,
            rd_bars: 1,
            rd_time: 0,
            rd_bpm: 70,
            rd_tab: false,
            rd_names: false,
            rd_map: false,
            rd_line: None,
            rd_play: None,
            rd_active: None,
            rd_acc: 0.0,
            rd_mode: 0,
            rd_book_loaded: false,
            rd_book: Vec::new(),
            rd_errs: Vec::new(),
            rd_lesson: 0,
            rd_piece: 0,
            rd_part: 0,
            rd_tempo_for: (usize::MAX, 0),
            rd_ctx: None,
            rd_bpm_text: String::new(),
            rd_both: true,
            rd_zoom: 1.0,
            rd_guide: 0,
            rd_marks: true,
            rd_fingering: true,
            cb_frets: [None; 6],
            cb_name: String::new(),
            vc_family: 4,
            vc_root: 0,
            vc_type: 0,
            vc_set: 0,
            pass_pair: 0,
            ln_tab: 0,
            pn_root: 9,
            pn_box: 0,
            pn_blue: false,
            pn_between: false,
            pn_label: 0,
            pn_ex: 0,
            pn_string: 5,
            pn_num: String::new(),
            pf_fam: String::new(),
            pf_range: 0,
            pf_num: String::new(),
            ar_num: String::new(),
            pn_fam: 0,
            pn_scale: 0,
            pn_nps: 2,
            si_root: 0,
            si_fam: 0,
            si_chord: 4,
            si_scale: 0,
            // G melodic minor, its Bb maj7#5 picked
            hm_parent: 1,
            ln_over: 3,
            hm_key: 7,
            hm_stack: 1,
            hm_deg: 2,
            hm_solo: 0,
            hm_vamp: Vec::new(),
            ear: Default::default(),
            at: Default::default(),
            plan: Default::default(),
            si_any: false,
            si_pick: 0,
            si_kind: 0,
            si_kroot: 7,
            tp_mode: 0,
            tp_any: false,
            tp_pick: 0,
            tp_a: (2, 0),
            tp_b: (4, 1),
            tp_strings: 0,
            tp_ex: 0,
            tp_prog: 0,
            tp_key: 0,
            pf_scale: "Ionian (Major)".to_string(),
            pf_root: 0,
            pf_sys: 0,
            pf_pos: 0,
            pf_label: 0,
            pf_stars: Vec::new(),
            pf_play: 0,
            cm_tab: 0,
            cm_list: None,
            cm_list_at: Instant::now(),
            cm_moves: Vec::new(),
            cm_moves_read: false,
            cm_file: None,
            cm_loaded: None,
            cm_found: None,
            cm_bpm: 90,
            cm_show: [true; 5],
            cm_play: None,
            cm_loop: false,
            cm_from: 0,
            cm_to: usize::MAX,
            cm_sel: None,
            cm_sel2: None,
            cm_kind: 0,
            cm_star_only: false,
            cm_shift: 0,
            cm_open: None,
            cm_marks: true,
            cm_mark_kind: 0,
            cm_mark: None,
            cm_drag: None,
            cm_named: false,
            ct_cell: 0,
            loop_ident: None,
            loop_saved: None,
            cm_hidden: Vec::new(),
            cm_mplay: None,
            cm_cursor: None,
            ar_src: 0,
            ar_root: 0,
            ar_scale: 0,
            ar_deg: 1,
            ar_q: 0,
            ar_pos: 0,
            ar_ex: 0,
            ar_tune: 0,
            cm_step: None,
            ln_backing: None,
            ln_prog: None,
            ln_at: 0,
            ln_root: 0,
            ln_back: false,
            ln_shuffle: 0,
            ln_bpm_text: String::new(),
            ln_sub_text: String::new(),
            ln_about: false,
            ln_how: false,
            sl_div: 0,
            sl_kind: 0,
            sl_pat: 0,
            ln_bpm: 80,
            ln_sub: 2,
            ln_loop: true,
            ln_playing: String::new(),
            sc_oct: 2,
            sc_dir: 2,
            seq_last: None,
            prog_fam: 0,
            prog_key: 0,
            prog_minor: false,
            prog_7: true,
            prog_ask: None,
            prog_new: String::new(),
            prac_chart: None,
            seq_play: None,
            skins_open: [false; 3],
            wa_groups_open: [true, false],
            edit_info: None,
            edit_kb: (String::new(), String::new()),
            edit_extra: (String::new(), String::new()),
            diary_tab: 0,
            tunes_tab: 0,
            tunes_gospel: false,
            listen_guitar: false,
            preview_tune: None,
            listen_gospel: false,
            trainer_keys: false,
            keys_done: 0,
            takes_open: false,
            take_ask: None,
            neck_zoom: 1.0,
            ghost_alpha: 0.7,
            an_pick: None,
            an_hover: None,
            shape_cache: HashMap::new(),
            chart_busy: false,
            chart_sugg: Vec::new(),
            chart_q: String::new(),
            live_q: String::new(),
            live: None,
            chart_live: false,
            chart_pick: None,
            mtab: 0,
            rec_buf: None,
            rec_t0: None,
            rec_dev: None,
            rec_devs: Vec::new(),
            rec_tune: 0,
            rec_band: true,
            rec_last: None,
            rec_ask: false,
            diary_ym: (0, 0),
            diary_sel: 0,
            diary_edit: false,
            diary_all: 0,
            notes_q: String::new(),
            notes_filter: None,
            notes_mode: 1,
            notes_tags_all: false,
            notes_flip: None,
            diary_cal_flip: None,
            notes_fit: 0,
            diary_flip: None,
            diary_add: (String::new(), String::new()),
            entry_edit: None,
            mt: band::Metro::default(),
            mt_bpm: 100,
            mt_add: 0.0,
            mt_step_at: Instant::now(),
            mt_sig: 0,
            mt_group: 0,
            vol_before: 0.7,
            look_style: 0,
            last_ppp: 0.0,
            last_size: Vec2::ZERO,
            frames: 0,
            mt_vis: None,
            beat_failed: HashSet::new(),
            beat_known: HashMap::new(),
            beat_grid: None,
            beat_wait: None,
            mt_gen: 0,
            mt_pos: (0.0, Instant::now()),
            mt_sync_at: Instant::now(),
            mt_lead_end: 0.0,
            beat: None,
            beat_busy: false,
            band_on: false,
            band_style: 0,
            band_parts: [true; 3],
            band_lv: [1.0; 3],
            band_loops: 0,
            band_stems: None,
            band_t0: Instant::now(),
            band_lead: 0.0,
            band_bar: 2.0,
            band_beat: 0.5,
            band_starts: Vec::new(),
            band_order: Vec::new(),
            band_sig: 0,
            band_chart: String::new(),
            pomo_sound: false,
            tm: Default::default(),
            lib_frac: 0.34,
            voicing_tab: 0,
            stack_w: 0.0,
            stack_k: 2.0,
            stack_frac: 1.0,
            prac_h: 0.0,
            net_down: false,
            net_try: Instant::now(),
            net_wait: 30,
            go_offline: false,
            lib_saved_n: 0,
            lib_seen: (0, Instant::now()),
            eq_h_user: None,
            deck_zoom: None,
            eq_zoom: None,
            deck_free: false,
            eq_naming: false,
            wa_map_until: 0.0,
            wa_guide: false,
            wa_outline: false,
            custom_cols: (Color32::from_rgb(36, 45, 67), Color32::from_rgb(110, 170, 255)),
            look_custom: false,
            look_edit: None,
            deco_now: None,
            queue_hidden: false,
            wa_pl_top: 0.0,
            pl_font: 0,
            eq_auto: false,
            eq_songs: HashMap::new(),
            eq_mine: Vec::new(),
            eq_name: String::new(),
            balance: 0,
            ebuf: String::new(),
            ebuf_id: 0,
            chart_tried: Default::default(),
            details_tried: Default::default(),
            chart_rn: true,
            chart_lock: false,
            ireal_in: String::new(),
            tune_more: false,
            look_q: String::new(),
            chart_cache: (String::new(), chart::Chart::default()),
        };
        meta::load(&app.store.meta);
        app.init_store();
        app.liked.extend(app.store.hearts.iter().map(|e| e.id));
        // songs played from the other computer last time were only for then
        std::thread::spawn(|| {
            let _ = std::fs::remove_dir_all(peer_files::for_now_dir());
        });
        // SYNC with your other computer, when it's on: what comes in arrives as messages
        {
            let (tx, ctx) = (app.tx.clone(), app.ctx.clone());
            sync::start(Box::new(move |inc| {
                let _ = tx.send(match inc {
                    sync::Incoming::Files(v) => Msg::SyncFiles(v),
                    sync::Incoming::Library(v) => Msg::SyncLibrary(v),
                });
                ctx.request_repaint();
            }));
        }
        app.restore_settings(&st);
        cache::scan();
        // (saved copies of YouTube and SoundCloud songs nothing has any more, left from before: deleted)
        app.drop_unused_copies(&cache::ids());
        // every stored song gets its details in its own tags (from the liked list, the lists you made and the songs
        // played lately), in the background; offline, the songs are listed from those tags
        let mut known: Vec<Track> = crate::api::load_liked();
        known.extend(app.store.recent.iter().filter(|(v, _)| v.kind != "file").map(|(v, _)| v.to_track()));
        for pl in &app.store.playlists {
            known.extend(pl.items.iter().filter(|v| v.kind != "file").map(|v| v.to_track()));
        }
        known.extend(app.store.yt.iter().chain(app.store.sc.iter()).map(|e| e.to_track()));
        // (the names list an earlier version kept, folded in once)
        let old = api::config_dir().join("saved_names.json");
        if old.exists() {
            known.extend(crate::api::load_tracks_file("saved_names.json"));
            let _ = std::fs::remove_file(old);
        }
        cache::know(&known);
        cache::start_tagging(|t| {
            if t.cover.is_empty() {
                return None;
            }
            [640, 320, 1280, 160, 80]
                .iter()
                .find_map(|s| std::fs::read(cover_path(&api::cover_url(&t.cover, *s))).ok().filter(|b| b.len() > 100))
        });
        app.player.send(Cmd::Volume(app.volume * app.volume));
        app.apply_eq();
        // the skin's bars as the window's title bar, to Windows
        frame_win::install(cc);
        // Windows' media controls (not while recording a demo, which plays in the background); the hotkeys only
        // when they can't be had
        if tour::demo_kind().is_none() {
            app.media = media::controls(cc, app.tx.clone(), app.ctx.clone());
            if app.media.is_none() {
                media::spawn(app.tx.clone(), app.ctx.clone());
            }
        }
        // a song's end is seen at once, even while the window is out of sight and drawn only now and then
        let wake = app.ctx.clone();
        player::on_end(move || wake.request_repaint());
        // the queue from last time, ready to play (CONTINUE picks up the song where it was)
        if let Some(l) = app.store.last.clone().filter(|l| !l.queue.is_empty()) {
            for v in &l.queue {
                app.register(v);
            }
            app.queue = Arc::new(l.queue.iter().map(|v| v.to_track()).collect());
            app.cur = Some(l.index.min(l.queue.len() - 1));
        }
        crate::api::log(&format!("backline {} started (token saved: {})", VERSION, app.api.has_token()));
        // a test copy (a test profile, BACKLINE_OPEN or BACKLINE_MUTE) makes no sound - not even the opening chime:
        // it plays, but nothing reaches the speakers
        // (BACKLINE_SHOW: a test copy put in front of you to try - with sound)
        let showing = std::env::var_os("BACKLINE_SHOW").is_some();
        // (BACKLINE_MUTE keeps even a shown one silent)
        if std::env::var_os("BACKLINE_MUTE").is_some()
            || (!showing && ["BACKLINE_HOME", "BACKLINE_OPEN"].iter().any(|v| std::env::var_os(v).is_some()))
        {
            crate::player::MUTED.store(true, Ordering::Relaxed);
        }
        app.play_intro();
        if app.api.has_token() {
            // the app opens straight away; the saved session is checked behind it (the library shows LOADING
            // meanwhile), and only an expired one brings up the sign-in screen
            app.auth = Auth::In;
            app.loading = true;
            let (api, tx, ctx) = (app.api.clone(), app.tx.clone(), app.ctx.clone());
            std::thread::spawn(move || {
                match api.refresh_info() {
                    Ok(()) => {
                        let _ = tx.send(Msg::LoggedIn);
                    }
                    // no internet (or offline mode) is not a logged-out session: open as usual with what is here
                    Err(e) if crate::api::no_internet(&e) || e == crate::api::OFFLINE_MSG => {
                        let _ = tx.send(Msg::NoInternet);
                    }
                    Err(e) => {
                        let _ = tx.send(Msg::NeedLogin(e));
                    }
                }
                ctx.request_repaint();
            });
        }
        app.open_for_test();
        app
    }

    fn apply_eq(&self) {
        self.player.eq.set(self.eq_gains, self.eq_pre, self.eq_on);
    }

    /// Fade out and stop when the sleep timer runs out.
    fn sleep_tick(&mut self) {
        let Some(at) = self.sleep_at else { return };
        let now = Instant::now();
        if now >= at {
            self.sleep_at = None;
            self.sleep_mins = 0;
            self.apply(Action::StopBtn);
            self.player.send(Cmd::Volume(self.volume * self.volume));
            self.set_note("SLEEP TIMER DONE. GOOD NIGHT");
        } else {
            let rem = (at - now).as_secs_f32();
            if rem < 20.0 {
                self.player.send(Cmd::Volume(self.volume * self.volume * (rem / 20.0)));
            }
        }
    }

    /// Push loop / speed state to the audio thread and remember the loop for this track.
    fn sync_loop(&mut self) {
        let both = self.loop_a.zip(self.loop_b);
        let on = self.practice && self.loop_on && both.is_some();
        let (a, b) = both.unwrap_or((0.0, 0.0));
        self.player.ctl.set_loop(on, a, b);
        self.player.ctl.set_speed(self.speed);
        self.player.ctl.set_semis(if self.practice { self.semis } else { 0 });
        self.player.ctl.set_chan(if self.practice { self.chan } else { 0 });
        self.player.ctl.set_balance(self.balance);
        self.player.ctl.set_count_in(if self.practice { self.count_in } else { 0 }, self.bpm as f32);
        // the loop is remembered for the song (not for the tour's demo song)
        if let (Some((id, i)), Some(t), Some((a, b))) = (self.loop_saved, self.cur_track(), both) {
            if t.id == id {
                if let Some(s) = self.store.sections.get_mut(&id).and_then(|l| l.get_mut(i)) {
                    if (s.a - a).abs() > 0.001 || (s.b - b).abs() > 0.001 {
                        s.a = a;
                        s.b = b;
                        self.store_dirty = true;
                    }
                }
            }
        }
        if let Some(t) = self.cur_track().filter(|t| !self.is_demo(t.id)) {
            let before = self.loops.get(&t.id).copied();
            match both {
                Some(ab) => {
                    self.loops.insert(t.id, ab);
                }
                None => {
                    self.loops.remove(&t.id);
                }
            }
            // (saved only when it changed: this runs at every song's start)
            if self.loops.get(&t.id).copied() != before {
                self.dirty = true;
            }
        }
    }

    fn track_len(&self) -> f32 {
        self.cur_track().map(|t| t.duration).unwrap_or(0.0)
    }

    fn sync_library_like(&mut self, t: &Track, like: bool) {
        let f = |p: &mut Arc<Page>| {
            if p.library {
                let pg = Arc::make_mut(p);
                if like {
                    if !pg.tracks.iter().any(|x| x.id == t.id) {
                        pg.tracks.insert(0, t.clone());
                        pg.total += 1;
                    }
                } else {
                    pg.tracks.retain(|x| x.id != t.id);
                    pg.total = pg.total.saturating_sub(1);
                }
            }
        };
        if let Some(p) = self.page.as_mut() {
            f(p);
        }
        // the saved copy of your library too, unless it is the page on show (already done)
        if let Some(l) = self.lib_page.as_mut() {
            if !self.page.as_ref().is_some_and(|p| Arc::ptr_eq(p, l)) {
                f(l);
            }
        }
        for b in self.back.iter_mut() {
            f(b);
        }
    }

    fn set_err(&mut self, e: String) {
        api::log(&format!("ERROR: {}", e));
        // no internet: say so plainly, not the network library's words (and keep trying until it is back)
        let raw = crate::api::no_internet(&e) && e.chars().any(|c| c.is_lowercase()) && !self.go_offline;
        if raw && self.auth == Auth::In && !self.offline && !self.net_down {
            let _ = self.tx.send(Msg::NoInternet);
        }
        let e = if raw { "NO INTERNET, AND THAT NEEDS IT. SONGS KEPT ON THIS COMPUTER STILL PLAY".to_string() } else { e };
        self.status = e;
        self.status_err = true;
    }

    fn set_note(&mut self, s: &str) {
        self.status = s.to_string();
        self.status_err = false;
    }

    /// A note that also shows for a moment over the library, where you are (for what you'd otherwise not see
    /// happen: a song put in a playlist).
    pub(crate) fn toast(&mut self, s: &str) {
        self.set_note(&s.to_uppercase());
        self.toast = Some((s.to_string(), self.ctx.input(|i| i.time)));
        self.ctx.request_repaint();
    }

    /// A panel is open over the app (a question, a window to fill in, help, preferences, the tour...): what's behind
    /// it doesn't take keys or the mouse's side buttons till it closes.
    pub(crate) fn panel_open(&self) -> bool {
        self.show_help
            || self.show_prefs
            || self.palette_open
            || self.tour.is_some()
            || self.credits.is_some()
            || self.viz_panel.is_some()
            || self.artist_pick.is_some()
            || self.new_pl_dlg.is_some()
            || self.pl_picker.is_some()
            || self.pl_dupe.is_some()
            || self.queue_ask.is_some()
            || self.queue_save.is_some()
            || self.wa_preview.is_some()
            || self.peer_ask.is_some()
            || self.edit_info.is_some()
            || self.takes_open
            || self.news.is_some()
            || self.pack_in.is_some()
            || self.share_view.is_some()
            || self.look_edit.is_some()
    }

    /// The name of the page these songs are the songs of (a radio, a mix, an album); empty when they're not.
    fn list_name(&self, tracks: &[Track]) -> String {
        match &self.page {
            Some(p) if !p.library && p.tracks.len() == tracks.len() && p.tracks.iter().zip(tracks).all(|(a, b)| a.id == b.id) => {
                p.title.clone()
            }
            _ => String::new(),
        }
    }

    /// Keep what the menus know of your Tidal playlists in step, and fetch them when a menu asked for them.
    fn sync_pl_menu(&mut self) {
        let tidal_on = self.api.has_token() && !self.offline;
        let want = PL_MENU.with(|m| {
            let mut m = m.borrow_mut();
            if m.tidal != self.tidal_lists {
                m.tidal = self.tidal_lists.clone();
            }
            m.tidal_on = tidal_on;
            std::mem::take(&mut m.want_tidal)
        });
        if want && tidal_on {
            self.load_tidal_lists();
        }
        // (and the songs your other computer may copy from this one)
        self.share_files();
    }

    /// Fetch your own Tidal playlists in the background (once; they're kept till something changes them).
    pub(crate) fn load_tidal_lists(&mut self) {
        if self.tidal_lists.is_some() || self.tidal_lists_busy {
            return;
        }
        self.tidal_lists_busy = true;
        let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(Msg::TidalLists(api.own_playlists()));
            ctx.request_repaint();
        });
    }

    // ------------------------------------------------------------- loading
    fn load(&mut self, push: bool, f: impl FnOnce(&Api) -> Result<Page, String> + Send + 'static) {
        self.loading = true;
        let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let _ = match f(&api) {
                Ok(p) => tx.send(Msg::Page(p, push)),
                Err(e) => tx.send(Msg::Err(e)),
            };
            ctx.request_repaint();
        });
    }

    /// Look the library over, quietly, when it was loaded or looked at more than a few minutes ago: what changed on
    /// your phone or the web (a song liked, a playlist made) comes in without reloading anything. Called on coming back
    /// to the window and on opening the library or one of its tabs; never on a timer.
    fn library_look(&mut self) {
        const STALE: Duration = Duration::from_secs(180);
        let stale = self.lib_at.is_some_and(|t| t.elapsed() > STALE);
        if !stale || self.lib_looking || self.lib_page.is_none() || self.auth != Auth::In || self.offline || self.net_down {
            return;
        }
        if self.tour_play.is_some() || tour::demo_kind().is_some() {
            return;
        }
        self.lib_looking = true;
        self.lib_at = Some(Instant::now());
        let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(Msg::LibraryHead(api.library_head()));
            ctx.request_repaint();
        });
    }

    /// What the look found, put in every copy of the library page (shown, kept, back, forward) without a reload: the
    /// lists as they are now, and the songs liked since at the top. Songs taken out elsewhere (fewer than it adds up
    /// to) read the whole library again, in the background.
    fn library_merge(&mut self, newest: Vec<Track>, total: usize, rows: Vec<(String, Vec<Card>)>) {
        let Some(lib) = self.lib_page.clone() else { return };
        let Some((songs, added)) = liked_top(&lib.tracks, lib.total.max(lib.tracks.len()), &newest, total) else {
            crate::api::log(&format!("library changed elsewhere ({} now, had {}): reading it again", total, lib.total));
            self.load_quietly();
            return;
        };
        // (the songs in the same order as before: nothing to change there)
        let same_order = songs.len() == lib.tracks.len() && songs.iter().zip(lib.tracks.iter()).all(|(a, b)| a.id == b.id);
        let lists_same = rows.iter().all(|(n, c)| lib.rows.iter().any(|(m, d)| m == n && d.len() == c.len() && d.iter().zip(c).all(|(a, b)| a.id == b.id && a.title == b.title)));
        if same_order && lists_same && rows.len() == lib.rows.iter().filter(|r| r.0 != api::NEW_FROM_FOLLOWED).count() {
            return;
        }
        crate::api::log(&format!(
            "library changed elsewhere: {} new song(s){}, lists {}",
            added,
            if same_order || added > 0 { "" } else { ", some moved up" },
            if lists_same { "the same" } else { "changed" }
        ));
        self.liked.extend(newest.iter().map(|t| t.id));
        let songs = Arc::new(songs);
        let pages = self.page.iter_mut().chain(self.lib_page.iter_mut()).chain(self.back.iter_mut()).chain(self.fwd.iter_mut());
        for p in pages.filter(|p| p.library) {
            let pg = Arc::make_mut(p);
            if !same_order {
                pg.tracks = songs.as_ref().clone();
                pg.total = total;
            }
            // the lists as they are now (NEW FROM ARTISTS YOU FOLLOW stays as it was)
            let followed: Vec<(String, Vec<Card>)> = pg.rows.drain(..).filter(|r| r.0 == api::NEW_FROM_FOLLOWED).collect();
            pg.rows = rows.clone();
            pg.rows.extend(followed);
        }
        // (the playlists the menus offer are read again when next wanted)
        self.tidal_lists = None;
    }

    /// The whole library read again in the background, put in place of the old one once it's all in (no reload to
    /// see, no counting up again).
    fn load_quietly(&mut self) {
        let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
        self.lib_looking = true;
        std::thread::spawn(move || {
            let r = api.library().map(|mut p| {
                while p.tracks.len() < p.total {
                    match api.fav_tracks(p.tracks.len()) {
                        Ok((t, _, raw)) if raw > 0 => p.tracks.extend(t),
                        _ => break,
                    }
                }
                p
            });
            if let Ok(p) = r {
                let _ = tx.send(Msg::LibraryWhole(p));
                ctx.request_repaint();
            }
        });
    }

    /// Fetch the rest of the liked songs in the background (the first 100 arrive with the page).
    fn spawn_more_tracks(&self, start: usize, total: usize, gen: u64) {
        let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
        LIB_LOAD.store(if start < total { 0 } else { 1 }, Ordering::Relaxed);
        std::thread::spawn(move || {
            let mut off = start;
            let mut ok = true;
            while off < total {
                match api.fav_tracks(off) {
                    Ok((t, _total, raw)) => {
                        if raw == 0 {
                            break;
                        }
                        off += raw;
                        let _ = tx.send(Msg::MoreTracks(gen, t));
                        ctx.request_repaint();
                    }
                    Err(e) => {
                        crate::api::log(&format!("liked songs: stopped at {} ({})", off, e));
                        ok = false;
                        break;
                    }
                }
            }
            let _ = tx.send(Msg::MoreTracksDone(gen, ok));
            ctx.request_repaint();
        });
    }

    fn fetch_lyrics(&mut self, id: i64) {
        // only Tidal has lyrics: a file, YouTube or SoundCloud song simply has none (no asking)
        if id <= 0 {
            self.lyrics = Some(Lyrics { id, status: 2, lines: Vec::new(), synced: false });
            return;
        }
        self.lyrics = Some(Lyrics { id, status: 0, lines: Vec::new(), synced: false });
        self.lyric_scroll = 0.0;
        let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let r = api.lyrics(id);
            let _ = tx.send(Msg::Lyrics(id, r));
            ctx.request_repaint();
        });
    }

    fn after_login(&mut self) {
        if self.net_down {
            self.net_down = false;
            self.images.retry_failed();
            self.set_note("BACK ONLINE. TIDAL IS HERE AGAIN");
        }
        self.auth = Auth::In;
        self.offline = false;
        self.dirty = true;
        self.login_code = None;
        self.back.clear();
        self.fwd.clear();
        crate::api::log("logged in; loading library");
        self.load(false, |a| a.library());
    }

    fn start_login(&mut self) {
        self.auth = Auth::LoggingIn;
        self.login_err.clear();
        let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            match api.start_login() {
                Ok(dc) => {
                    let _ = webbrowser::open(&dc.url);
                    let _ = tx.send(Msg::Code(dc.user_code.clone(), dc.url.clone()));
                    ctx.request_repaint();
                    match api.finish_login(&dc) {
                        Ok(()) => {
                            let _ = api.refresh_info();
                            let _ = tx.send(Msg::LoggedIn);
                        }
                        Err(e) => {
                            let _ = tx.send(Msg::NeedLogin(e));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Msg::NeedLogin(e));
                }
            }
            ctx.request_repaint();
        });
    }

    // ------------------------------------------------------------ playback
    fn set_kbps(&mut self, bytes: usize) {
        let dur = self.cur_track().map(|t| t.duration).unwrap_or(0.0);
        self.kbps = if dur > 1.0 { (bytes as f32 * 8.0 / 1000.0 / dur) as u32 } else { 0 };
    }

    /// A song with saved loops or bookmarks: it stays on this computer, so practising it never waits on Tidal.
    pub(crate) fn practised(&self, id: i64) -> bool {
        self.store.sections.get(&id).is_some_and(|l| !l.is_empty()) || self.store.marks.get(&id).is_some_and(|l| !l.is_empty())
    }

    /// Keep this song on this computer (in the background), now that it has loops or bookmarks.
    pub(crate) fn keep_for_practice(&mut self, id: i64) {
        if cache::has(id) || matches!(self.srcmap.get(&id), Some(Src::File(_))) || self.is_demo(id) {
            return;
        }
        // (one at a time per song: a bookmark after a bookmark doesn't start a second download)
        if !self.keeping.insert(id) {
            return;
        }
        let lossless = self.prefer_lossless;
        let src = self.srcmap.get(&id).cloned();
        let api = self.api.clone();
        let have = self.playing_bytes(id);
        std::thread::spawn(move || match have {
            Some(b) => {
                cache::put(id, &b);
            }
            None => {
                let _ = fetch_audio(&api, id, lossless, true, src);
            }
        });
    }

    /// The playing song's audio as the player has it, when `id` is that song (so it isn't fetched again).
    pub(crate) fn playing_bytes(&self, id: i64) -> Option<Arc<Vec<u8>>> {
        if self.cur_track().map(|t| t.id) != Some(id) || self.buffering || self.stopped {
            return None;
        }
        self.player.viz.lock().ok()?.song.as_ref().map(|s| s.1.clone())
    }

    /// The songs you queued by hand (PLAY NEXT / ADD TO QUEUE) still ahead of the one playing, in their order, for a
    /// new list to carry on with (all but `starting`, the song the new list begins with); the rest of your queued
    /// copies are forgotten with the old list.
    fn queued_ahead(&mut self, starting: i64) -> Vec<Track> {
        let from = self.cur.map_or(0, |c| c + 1);
        let kept: Vec<Track> = self
            .queue
            .iter()
            .skip(from)
            .filter(|t| t.qid != 0 && self.q_added.contains(&t.qid) && t.id != starting)
            .cloned()
            .collect();
        self.q_added.retain(|q| kept.iter().any(|t| t.qid == *q));
        kept
    }

    /// On to song `i` because the one playing finished or was skipped (NEXT): a queued copy that was playing leaves
    /// the queue. Any other way of picking a song (clicking around in the playlist) leaves the queue as it is.
    fn advance_to(&mut self, i: usize) {
        self.advancing = true;
        self.play_index(i);
        self.advancing = false;
    }

    fn play_index(&mut self, mut i: usize) {
        // the next song in line (not a jump): one fewer of your queued songs ahead
        let next_in_line = self.cur.is_some_and(|c| i == c + 1);
        if let Some(c) = self.cur.filter(|c| i > *c && self.q_drop && self.advancing) {
            if let Some(q) = self.queue.get(c).map(|t| t.qid).filter(|q| *q != 0 && self.q_added.remove(q)) {
                Arc::make_mut(&mut self.queue).remove(c);
                if let Some(o) = self.orig_queue.as_mut() {
                    if let Some(k) = o.iter().position(|t| t.qid == q) {
                        o.remove(k);
                    }
                }
                i -= 1;
            }
        }
        let Some(t) = self.queue.get(i).cloned() else { return };
        // a song Tidal can't stream any more, with no copy here: on to the next one
        if t.gone && !cache::has(t.id) {
            if let Some(k) = (i + 1..self.queue.len()).find(|&k| !(self.queue[k].gone && !cache::has(self.queue[k].id))) {
                self.play_index(k);
            } else {
                self.set_note("NO LONGER AVAILABLE ON TIDAL");
            }
            return;
        }
        // a file of yours that was deleted or moved: out of the lists, and on to the next song
        if let Some(Src::File(p)) = self.srcmap.get(&t.id) {
            if !p.exists() {
                let name = format!("{} - {}", t.artist, t.title);
                // a song on your other computer's disk (it came in the sync): not gone, just not on this one
                if sync::linked() && !self.store.files.iter().any(|e| e.id == t.id) {
                    // played from there, or copied here, or asked which
                    let there = p.to_string_lossy().into_owned();
                    self.song_elsewhere(&t, there);
                    return;
                }
                self.apply(Action::FileGone(t.id));
                self.set_note(&format!("GONE FROM DISK, REMOVED: {}", name.to_uppercase()));
                if i < self.queue.len() {
                    self.play_index(i);
                }
                return;
            }
        }
        self.q_manual = if next_in_line { self.q_manual.saturating_sub(1) } else { 0 };
        self.cur = Some(i);
        self.on_track_change(&t);
        self.q_scroll = true;
        self.play_gen += 1;
        let g = self.play_gen;
        self.player.send(Cmd::Stop);
        self.status.clear();
        self.stopped = false;
        self.paused = false;
        self.audio_err_shown = false;
        crate::api::log(&format!("play: '{}' - '{}' (lossless: {})", t.artist, t.title, self.prefer_lossless));
        // kept for RECENT (the same song again moves to the top)
        if !self.is_demo(t.id) {
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
            if !matches!(self.srcmap.get(&t.id), Some(Src::File(_))) && !self.is_demo(t.id) {
                // (its stored copy is named by number only: its details go into the file's tags)
                cache::know(std::slice::from_ref(&t));
            }
            let v = self.version_of(&t);
            self.store.recent.retain(|(r, _)| r.id != t.id);
            self.store.recent.insert(0, (v, now));
            self.store.recent.truncate(500);
            self.store_dirty = true;
        }

        if let Some((id, bytes)) = self.prefetched.take() {
            if id == t.id {
                let n = bytes.len();
                self.start_wave(id, &bytes);
                self.start_meta(id, &bytes);
                self.start_beats(id, &bytes);
                self.player.send(Cmd::Play(bytes));
                self.after_play();
                self.set_kbps(n);
                self.buffering = false;
                self.prefetch_next(id);
                return;
            }
        }
        self.buffering = true;
        let lossless = self.prefer_lossless;
        let keep = self.keep_cache || self.practised(t.id);
        let src = self.srcmap.get(&t.id).cloned();
        // (a YouTube song not saved here comes down whole before it plays)
        if matches!(src, Some(Src::Yt(_))) && !cache::has(t.id) {
            self.set_note("GETTING THE SONG FROM YOUTUBE...");
        }
        let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let r = fetch_audio(&api, t.id, lossless, keep, src);
            let _ = match r {
                Ok(b) => tx.send(Msg::Audio(g, t.id, b)),
                Err(e) => tx.send(Msg::PlayErr(g, e)),
            };
            ctx.request_repaint();
        });
    }

    fn next_index(&self) -> Option<usize> {
        let cur = self.cur?;
        let n = self.queue.len();
        if n == 0 {
            return None;
        }
        if cur + 1 < n {
            Some(cur + 1)
        } else if self.repeat == Repeat::All {
            Some(0)
        } else {
            None
        }
    }

    fn next_track(&self) -> Option<Track> {
        if self.repeat == Repeat::One {
            return self.cur_track();
        }
        let cur = self.cur?;
        self.queue.get(cur + 1).cloned().or_else(|| if self.repeat == Repeat::All { self.queue.first().cloned() } else { None })
    }

    fn prefetch_next(&mut self, _playing_id: i64) {
        if self.repeat == Repeat::One {
            return;
        }
        let Some(cur) = self.cur else { return };
        let Some(next) = self.queue.get(cur + 1) else { return };
        if self.prefetching == next.id || self.prefetched.as_ref().map(|p| p.0) == Some(next.id) {
            return;
        }
        self.prefetching = next.id;
        let id = next.id;
        let lossless = self.prefer_lossless;
        let keep = self.keep_cache || self.practised(id);
        let src = self.srcmap.get(&id).cloned();
        let (api, tx, ctx) = (self.api.clone(), self.tx.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            if let Ok(b) = fetch_audio(&api, id, lossless, keep, src) {
                let _ = tx.send(Msg::Prefetched(id, b));
                ctx.request_repaint();
            }
        });
    }

    /// The queue order changed: forget the old prefetch and fetch the new "next" track.
    fn refresh_prefetch(&mut self) {
        self.prefetched = None;
        self.prefetching = 0;
        if self.cur.is_some() && !self.stopped && !self.buffering {
            self.prefetch_next(0);
        }
    }

    fn pos(&self) -> f32 {
        self.player.shared.lock().unwrap().pos
    }

    fn cur_track(&self) -> Option<Track> {
        self.cur.and_then(|i| self.queue.get(i).cloned())
    }

    fn set_shuffle(&mut self, on: bool) {
        if on == self.shuffle {
            return;
        }
        self.shuffle = on;
        if on {
            let orig: Vec<Track> = (*self.queue).clone();
            let new = match self.cur {
                Some(c) if c < orig.len() => {
                    self.cur = Some(0);
                    shuffled_from(&orig, c)
                }
                _ => {
                    let mut v = orig.clone();
                    shuffle_vec(&mut v);
                    v
                }
            };
            self.orig_queue = Some(orig);
            self.queue = Arc::new(new);
        } else if let Some(orig) = self.orig_queue.take() {
            let cur_id = self.cur_track().map(|t| t.id);
            let pos = cur_id.and_then(|id| orig.iter().position(|t| t.id == id));
            self.queue = Arc::new(orig);
            if self.cur.is_some() {
                self.cur = Some(pos.unwrap_or(0));
            }
        }
        self.q_scroll = true;
        self.refresh_prefetch();
    }

    /// Bar colours for the visualizer: None = the skin's own, else (main, cap).
    fn viz_tint(&self, color: u8) -> Option<(Color32, Color32)> {
        if color == 0 {
            if let Some(v) = self.wa_art.as_ref().map(|t| &t.vis).filter(|v| v.len() >= 24) {
                return Some((v[12], v[2]));
            }
        }
        match color {
            2 => Some((Color32::from_rgb(70, 220, 110), Color32::from_rgb(255, 72, 60))),
            1 => {
                let t = self.cur_track()?;
                let avg = self
                    .images
                    .avg
                    .get(&cover_url(&t.cover, 640))
                    .or_else(|| self.images.avg.get(&cover_url(&t.cover, 160)))
                    .copied()?;
                // the cover's own colour lifted to a readable level, with a gentle complementary tip
                let (h, sat, _) = rgb_to_hsv(avg[1]);
                let (h0, s0, _) = rgb_to_hsv(avg[0]);
                let h = if sat < 0.12 { h0 } else { h };
                let s = if sat < 0.12 { s0 } else { sat };
                let pri = hsv_to_rgb(h, s.clamp(0.35, 0.85), 0.9);
                let sec = hsv_to_rgb((h + 0.42) % 1.0, 0.5, 1.0);
                Some((pri, sec))
            }
            _ => None,
        }
    }

    /// Bands, peaks and wave shaped for one visualizer: its own bar count and sensitivity.
    fn viz_data(&self, i: usize) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
        let c = self.viz[i];
        let k = c.gain / self.viz[0].gain.max(self.viz[1].gain);
        (viz::resample(&self.bands, c.n, k), viz::resample(&self.peaks, c.n, k), self.viz_wave.iter().map(|v| v * k).collect())
    }

    fn update_bands(&mut self) {
        let active = self.cur.is_some() && !self.paused && !self.stopped && !self.buffering;
        let (samples, rate) = if active {
            let v = self.player.viz.lock().unwrap();
            let n = v.samples.len().min(2048);
            (v.samples[v.samples.len() - n..].to_vec(), v.rate as f32)
        } else {
            (Vec::new(), 44100.0)
        };
        // analysed once at the finer of the two bar counts and the higher sensitivity; each view reshapes it
        let nb = self.viz[0].n.max(self.viz[1].n).clamp(8, 96);
        let gain = self.viz[0].gain.max(self.viz[1].gain);
        analyse(&mut self.bands, &mut self.peaks, &mut self.viz_wave, &samples, rate, nb, gain);
    }

    /// A line's sound: the notes, `gap` seconds apart, over its backing - nothing, the root held, the chord
    /// strummed each bar, or the band on the chord (four beats a bar at the line's tempo).
    fn line_pcm(&self, notes: &[Vec<i32>], gap: f32) -> Vec<i16> {
        let line = band::line_sound(notes, gap);
        let Some((mode, chord)) = self.ln_backing.as_ref().filter(|b| b.0 > 0 && !b.1.is_empty()) else { return line };
        let bar = 4.0 * 60.0 / self.ln_bpm.max(20) as f32;
        let len = notes.len() as f32 * gap;
        let bars = (len / bar).ceil().max(1.0) as usize;
        let back: Vec<i16> = match (mode, self.ln_prog.as_ref().filter(|p| !p.2.is_empty())) {
            // over changes: the band plays the chart; the drone and the chord follow it, struck as each chord comes
            (3, Some((text, order, _))) => {
                let chart = crate::chart::parse_friendly(text);
                let lv = [0, 1, 2].map(|i| if self.band_parts[i] { self.band_lv[i] } else { 0.0 });
                band::render(&chart, order, self.ln_bpm as f32, self.band_style, lv, false)
            }
            (_, Some((_, _, chords))) => {
                let beat = 60.0 / self.ln_bpm.max(20) as f32;
                let mut mix: Vec<f32> = Vec::new();
                let mut t = 0.0;
                for (notes, beats) in chords {
                    let d = beats * beat;
                    let strike: Vec<i32> = if *mode == 1 { vec![notes[0] - 12] } else { notes.clone() };
                    let at = (t * band::RATE) as usize;
                    for (i, x) in band::sequence_sound(&[strike], d).into_iter().enumerate() {
                        if mix.len() <= at + i {
                            mix.resize(at + i + 1, 0.0);
                        }
                        mix[at + i] += x as f32;
                    }
                    t += d;
                }
                mix.into_iter().map(|x| x.clamp(-32768.0, 32767.0) as i16).collect()
            }
            (mode, None) => match mode {
                // the root, low, struck each bar so it never dies away
                1 => band::sequence_sound(&vec![vec![chord[0] - 12]; bars], bar),
                // the chord, strummed on the one of each bar
                2 => band::sequence_sound(&vec![chord.clone(); bars], bar),
                // the band on the chord
                _ => {
                    let name = format!(
                        "{}{}",
                        theory::note_name(chord[0], lines::key_sharps(chord[0])),
                        if chord.get(1).is_some_and(|t| t - chord[0] == 3) { "m7" } else { "6" }
                    );
                    let text = format!("T44 {}|", format!("| {} ", name).repeat(bars));
                    let chart = crate::chart::parse_friendly(&text);
                    let order: Vec<usize> = (0..chart.bars.len()).collect();
                    let lv = [0, 1, 2].map(|i| if self.band_parts[i] { self.band_lv[i] } else { 0.0 });
                    band::render(&chart, &order, self.ln_bpm as f32, self.band_style, lv, false)
                }
            },
        };
        // the two together; the line a little louder than what it sits on
        let n = line.len().max(back.len());
        (0..n)
            .map(|i| {
                let a = line.get(i).copied().unwrap_or(0) as f32;
                let b = back.get(i).copied().unwrap_or(0) as f32 * 0.6;
                (a + b).clamp(-32768.0, 32767.0) as i16
            })
            .collect()
    }

    /// The opening sound, as set in Preferences.
    fn play_intro(&self) {
        let file = match self.intro {
            0 => return,
            2 => self.intro_file.clone(),
            _ => None,
        };
        intro::play(file, self.intro_sound as usize, self.out_device.clone(), (self.volume * self.volume).max(0.15));
    }
}

/// Control-click is a right-click (a Mac's way, for a mouse or trackpad with one button): the press and its release
/// both go to the right button. `held`: a press was turned, so its release is too.
fn ctrl_click_is_right_click(events: &mut [egui::Event], held: &mut bool) {
    for e in events.iter_mut() {
        if let egui::Event::PointerButton { button, pressed, modifiers, .. } = e {
            if *button != egui::PointerButton::Primary {
                continue;
            }
            if *pressed && modifiers.ctrl && !modifiers.mac_cmd {
                *button = egui::PointerButton::Secondary;
                *held = true;
            } else if !*pressed && *held {
                *button = egui::PointerButton::Secondary;
                *held = false;
            }
        }
    }
}

impl eframe::App for App {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        if cfg!(target_os = "macos") {
            static HELD: AtomicBool = AtomicBool::new(false);
            let mut held = HELD.load(Ordering::Relaxed);
            ctrl_click_is_right_click(&mut raw.events, &mut held);
            HELD.store(held, Ordering::Relaxed);
        }
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // a test copy can picture itself (BACKLINE_SHOT=file.png, after BACKLINE_SHOT_AT seconds): for checking layouts
        // when no one is at the screen
        test_shot(ctx);
        // can anyone see the window? (minimized or covered: the visuals and the redraws while playing stop)
        onscreen::check(ctx, frame);
        // (a test copy says how often its main window draws, every few seconds)
        if hidden_test_copy() {
            static DRAWN: std::sync::Mutex<(u32, f64)> = std::sync::Mutex::new((0, 0.0));
            let now = ctx.input(|i| i.time);
            let mut d = DRAWN.lock().unwrap_or_else(|e| e.into_inner());
            d.0 += 1;
            // (BACKLINE_POP_LOG=<seconds>: how often; 5 if not set)
            let every = std::env::var("BACKLINE_POP_LOG").ok().and_then(|v| v.parse::<f64>().ok()).unwrap_or(5.0);
            if now - d.1 >= every {
                if d.1 > 0.0 {
                    crate::api::log(&format!("main window: {:.0} frames a second", d.0 as f64 / (now - d.1)));
                }
                *d = (0, now);
            }
        }
        TOUR_ON.store(self.tour.is_some(), std::sync::atomic::Ordering::Relaxed);
        // a skin worn in the full window (Windows): no system title bar - the skin's own bars are the app's, as a
        // modern app's are (the mini player decides its own, below)
        let own_bar = cfg!(windows) && !self.mini && self.wa_art.is_some();
        APP_BAR.store(own_bar, std::sync::atomic::Ordering::Relaxed);
        if !self.mini && self.deco_now != Some(!own_bar) {
            self.deco_now = Some(!own_bar);
            // (the inside kept its size: the bar coming off or on would otherwise grow or shrink it, a little more
            // every start)
            let (inner, max) =
                ctx.input(|i| (i.viewport().inner_rect.map(|r| r.size()), i.viewport().maximized.unwrap_or(false)));
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(!own_bar));
            if let (Some(sz), false) = (inner, max || self.fullscreen) {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(sz));
            }
        }
        // ctrl + / - / 0 size the whole app - except over the library, where they zoom the library (done there)
        ctx.options_mut(|o| o.zoom_with_keyboard = false);
        let over_lib = ctx.input(|i| i.pointer.hover_pos()).is_some_and(|p| self.lib_rect.contains(p));
        if !over_lib {
            let (plus, minus, zero) = ctx.input_mut(|i| {
                let m = i.modifiers;
                if !m.command {
                    return (false, false, false);
                }
                (
                    i.consume_key(m, egui::Key::Plus) | i.consume_key(m, egui::Key::Equals),
                    i.consume_key(m, egui::Key::Minus),
                    i.consume_key(m, egui::Key::Num0),
                )
            });
            let z = ctx.zoom_factor();
            let nz = if zero {
                1.0
            } else if plus {
                z + 0.1
            } else if minus {
                z - 0.1
            } else {
                z
            };
            if nz != z {
                ctx.set_zoom_factor(nz.clamp(0.8, 2.0));
                self.dirty = true;
            }
        }
        // This app draws and handles its own buttons and text boxes. egui's own keyboard focus would make Enter / Space
        // "press" whichever button was clicked last (typing a loop time and pressing Enter also hit the + next to it), so drop it.
        if let Some(id) = ctx.memory(|mem| mem.focused()) {
            ctx.memory_mut(|mem| mem.surrender_focus(id));
        }
        font::set_ppp(ctx.pixels_per_point());
        // smooth text in the sleek looks, and wherever a skin's frames are on (in the skin's own font)
        font::set_modern(winamp_ui::chrome_on());
        // the mini window's size once more, after the title bar has come off
        if let Some((at, size)) = self.mini_size_at {
            if at.elapsed() > Duration::from_millis(300) {
                self.mini_size_at = None;
                if self.mini {
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
                }
            } else {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }
        tour::begin_frame();
        // the window's scale and size settle over the first frames: keep drawing until they do
        {
            let (ppp, sz) = (ctx.pixels_per_point(), ctx.screen_rect().size());
            if (ppp - self.last_ppp).abs() > 0.001 || sz != self.last_size || self.frames < 10 {
                self.last_ppp = ppp;
                self.last_size = sz;
                self.frames += 1;
                ctx.request_repaint();
            }
        }
        self.drain(ctx);
        // the covers the rows asked for last frame: ready to draw this one (or known to have no picture)
        let wants: Vec<String> = ART_WANT.with(|w| std::mem::take(&mut *w.borrow_mut()));
        let mut ready: HashMap<String, Option<egui::TextureId>> = HashMap::new();
        for u in wants {
            if !ready.contains_key(&u) {
                match self.images.get(&u) {
                    Some(id) => {
                        ready.insert(u, Some(id));
                    }
                    None if self.images.failed(&u) => {
                        ready.insert(u, None);
                    }
                    None => {}
                }
            }
        }
        ART_TEX.with(|t| *t.borrow_mut() = ready);
        self.sync_pl_menu();
        // back to the window after a while: the library looked over, quietly (what changed on your phone comes in)
        let focused = ctx.input(|i| i.focused);
        if focused && !self.was_focused {
            self.library_look();
        }
        self.was_focused = focused;
        // the visuals' own window, while it is open (it draws on a thread of its own)
        self.pop_frame(ctx);
        let mut acts: Vec<Action> = Vec::new();

        // files / folders dropped onto the window
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if let (Some(d), Some(pic)) = (self.new_pl_dlg.as_mut(), dropped.iter().find(|p| new_playlist::is_picture(p))) {
            d.art = pic.to_string_lossy().into_owned();
        } else if !dropped.is_empty() {
            self.handle_paths(dropped);
        }
        let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());

        // Player events: errors and end-of-track
        {
            let (ended, err) = {
                let mut g = self.player.shared.lock().unwrap();
                let e = g.ended;
                g.ended = false;
                (e, g.error.take())
            };
            if let Some(e) = err {
                self.buffering = false;
                if e.starts_with("DECODE") {
                    // a damaged stored copy must not keep failing: forget it so it is fetched again
                    if let Some(t) = self.cur_track() {
                        cache::remove(t.id);
                    }
                }
                if e.starts_with("DECODE") && self.prefer_lossless {
                    api::log(&format!("lossless decode failed ({}), retrying as AAC", e));
                    self.prefer_lossless = false;
                    self.prefetched = None;
                    self.prefetching = 0;
                    if let Some(c) = self.cur {
                        self.play_index(c);
                    }
                    self.set_note("LOSSLESS FAILED, SO USING 320K AAC");
                } else {
                    self.stopped = true;
                    self.set_err(e);
                }
            }
            if ended && !self.buffering && !self.stopped {
                if self.repeat == Repeat::One {
                    if let Some(c) = self.cur {
                        self.play_index(c);
                    }
                } else if let Some(j) = self.next_index() {
                    self.advance_to(j);
                } else {
                    self.stopped = true;
                }
            }
        }
        let dead = self.player.shared.lock().unwrap().dead.clone();
        if let Some(d) = dead {
            if !self.audio_err_shown && self.cur.is_some() && !self.stopped {
                self.audio_err_shown = true;
                self.stopped = true;
                self.buffering = false;
                self.set_err(d);
            }
        }
        self.update_bands();
        self.sleep_tick();
        self.tick_practice();
        let mut title = match self.cur_track() {
            Some(t) if !self.stopped => {
                format!("{} - {} ({})", t.artists_text(), t.title, if self.paused { "Paused" } else { "Now Playing" })
            }
            _ => "Backline".to_string(),
        };
        // the same in Windows' media flyout: playing whenever sound plays (a practice loop too), paused, or nothing
        let now: media::Shown = match self.cur_track().filter(|_| !self.stopped) {
            Some(t) => (
                Some((
                    t.artists_text(),
                    t.title.clone(),
                    Some(cover_url(&t.cover, 320)).filter(|c| c.starts_with("http")).unwrap_or_default(),
                )),
                if self.paused { 2 } else { 1 },
            ),
            None => (None, 0),
        };
        let playing = now.1 == 1;
        if let Some(m) = self.media.as_mut() {
            media::show(m, now);
        }
        // the taskbar buttons, once the taskbar has made the window's button (a couple of seconds in)
        if !self.thumb_tried && self.media.is_some() && ctx.input(|i| i.time) > 2.0 {
            self.thumb_tried = true;
            self.thumb = media::thumb_bar(self.tx.clone(), ctx.clone());
        }
        if let Some(tb) = self.thumb.as_mut() {
            media::thumb_show(tb, playing);
        }
        if let (true, Some(end)) = (self.pomo > 0, self.pomo_end) {
            let left = end.saturating_duration_since(Instant::now()).as_secs();
            title = format!("{}:{:02} {}  |  {}", left / 60, left % 60, if self.pomo == 1 { "FOCUS" } else { "BREAK" }, title);
        } else if self.pomo == 3 {
            title = format!("BREAK OVER, NEXT FOCUS  |  {}", title);
        }
        if self.pomo_end.is_some() {
            ctx.request_repaint_after(Duration::from_millis(500));
        }
        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
        if COLS_CHANGED.swap(false, Ordering::Relaxed) {
            self.dirty = true;
        }
        if self.dirty && self.last_save.elapsed() > Duration::from_secs(1) {
            self.save_settings();
        }

        if self.auth != Auth::In && !self.offline {
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(pal().app_bg))
                .show(ctx, |ui| self.login_ui(ui, &mut acts));
        } else {
            // Keyboard: Space = play/pause, arrows = seek, classic Winamp keys Z X C V B,
            // A = art viewer, G = black & white, L = lyrics, F / F11 = fullscreen, Esc = back.
            // (a panel open: the keys and the mouse's side buttons are its own; only the keyboard's media keys and the
            // pop-out visuals' buttons, which don't come this way, still reach the player)
            let panel = self.panel_open();
            if !self.search_focus && self.ed.id == 0 && !panel {
                let kp = |k: egui::Key| ctx.input(|i| i.key_pressed(k));
                for cmd in self.key_commands(ctx) {
                    self.run_cmd(cmd, &mut acts);
                }
                if kp(egui::Key::Escape) {
                    if self.fullscreen {
                        acts.push(Action::ToggleFullscreen);
                    } else if self.art_view {
                        acts.push(Action::ToggleArt);
                    }
                }
            }
            if !panel && ctx.input(|i| i.key_pressed(egui::Key::F11)) {
                acts.push(Action::ToggleFullscreen);
            }
            // the mouse's side buttons: back closes the album view, otherwise both walk the page history
            let (m_back, m_fwd) = ctx.input(|i| {
                (i.pointer.button_pressed(egui::PointerButton::Extra1), i.pointer.button_pressed(egui::PointerButton::Extra2))
            });
            let (m_back, m_fwd) = (m_back && !panel, m_fwd && !panel);
            if m_back && self.art_view {
                acts.push(Action::ToggleArt);
            } else if m_back {
                acts.push(Action::Back);
            }
            if m_fwd && !self.art_view {
                acts.push(Action::Forward);
            }

            // lyrics are fetched on demand, only while the viewer is open
            if self.art_view && self.show_lyrics {
                if let Some(t) = self.cur_track() {
                    if self.lyrics.as_ref().map(|l| l.id) != Some(t.id) {
                        self.fetch_lyrics(t.id);
                    }
                }
            }

            self.vis_idle_check(ctx);
            if self.vis_full {
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(Color32::BLACK))
                    .show(ctx, |ui| self.vis_full_ui(ui, &mut acts));
            } else if self.mini {
                self.mini_stack_size(ctx);
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(pal().app_bg))
                    .show(ctx, |ui| self.mini_window(ui, &mut acts));
            } else if self.art_view {
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(pal().app_bg))
                    .show(ctx, |ui| self.art_ui(ui, &mut acts));
            } else {
                self.main_layout(ctx, &mut acts);
            }
        }

        if hovering {
            let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("drop_hint"));
            let p = ctx.layer_painter(layer);
            let r = ctx.screen_rect();
            p.rect_filled(r, Rounding::same(0.0), Color32::from_black_alpha(170));
            ptext(&p, r.center(), Align::Center, "DROP AUDIO FILES OR FOLDERS TO ADD THEM", 3.0, pal().bar_txt);
        }
        self.floating_tools(ctx, &mut acts);
        // the window can also go fullscreen/windowed by itself (the green Mac button, F11): follow it,
        // except for a moment after our own command, while the window reports the old state
        if self.fs_cmd_at.is_none_or(|t| t.elapsed() > Duration::from_millis(1500)) {
            if let Some(f) = ctx.input(|i| i.viewport().fullscreen) {
                self.fullscreen = f;
                self.fs_by_art &= f;
            }
        }
        // left the album view (Esc, A, a mode button...) after going fullscreen in it: back to the window
        if self.fs_by_art && !self.art_view {
            self.apply(Action::ToggleFullscreen);
        }
        #[cfg(target_os = "macos")]
        if self.frames == 3 {
            self.menu_keep = macmenu::install(&self.tx, &self.ctx);
        }
        self.update_banner(ctx, &mut acts);
        self.floating_modes(ctx, &mut acts);
        self.help_overlay(ctx, &mut acts);
        self.credits_overlay(ctx);
        self.art_preview_overlay(ctx);
        self.new_playlist_panel(ctx, &mut acts);
        self.picker_panel(ctx, &mut acts);
        self.peer_ask_panel(ctx);
        self.dupe_panel(ctx, &mut acts);
        self.queue_ask_panel(ctx);
        self.queue_save_panel(ctx, &mut acts);
        self.wa_preview_panel(ctx, &mut acts);
        self.edit_info_panel(ctx);
        self.takes_panel(ctx);
        self.skin_loading_panel(ctx);
        self.viz_panel(ctx);
        self.share_panel(ctx, &mut acts);
        self.pack_panel(ctx, &mut acts);
        self.news_panel(ctx);
        self.artist_picker(ctx, &mut acts);
        self.palette_overlay(ctx);
        self.prefs_overlay(ctx, &mut acts);
        self.look_panel(ctx);
        // away from the Settings page, its skin lists fold shut again: next time they show as one line each
        if self.side != Some(Side::Backline) {
            self.skins_open = [false; 3];
        }
        // the first time in: once you've chosen Tidal or not, straight into the tour
        if !self.tour_seen && (self.auth == Auth::In || self.offline) && self.frames >= 10 && self.tour.is_none() {
            self.tour_seen = true;
            self.dirty = true;
            self.tour_start(0);
        }
        // the very first run wears Backline's first look
        if self.first_look {
            self.first_look = false;
            self.apply(Action::Look(FIRST_LOOK));
        }
        self.tour_overlay(ctx);
        self.demo_tick(ctx);
        if self.tour.is_none() && !self.show_prefs && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) {
            self.open_palette();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            acts.push(Action::ToggleHelp);
        }
        if self.show_help && self.tour.is_none() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_help = false;
        }
        if app_bar() {
            // (Now Playing draws its own bar, always there: `art_ui`)
            if self.vis_full && !self.mini && !self.fullscreen {
                floating_app_bar(ctx);
            }
            // (Windows sizes the window at its edges itself when it knows the title bar: frame_win)
            if !frame_win::active() {
                edge_resize(ctx);
            }
        }
        frame_win::end_frame(app_bar() && !self.fullscreen);
        for a in acts {
            self.apply(a);
        }
        self.hist_track();
        // keep the library copy current (more songs arrive in pages; likes change it)
        if self.page.as_ref().is_some_and(|p| p.library) && self.tour_play.is_none() {
            self.lib_page = self.page.clone();
            // the list, once it has finished coming in (some likes never arrive - gone from Tidal): kept on disk
            // for when there is no internet
            if let Some(p) = self.lib_page.as_ref().filter(|_| !self.net_down) {
                let n = p.tracks.len();
                if n != self.lib_seen.0 {
                    self.lib_seen = (n, Instant::now());
                    self.ctx.request_repaint_after(Duration::from_secs(6));
                } else if n != self.lib_saved_n && self.lib_seen.1.elapsed() > Duration::from_secs(5) {
                    self.lib_saved_n = n;
                    let tracks = p.tracks.clone();
                    std::thread::spawn(move || crate::api::save_liked(&tracks));
                }
            }
        }
        // (what notes boxes' tools were under the pointer this frame, for the next one's presses)
        self.ed.hold = std::mem::take(&mut self.ed.hold_next);
        // covers rows asked for that weren't drawn: asked for now, and one more frame when they are in already
        let more: Vec<String> =
            ART_WANT.with(|w| ART_TEX.with(|t| w.borrow().iter().filter(|u| !t.borrow().contains_key(*u)).cloned().collect()));
        for u in more {
            if self.images.get(&u).is_some() || self.images.failed(&u) {
                ctx.request_repaint();
            }
        }
        // (out of sight, a playing song needs no clock or spectrum drawn: the player says when it ends)
        let seen = onscreen::seen();
        let busy = self.cur.is_some() && !self.stopped && !self.paused && seen;
        // (fast only while something moves and can be seen, or the band or the metronome plays: they keep time with
        // the frames; out of sight and still, hardly at all: whatever comes in from the network asks for its own
        // redraw)
        let wait = Duration::from_millis(if self.art_view && busy {
            16
        } else if busy || self.band_on || self.mt.on || (seen && self.search_focus) {
            33
        } else if seen {
            250
        } else {
            1000
        });
        ctx.request_repaint_after(wait);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // the visuals' own window closes first, its thread letting go of its graphics
        self.pop_shut();
    }
}

fn main() -> eframe::Result<()> {
    std::panic::set_hook(Box::new(|info| {
        let loc = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "?".to_string()
        };
        crate::api::log(&format!("PANIC at {}: {}", loc, msg));
    }));
    // an update downloaded last time is installed before anything opens
    if update::apply_staged_at_start() {
        return Ok(());
    }
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Backline")
        .with_icon(Arc::new(app_icon()))
        .with_inner_size([1280.0, 820.0])
        .with_min_inner_size([980.0, 640.0]);
    // recording a demo: a fixed size, away to the side, never taking the keyboard from what you are doing
    if tour::demo_kind().is_some() {
        viewport = viewport
            .with_inner_size([1600.0, 900.0])
            .with_resizable(false)
            .with_position([-1700.0, 40.0])
            .with_active(false)
            .with_taskbar(false);
    } else if hidden_test_copy() {
        // a test copy (a test profile): out of sight, never in front of what you are doing or taking the keyboard
        // (far to the left of every screen: just to the left of one can be on a second screen there)
        // (BACKLINE_SIZE=WxH: its size, for pictures of a set size)
        viewport = viewport.with_position([-12000.0, 40.0]).with_active(false).with_taskbar(false);
        let size = std::env::var("BACKLINE_SIZE").ok().and_then(|v| {
            let (w, h) = v.split_once('x')?;
            Some([w.parse::<f32>().ok()?, h.parse::<f32>().ok()?])
        });
        if let Some(sz) = size {
            viewport = viewport.with_inner_size(sz).with_min_inner_size(sz).with_resizable(false);
        }
    }
    // a skin worn last time (Windows): the window opens without the system title bar, so it never shows and then
    // goes (the skin's own bars are the app's; "gen:0" is Backline's plain look, which keeps the system's)
    if cfg!(windows) {
        let st: serde_json::Value = std::fs::read_to_string(crate::api::config_dir().join("settings.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        if st["winamp"][0].as_str() != Some("gen:0") {
            viewport = viewport.with_decorations(false);
        }
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    crate::api::log(&format!("start: build {}", update::build()));
    let r = eframe::run_native("Backline", options, Box::new(|cc| Ok(Box::new(App::new(cc)))));
    // the window couldn't open (graphics, often after sleep or a driver update): say so, don't just vanish
    if let Err(e) = &r {
        crate::api::log(&format!("start: the window could not open: {}", e));
        rfd::MessageDialog::new()
            .set_title("Backline")
            .set_level(rfd::MessageLevel::Error)
            .set_description(format!(
                "Backline couldn't open its window.\n\n{}\n\nTry again in a moment. If it keeps happening, restarting the computer or updating the graphics driver usually fixes it. Details are in log.txt in %APPDATA%\\backline.",
                e
            ))
            .show();
    }
    // the stems runtime on a Mac crashes while the program winds down ("quit unexpectedly"), after everything is
    // saved: once it has been used, leave without that wind-down
    #[cfg(target_os = "macos")]
    if stems::LOADED.load(Ordering::Relaxed) {
        extern "C" {
            fn _exit(code: i32) -> !;
        }
        unsafe { _exit(if r.is_ok() { 0 } else { 1 }) }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tune_typed_short_finds_its_proper_title() {
        let found = [
            "Incompatibilidade de Gênios",
            "João Bosco & Aldir Blanc - Incompatibilidade de Gênios",
            "Incompatibilidade de Gênios (Ao Vivo)",
            "Incompatibilidade De Genios",
            "O Bêbado e a Equilibrista",
        ];
        let known: Vec<&str> = canon::SESSION_TUNES.iter().flat_map(|k| k.1.iter().map(|t| t.0)).collect();
        // short, or without its accents: the name the recordings go by
        for typed in ["Incompatibilidade de", "incompatibilidade de genios", "Incompatibilidad de Genios"] {
            assert_eq!(likely_title(typed, &found, &known).as_deref(), Some("Incompatibilidade de Gênios"), "{}", typed);
        }
        // already right (capitals aside): nothing to suggest
        assert_eq!(likely_title("Incompatibilidade de Gênios", &found, &known), None);
        assert_eq!(likely_title("incompatibilidade de gênios", &found, &known), None);
        // nothing like it
        assert_eq!(likely_title("Stablemates", &found, &[]), None);
        // offline: the known tunes alone
        assert_eq!(likely_title("Autum Leaves", &[], &known).as_deref(), Some("Autumn Leaves"));
        assert_eq!(likely_title("Stella by", &[], &known).as_deref(), Some("Stella by Starlight"));
    }

    #[test]
    fn library_filter_forgives() {
        let monk = ["Monk's Dream (Take 8)", "Thelonious Monk", "Monk's Dream"];
        assert!(matches_filter("monk dream", &monk));
        assert!(matches_filter("monks", &monk));
        assert!(matches_filter("thelonius", &monk), "a missing letter");
        assert!(matches_filter("dreem", &monk), "a typo");
        assert!(matches_filter("theloni", &monk), "the start of a word");
        assert!(matches_filter("shoga", &["Shōga", "Pat Metheny", "Moondial"]), "accents aside");
        assert!(!matches_filter("coltrane", &monk));
        assert!(!matches_filter("dog", &monk), "short words must be right");
        // the closest first: as typed, before a typo
        assert!(filter_score("monk", &monk) < filter_score("monc", &monk));
    }

    fn track(id: i64) -> Track {
        Track { id, title: format!("t{}", id), ..Default::default() }
    }

    #[test]
    fn times() {
        assert_eq!(fmt_time(0.0), "0:00");
        assert_eq!(fmt_time(83.9), "1:23");
        assert_eq!(fmt_time(-5.0), "0:00");
        assert_eq!(fmt_t(83.44), "1:23.4");
        assert_eq!(fmt_t(5.0), "0:05.0");
        assert_eq!(fmt_t(59.96), "1:00.0", "tenths round up into the next minute");
        assert_eq!(fmt_t(-1.0), "0:00.0");
        assert_eq!(fmt_long(59.0), "0:59");
        assert_eq!(fmt_long(3725.0), "1:02:05");
        // what fmt_t shows, the loop-time box reads back
        for s in [0.0, 1.5, 59.9, 83.4, 600.1] {
            assert!((crate::extras::parse_time(&fmt_t(s)).unwrap() - s).abs() < 0.051, "{}", s);
        }
    }

    #[test]
    fn shuffle_keeps_the_chosen_track_first() {
        let list: Vec<Track> = (1..=20).map(track).collect();
        for first in [0, 7, 19, 99] {
            let s = shuffled_from(&list, first);
            assert_eq!(s.len(), 20);
            assert_eq!(s[0].id, list[first.min(19)].id);
            let mut ids: Vec<i64> = s.iter().map(|t| t.id).collect();
            ids.sort();
            assert_eq!(ids, (1..=20).collect::<Vec<_>>(), "every track exactly once");
        }
        assert!(shuffled_from(&[], 0).is_empty());
    }

    #[test]
    fn lcd_digits() {
        // segments a-g; an 8 lights them all, a 1 only the right side
        assert_eq!(seg_mask(8), [1; 7]);
        assert_eq!(seg_mask(1), [0, 1, 1, 0, 0, 0, 0]);
        let lit: Vec<usize> = (0..10).map(|d| seg_mask(d).iter().filter(|&&s| s == 1).count()).collect();
        assert_eq!(lit, vec![6, 2, 5, 5, 4, 5, 6, 3, 7, 6]);
    }

    #[test]
    fn tone_strength() {
        let rate = 8000.0;
        let x: Vec<f32> = (0..4000).map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / rate).sin()).collect();
        assert!((goertzel(&x, 440.0, rate) - 1.0).abs() < 0.05, "a full-scale 440 Hz tone reads about 1");
        assert!(goertzel(&x, 1000.0, rate) < 0.05, "and nothing at 1 kHz");
    }

    #[test]
    fn control_click_opens_the_right_click_menu() {
        let ev = |pressed: bool, ctrl: bool, cmd: bool| egui::Event::PointerButton {
            pos: Pos2::ZERO,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers { ctrl, mac_cmd: cmd, command: cmd, ..Default::default() },
        };
        let button = |e: &egui::Event| match e {
            egui::Event::PointerButton { button, .. } => *button,
            _ => unreachable!(),
        };
        let mut held = false;
        // a Control-click, let go after Control is: both ends on the right button
        let mut evs = vec![ev(true, true, false), ev(false, false, false)];
        ctrl_click_is_right_click(&mut evs, &mut held);
        assert_eq!(evs.iter().map(button).collect::<Vec<_>>(), [egui::PointerButton::Secondary; 2]);
        assert!(!held);
        // a plain click, and a Command-click, stay clicks
        let mut evs = vec![ev(true, false, false), ev(false, false, false), ev(true, true, true), ev(false, true, true)];
        ctrl_click_is_right_click(&mut evs, &mut held);
        assert!(evs.iter().all(|e| button(e) == egui::PointerButton::Primary));
        // the press in one frame, the release in the next
        let mut a = vec![ev(true, true, false)];
        ctrl_click_is_right_click(&mut a, &mut held);
        let mut b = vec![ev(false, false, false)];
        ctrl_click_is_right_click(&mut b, &mut held);
        assert_eq!(button(&b[0]), egui::PointerButton::Secondary);
    }

    #[test]
    fn how_long_ago() {
        let now = 1_000_000_000u64;
        assert_eq!(ago_text(now - 20, now), "just now");
        assert_eq!(ago_text(now - 60, now), "1 min ago");
        assert_eq!(ago_text(now - 150, now), "2 mins ago");
        assert_eq!(ago_text(now - 3600, now), "1 hr ago");
        assert_eq!(ago_text(now - 6 * 3600 + 5, now), "5 hrs ago");
        assert_eq!(ago_text(now - 2 * 86400, now), "2 days ago");
    }

    #[test]
    fn songs_liked_elsewhere_come_in_without_a_reload() {
        let songs = |ids: &[i64]| ids.iter().map(|&i| track(i)).collect::<Vec<_>>();
        let ids = |v: &[Track]| v.iter().map(|t| t.id).collect::<Vec<_>>();
        // the library, newest first (the look sees the newest two)
        let old = songs(&[5, 4, 3, 2, 1]);
        // nothing new
        let (v, n) = liked_top(&old, 5, &songs(&[5, 4]), 5).unwrap();
        assert_eq!((ids(&v), n), (vec![5, 4, 3, 2, 1], 0));
        // two liked on the phone: on top
        let (v, n) = liked_top(&old, 5, &songs(&[9, 8]), 7).unwrap();
        assert_eq!((ids(&v), n), (vec![9, 8, 5, 4, 3, 2, 1], 2));
        // the first song ever liked, taken out and liked again: on top, and only there (not twice)
        let (v, n) = liked_top(&old, 5, &songs(&[1, 5]), 5).unwrap();
        assert_eq!((ids(&v), n), (vec![1, 5, 4, 3, 2], 0));
        // liked again and a new one together: both on top in Tidal's order
        let (v, n) = liked_top(&old, 5, &songs(&[9, 1]), 6).unwrap();
        assert_eq!((ids(&v), n), (vec![9, 1, 5, 4, 3, 2], 1));
        // one taken out (the count doesn't add up): read again whole
        assert!(liked_top(&old, 5, &songs(&[9, 5]), 5).is_none());
        // more new than the look sees: read again whole
        assert!(liked_top(&old, 5, &songs(&[9, 8]), 8).is_none());
    }

    #[test]
    fn column_widths_fill_the_space() {
        let saved = COLS.load(Ordering::Relaxed);
        for bits in [0b0000, 0b0001, 0b0010, 0b0011] {
            COLS.store(bits, Ordering::Relaxed);
            let w = col_widths(600.0);
            assert!((w.iter().sum::<f32>() - 600.0).abs() < 0.01, "{:b}", bits);
            assert_eq!(w[1] > 0.0, bits & 1 == 1);
            assert_eq!(w[2] > 0.0, bits & 2 == 2);
        }
        COLS.store(0b1000, Ordering::Relaxed);
        let wide = row_fit(900.0, true, true);
        assert_eq!(wide.right, 80.0 + HEART_W + KB_W, "room for the heart and KEY / BPM when shown");
        assert!(wide.kb && wide.art && wide.avail == 900.0 - 10.0 - wide.right - (8.0 + 44.0 + ART_W));
        // a narrow list: KEY / BPM give way first, then the covers, so the names stay readable
        let mid = row_fit(335.0, true, true);
        assert!(!mid.kb && mid.art && mid.avail >= 110.0, "{:?}", mid);
        let thin = row_fit(260.0, true, true);
        assert!(!thin.kb && !thin.art && thin.num && thin.heart && thin.avail >= 90.0, "{:?}", thin);
        // a very narrow one: the numbers and the hearts go too
        let tiny = row_fit(150.0, true, true);
        assert!(!tiny.num && !tiny.heart && tiny.avail >= 50.0, "{:?}", tiny);
        assert!(row_fit(100.0, true, false).avail >= 40.0, "never less than a sliver");
        COLS.store(saved, Ordering::Relaxed);
    }

    #[test]
    fn links_to_share() {
        assert_eq!(track_link(&track(42), None).as_deref(), Some("https://tidal.com/browse/track/42"));
        let yt = store::Ext { kind: "yt".into(), src: "dQw4w9WgXcQ".into(), ..Default::default() };
        assert_eq!(track_link(&track(-1), Some(&yt)).as_deref(), Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
        let sc = store::Ext { kind: "yt".into(), src: "https://soundcloud.com/a/b".into(), ..Default::default() };
        assert_eq!(track_link(&track(-2), Some(&sc)).as_deref(), Some("https://soundcloud.com/a/b"));
        let file = store::Ext { kind: "file".into(), src: "C:/a.mp3".into(), ..Default::default() };
        assert_eq!(track_link(&track(-3), Some(&file)), None);
        assert_eq!(track_link(&track(-4), None), None);
    }

    #[test]
    fn key_and_tempo_cell() {
        meta::from_tidal(888001, Some(120), Some(12 + 9));
        assert!(track_cells(888001, "T", "A", "B", "tidal").ends_with("\tAm|120\tTidal"));
        meta::set_detected(888002, meta::Info { bpm: Some(95), key: Some(0), ..Default::default() });
        assert!(track_cells(888002, "T", "A", "B", "sc").ends_with("\tC?|95\tSoundCloud"), "detected keys carry a ?");
        assert!(track_cells(888003, "T", "A", "", "file").ends_with("\t|\tFile"), "unknown: empty cell");
    }
}

/// The latest samples (`rate` a second) as the visualizers see them: `bands` from 50 Hz to 12 kHz (`nb` of them,
/// 0..1, up at once and down slowly), their falling `peaks`, and `wave`, the loudest swing in each slice; `gain` is the
/// sensitivity. No samples (nothing playing): everything settles down.
pub(crate) fn analyse(
    bands: &mut Vec<f32>,
    peaks: &mut Vec<f32>,
    wave: &mut Vec<f32>,
    samples: &[f32],
    rate: f32,
    nb: usize,
    gain: f32,
) {
    let n = samples.len();
    if bands.len() != nb {
        *bands = vec![0.0; nb];
        *peaks = vec![0.0; nb];
    }
    let mut target = vec![0.0f32; nb];
    if n >= 512 && rate > 0.0 {
        let denom = (n - 1) as f32;
        let xs: Vec<f32> = samples
            .iter()
            .enumerate()
            .map(|(i, v)| v * (0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / denom).cos()))
            .collect();
        for i in 0..nb {
            let f = 50.0 * (12000.0f32 / 50.0).powf(i as f32 / (nb - 1) as f32);
            if f >= rate / 2.0 {
                continue;
            }
            let amp = goertzel(&xs, f, rate) * 2.0;
            let db = 20.0 * (amp + 1e-6).log10();
            target[i] = ((db + 64.0 + 23.0 * i as f32 / nb as f32) / 54.0 * gain).clamp(0.0, 1.0);
        }
    }
    // waveform: the loudest swing in each slice of the latest samples
    if wave.len() != WAVE_N {
        *wave = vec![0.0; WAVE_N];
    }
    for k in 0..WAVE_N {
        let tv = if n >= WAVE_N {
            let (a, b) = (k * n / WAVE_N, ((k + 1) * n / WAVE_N).max(k * n / WAVE_N + 1));
            let mut best = 0.0f32;
            for v in &samples[a..b.min(n)] {
                if v.abs() > best.abs() {
                    best = *v;
                }
            }
            (best * 1.6 * gain).clamp(-1.0, 1.0)
        } else {
            0.0
        };
        wave[k] = wave[k] * 0.45 + tv * 0.55;
    }
    for i in 0..nb {
        let old = bands[i];
        bands[i] = if target[i] > old { target[i] } else { (old - 0.06).max(target[i]) };
        peaks[i] = bands[i].max(peaks[i] - 0.012);
    }
}
