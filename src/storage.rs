//! Saving and loading everything the user makes: memos, projects, links,
//! the wiki tree, folders, trash and settings.
//!
//! Data lives in `data.db` (SQLite) in the data folder (by default
//! `%APPDATA%\OmniMemoJang\data`): one row per memo plus one row for the
//! rest (projects, wiki tree, settings, open memo windows). Pictures are PNG
//! files named by their content (`images\<hash>.png`), written once and never
//! deleted; a saved picture is read back only while it is on screen, and lists
//! use small cached thumbnails (`images\thumbs`).
//!
//! `App::autosave` runs after user input (at most every two seconds), once a
//! minute as a safety net, and on exit; it writes only the memos that changed.
//! An older `data.json` is moved into the database on first start.
use crate::composer::{Block, Cell, Document, Format, Picture, Span, Tab, Table};
use crate::*;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// The database file.
pub const DB: &str = "data.db";
/// The single-file format used before 0.4 (and still readable).
pub const FILE: &str = "data.json";

#[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(default)]
pub struct Saved {
    pub version: u32,
    pub notes: Vec<NoteRec>,
    pub projects: Vec<String>,
    pub links: Vec<(usize, usize)>,
    pub wiki_tabs: Vec<String>,
    pub wiki_folders: Vec<WikiFolderRec>,
    pub folder_open: Vec<usize>,
    pub catalog_folders: Vec<CatalogFolderRec>,
    pub catalog_assignments: Vec<(usize, usize)>,
    pub hidden_projects: Vec<usize>,
    pub deleted_projects: Vec<usize>,
    pub deleted_at: Vec<(usize, NaiveDateTime)>,
    /// Salted SHA-256 of memo passwords: "salt$hex".
    pub password_hashes: Vec<(usize, String)>,
    pub settings: SettingsRec,
    /// Memo windows open at the last save, reopened at start-up.
    pub session: Vec<SessionRec>,
}

/// An open memo window (sticker, title bar, color box or editor).
#[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(default)]
pub struct SessionRec {
    pub note: usize,
    /// 0 editor, 1 title bar, 2 color box, 3 sticker.
    pub mode: u8,
    pub sticker_preferred: bool,
    pub pos: Option<[f32; 2]>,
    pub full_size: [f32; 2],
    pub sticker_size: [f32; 2],
    pub title_width: f32,
    pub swatch_size: [f32; 2],
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(default)]
pub struct NoteRec {
    pub title: String,
    pub body: String,
    pub document: DocRec,
    pub reminder: Option<NaiveDateTime>,
    pub locked: bool,
    pub purged: bool,
    pub date: Option<NaiveDate>,
    pub duration: u16,
    pub end_time: Option<chrono::NaiveTime>,
    pub occurrence_completed: BTreeMap<NaiveDate, NaiveDateTime>,
    pub occurrence_comments: BTreeMap<NaiveDate, Vec<String>>,
    pub time: Option<chrono::NaiveTime>,
    pub done: bool,
    pub completed: Option<NaiveDateTime>,
    pub starred: bool,
    pub recurring: bool,
    pub repeat: crate::repeat::Repeat,
    pub project: Option<usize>,
    pub parents: Vec<usize>,
    pub created: Option<NaiveDateTime>,
    pub wiki: bool,
    pub folder: Option<usize>,
    pub wiki_tab: Option<usize>,
    pub wiki_comments: Vec<String>,
    pub journal_comments: Vec<String>,
    pub color: [u8; 4],
    pub topmost: bool,
    pub fold: u8,
    pub deleted: bool,
    pub has_image: bool,
    pub has_table: bool,
    pub has_link: bool,
    pub pos: [f32; 2],
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(default)]
pub struct DocRec {
    pub tabs: Vec<TabRec>,
    pub active: usize,
    pub trash: Vec<TabRec>,
    pub font_size: f32,
    pub font_name: String,
    pub opacity: f32,
    pub toolbar_visible: bool,
    pub zoom: f32,
    pub image_scale: f32,
    pub image_fit: bool,
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(default)]
pub struct TabRec {
    pub title: String,
    pub body: String,
    pub hidden: bool,
    /// (start, end, bold, italic, underline, strike)
    pub spans: Vec<(usize, usize, [bool; 4])>,
    pub blocks: Vec<BlockRec>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum BlockRec {
    /// Legacy shape, kept only so old saves keep deserializing: a single
    /// column width, no merged cells. Never written; a load migrates this
    /// into a `TableV2` with `positioned: false` and a uniform
    /// `col_widths`. A new variant name (rather than new fields on this
    /// one) is what lets that migration tell old data from new on sight.
    Table {
        cells: Vec<Vec<(String, [bool; 4])>>,
        width: f32,
        row_height: f32,
    },
    /// Current shape: per-column widths, cell merges, and whether the
    /// table is positioned inline in the body text (like a picture) or
    /// always rendered below it (pre-migration tables, until re-saved).
    TableV2 {
        id: u32,
        cells: Vec<Vec<CellRec>>,
        col_widths: Vec<f32>,
        row_height: f32,
        positioned: bool,
    },
    Picture {
        id: u32,
        name: String,
        /// File name in the images folder.
        file: String,
        width: usize,
        height: usize,
        scale: f32,
        fit: bool,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct CellRec {
    pub text: String,
    pub format: [bool; 4],
    pub col_span: usize,
    pub row_span: usize,
    pub covered: bool,
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(default)]
pub struct WikiFolderRec {
    pub title: String,
    pub description: String,
    pub parent: Option<usize>,
    pub tab: usize,
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(default)]
pub struct CatalogFolderRec {
    pub title: String,
    pub parent: Option<usize>,
    pub open: bool,
    pub deleted: bool,
}

/// User settings that are not in `prefs` (those are needed before the data
/// folder is known).
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct SettingsRec {
    pub dark: bool,
    pub shortcuts: Vec<String>,
    pub hide_completed: bool,
    pub hide_locked: bool,
    pub show_preview: bool,
    pub auto_toolbar: bool,
    pub skip_title: bool,
    pub unpin_minimized: bool,
    pub default_color: [u8; 4],
    pub font: String,
    pub note_font: String,
    pub font_size: f32,
    pub note_font_size: f32,
    pub letter_spacing: f32,
    pub row_spacing: usize,
    pub opacity: f32,
    pub image_scale: usize,
    pub autostart: bool,
    pub taskbar: bool,
    pub retention: usize,
    pub auto_lock: usize,
    pub sound: bool,
    pub popup: bool,
    pub mute: bool,
    pub calendar_basis: usize,
    pub always_sticker: bool,
    pub sticker_size: [f32; 2],
    pub daily_today: bool,
    pub arrange_monitor: Option<usize>,
    pub show_done: bool,
    pub cal_box_color: [u8; 4],
    pub cal_box_size: f32,
}

impl Default for SettingsRec {
    fn default() -> Self {
        let d = crate::desktop::Desktop::default();
        let mut s = Self {
            dark: false,
            shortcuts: vec![],
            hide_completed: false,
            hide_locked: false,
            show_preview: false,
            auto_toolbar: false,
            skip_title: false,
            unpin_minimized: false,
            default_color: [0; 4],
            font: String::new(),
            note_font: String::new(),
            font_size: 0.,
            note_font_size: 0.,
            letter_spacing: 0.,
            row_spacing: 0,
            opacity: 100.,
            image_scale: 0,
            autostart: false,
            taskbar: false,
            retention: 1,
            auto_lock: 1,
            sound: true,
            popup: true,
            mute: false,
            calendar_basis: 1,
            always_sticker: true,
            sticker_size: [360., 360.],
            daily_today: false,
            arrange_monitor: None,
            show_done: false,
            cal_box_color: color(PALETTE[6]),
            cal_box_size: 56.,
        };
        s.read_desktop(&d);
        s
    }
}

impl SettingsRec {
    fn read_desktop(&mut self, d: &crate::desktop::Desktop) {
        self.hide_completed = d.hide_completed;
        self.hide_locked = d.hide_locked;
        self.show_preview = d.show_preview;
        self.auto_toolbar = d.auto_toolbar;
        self.skip_title = d.skip_title;
        self.unpin_minimized = d.unpin_minimized;
        self.default_color = color(d.default_color);
        self.font = d.font.clone();
        self.note_font = d.note_font.clone();
        self.font_size = d.font_size;
        self.note_font_size = d.note_font_size;
        self.letter_spacing = d.letter_spacing;
        self.row_spacing = d.row_spacing;
        self.opacity = d.opacity;
        self.image_scale = d.image_scale;
        self.autostart = d.autostart;
        self.taskbar = d.taskbar;
        self.retention = d.retention;
        self.auto_lock = d.auto_lock;
        self.sound = d.sound;
        self.popup = d.popup;
        self.mute = d.mute;
        self.calendar_basis = d.calendar_basis;
        self.always_sticker = d.always_sticker;
        self.sticker_size = [d.sticker_size.x, d.sticker_size.y];
        self.daily_today = d.daily_today;
        self.arrange_monitor = d.arrange_monitor;
    }
    fn write_desktop(&self, d: &mut crate::desktop::Desktop) {
        d.hide_completed = self.hide_completed;
        d.hide_locked = self.hide_locked;
        d.show_preview = self.show_preview;
        d.auto_toolbar = self.auto_toolbar;
        d.skip_title = self.skip_title;
        d.unpin_minimized = self.unpin_minimized;
        d.default_color = to_color(self.default_color);
        d.font = self.font.clone();
        d.note_font = self.note_font.clone();
        d.font_size = self.font_size.clamp(8., 32.);
        d.note_font_size = self.note_font_size.clamp(8., 48.);
        d.letter_spacing = self.letter_spacing;
        d.row_spacing = self.row_spacing;
        d.opacity = self.opacity.clamp(20., 100.);
        d.image_scale = self.image_scale;
        d.autostart = self.autostart;
        d.taskbar = self.taskbar;
        d.retention = self.retention.min(4);
        d.auto_lock = self.auto_lock.min(3);
        d.sound = self.sound;
        d.popup = self.popup;
        d.mute = self.mute;
        d.calendar_basis = self.calendar_basis.min(2);
        d.always_sticker = self.always_sticker;
        d.sticker_size = Vec2::new(
            self.sticker_size[0].clamp(340., 900.),
            self.sticker_size[1].clamp(220., 900.),
        );
        d.daily_today = self.daily_today;
        d.arrange_monitor = self.arrange_monitor;
    }
}

fn color(c: Color32) -> [u8; 4] {
    c.to_array()
}
fn to_color(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_premultiplied(c[0], c[1], c[2], c[3])
}
/// Sets iterate in no fixed order; sorting keeps snapshots comparable.
fn sorted(items: impl Iterator<Item = usize>) -> Vec<usize> {
    let mut v: Vec<usize> = items.collect();
    v.sort_unstable();
    v
}
fn format_bits(f: &Format) -> [bool; 4] {
    [f.bold, f.italic, f.underline, f.strike]
}
fn bits_format(b: [bool; 4]) -> Format {
    Format {
        bold: b[0],
        italic: b[1],
        underline: b[2],
        strike: b[3],
    }
}

/// Salted SHA-256 of a memo password, as "salt$hex".
pub fn hash_password(password: &str) -> String {
    let salt = format!(
        "{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    format!("{salt}${}", digest(&salt, password))
}
/// True when `password` matches a hash from `hash_password`.
pub fn check_password(hash: &str, password: &str) -> bool {
    hash.split_once('$')
        .is_some_and(|(salt, hex)| digest(salt, password) == hex)
}
fn digest(salt: &str, password: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(salt.as_bytes());
    h.update([0]);
    h.update(password.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// The data folder: the one chosen in settings, else the user's app data.
pub fn data_dir(custom: &str) -> PathBuf {
    if !custom.trim().is_empty() {
        return PathBuf::from(custom.trim());
    }
    std::env::var_os("APPDATA")
        .map(|d| PathBuf::from(d).join("OmniMemoJang").join("data"))
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share/omnimemojang"))
        })
        .unwrap_or_else(|| PathBuf::from("omnimemojang-data"))
}

/// On the very first run, lets the user choose where memos and schedules
/// are stored instead of silently defaulting to the Windows app-data
/// folder. Cancelling the folder picker (or this platform having no
/// dialog backend) keeps the default; `data_path` is only changed when a
/// folder is actually chosen.
fn prompt_first_run_data_dir(data_path: &mut String) {
    rfd::MessageDialog::new()
        .set_title("OmniMemoJang")
        .set_description(
            "메모와 일정을 저장할 폴더를 선택해주세요.\n\
             선택하지 않고 넘어가면 기본 위치(앱 데이터 폴더)에 저장됩니다.\n\
             나중에 설정에서 언제든지 바꿀 수 있습니다.",
        )
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
    if let Some(path) = rfd::FileDialog::new()
        .set_title("데이터 저장 폴더 선택")
        .pick_folder()
    {
        *data_path = path.display().to_string();
    }
}

/// Folders searched for picture files, current data folder first (an older
/// one stays while its pictures are copied after the data folder changed).
static IMAGE_DIRS: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
/// Picture files that could not be read, so they are not retried every frame.
static MISSING: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// Make `dir`'s images folder the first place pictures are read from.
pub fn use_image_dir(dir: &Path) {
    let images = dir.join("images");
    let mut dirs = IMAGE_DIRS.lock().unwrap_or_else(|e| e.into_inner());
    dirs.retain(|d| *d != images);
    dirs.insert(0, images);
    if let Ok(mut m) = MISSING.lock() {
        *m = None;
    }
}

fn find_image(file: &str) -> Option<PathBuf> {
    if file.is_empty() || file.contains(['/', '\\']) {
        return None;
    }
    let dirs = IMAGE_DIRS.lock().unwrap_or_else(|e| e.into_inner());
    dirs.iter().map(|d| d.join(file)).find(|p| p.exists())
}

/// RGBA pixels of a saved picture, at its recorded size.
pub fn load_picture(file: &str, size: [usize; 2]) -> Option<Vec<u8>> {
    if file.is_empty() {
        return None;
    }
    let mut missing = MISSING.lock().unwrap_or_else(|e| e.into_inner());
    if missing.as_ref().is_some_and(|m| m.contains(file)) {
        return None;
    }
    let img = find_image(file)
        .and_then(|p| image::open(p).ok())
        .map(|i| i.to_rgba8());
    let Some(mut img) = img else {
        missing
            .get_or_insert_with(Default::default)
            .insert(file.into());
        return None;
    };
    let (w, h) = (size[0] as u32, size[1] as u32);
    if (img.width(), img.height()) != (w, h) && w > 0 && h > 0 {
        img = image::imageops::resize(&img, w, h, image::imageops::FilterType::Triangle);
    }
    Some(img.into_raw())
}

/// Longest side of list thumbnails, in pixels.
const THUMB: u32 = 320;

/// A small version of a picture for lists and galleries. Saved pictures keep
/// their thumbnail in `images\thumbs`, so the full picture is decoded once.
pub fn thumbnail(p: &Picture) -> Option<egui::ColorImage> {
    let cached = find_image(&p.file).and_then(|f| {
        let thumbs = f.parent()?.join("thumbs");
        Some((thumbs.join(&p.file), thumbs))
    });
    if let Some((path, _)) = &cached {
        if let Ok(img) = image::open(path) {
            let img = img.to_rgba8();
            let size = [img.width() as usize, img.height() as usize];
            return Some(egui::ColorImage::from_rgba_unmultiplied(size, &img));
        }
    }
    let px = p.rgba();
    let img = image::RgbaImage::from_raw(p.width as u32, p.height as u32, px.into_owned())?;
    let img = fit_thumb(img);
    if let Some((path, thumbs)) = cached {
        let _ = std::fs::create_dir_all(thumbs);
        let _ = img.save(path);
    }
    let size = [img.width() as usize, img.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(size, &img))
}

/// Scale down keeping the aspect ratio so the longest side is `THUMB`.
fn fit_thumb(img: image::RgbaImage) -> image::RgbaImage {
    let (w, h) = (img.width(), img.height());
    let long = w.max(h);
    if long <= THUMB {
        return img;
    }
    let (nw, nh) = (
        (w as u64 * THUMB as u64 / long as u64).max(1) as u32,
        (h as u64 * THUMB as u64 / long as u64).max(1) as u32,
    );
    image::imageops::thumbnail(&img, nw, nh)
}

/// Write a new picture as a PNG named by its content and remember the name;
/// its pixels are then dropped from memory.
fn save_picture(dir: &Path, p: &mut Picture) {
    if !p.file.is_empty() || p.width == 0 || p.pixels.len() != p.width * p.height * 4 {
        return;
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    p.pixels.hash(&mut h);
    (p.width, p.height).hash(&mut h);
    let name = format!("{:016x}.png", h.finish());
    let path = dir.join("images").join(&name);
    if !path.exists() {
        let _ = std::fs::create_dir_all(dir.join("images"));
        let Some(img) = image::RgbaImage::from_raw(
            p.width as u32,
            p.height as u32,
            std::mem::take(&mut p.pixels),
        ) else {
            return;
        };
        // Written under a temporary name so a crash never leaves half a file.
        let tmp = dir.join("images").join(format!("{name}.tmp.png"));
        if img.save(&tmp).is_err() || std::fs::rename(&tmp, &path).is_err() {
            p.pixels = img.into_raw();
            return;
        }
    }
    p.pixels = Vec::new();
    p.file = name;
}

fn tab_rec(t: &Tab) -> TabRec {
    TabRec {
        title: t.title.clone(),
        body: t.body.clone(),
        hidden: t.hidden,
        spans: t
            .spans
            .iter()
            .map(|s| (s.range.start, s.range.end, format_bits(&s.format)))
            .collect(),
        blocks: t
            .blocks
            .iter()
            .map(|b| match b {
                Block::Table(t) => BlockRec::TableV2 {
                    id: t.id,
                    cells: t
                        .cells
                        .iter()
                        .map(|row| {
                            row.iter()
                                .map(|c| CellRec {
                                    text: c.text.clone(),
                                    format: format_bits(&c.format),
                                    col_span: c.col_span,
                                    row_span: c.row_span,
                                    covered: c.covered,
                                })
                                .collect()
                        })
                        .collect(),
                    col_widths: t.col_widths.clone(),
                    row_height: t.row_height,
                    positioned: t.positioned,
                },
                Block::Picture(p) => BlockRec::Picture {
                    id: p.id,
                    name: p.name.clone(),
                    file: p.file.clone(),
                    width: p.width,
                    height: p.height,
                    scale: p.scale,
                    fit: p.fit,
                },
            })
            .collect(),
    }
}

fn tab_from(r: &TabRec) -> Tab {
    Tab {
        title: r.title.clone(),
        body: r.body.clone(),
        hidden: r.hidden,
        spans: r
            .spans
            .iter()
            .map(|&(a, b, f)| Span {
                range: a..b,
                format: bits_format(f),
            })
            .collect(),
        blocks: r
            .blocks
            .iter()
            .map(|b| match b {
                // Legacy shape: migrate into the current one, unpositioned
                // (it always rendered below the text) and with one width
                // applied to every column.
                BlockRec::Table {
                    cells,
                    width,
                    row_height,
                } => Block::Table(Table {
                    id: 0,
                    cells: cells
                        .iter()
                        .map(|row| {
                            row.iter()
                                .map(|(text, f)| Cell {
                                    text: text.clone(),
                                    format: bits_format(*f),
                                    col_span: 1,
                                    row_span: 1,
                                    covered: false,
                                })
                                .collect()
                        })
                        .collect(),
                    col_widths: vec![*width; cells.first().map_or(1, Vec::len)],
                    row_height: *row_height,
                    positioned: false,
                }),
                BlockRec::TableV2 {
                    id,
                    cells,
                    col_widths,
                    row_height,
                    positioned,
                } => Block::Table(Table {
                    id: *id,
                    cells: cells
                        .iter()
                        .map(|row| {
                            row.iter()
                                .map(|c| Cell {
                                    text: c.text.clone(),
                                    format: bits_format(c.format),
                                    col_span: c.col_span,
                                    row_span: c.row_span,
                                    covered: c.covered,
                                })
                                .collect()
                        })
                        .collect(),
                    col_widths: col_widths.clone(),
                    row_height: *row_height,
                    positioned: *positioned,
                }),
                BlockRec::Picture {
                    id,
                    name,
                    file,
                    width,
                    height,
                    scale,
                    fit,
                } => Block::Picture(Picture::saved(
                    *id,
                    name.clone(),
                    file.clone(),
                    [*width, *height],
                    *scale,
                    *fit,
                )),
            })
            .collect(),
    }
}

fn doc_rec(d: &Document) -> DocRec {
    DocRec {
        tabs: d.tabs.iter().map(tab_rec).collect(),
        active: d.active,
        trash: d.trash.iter().map(tab_rec).collect(),
        font_size: d.font_size,
        font_name: d.font_name.clone(),
        opacity: d.opacity,
        toolbar_visible: d.toolbar_visible,
        zoom: d.zoom,
        image_scale: d.image_scale,
        image_fit: d.image_fit,
    }
}

fn doc_from(r: &DocRec) -> Document {
    let mut d = Document::default();
    if !r.tabs.is_empty() {
        d.tabs = r.tabs.iter().map(tab_from).collect();
    }
    d.active = r.active.min(d.tabs.len() - 1);
    d.trash = r.trash.iter().map(tab_from).collect();
    if r.font_size > 0. {
        d.font_size = r.font_size;
    }
    if !r.font_name.is_empty() {
        d.font_name = r.font_name.clone();
    }
    d.opacity = if r.opacity > 0. {
        r.opacity.clamp(0.2, 1.)
    } else {
        1.
    };
    d.toolbar_visible = r.toolbar_visible;
    if r.zoom > 0. {
        d.zoom = r.zoom;
    }
    if r.image_scale > 0. {
        d.image_scale = r.image_scale;
    }
    d.image_fit = r.image_fit;
    d
}

fn note_rec(n: &Note) -> NoteRec {
    NoteRec {
        title: n.title.clone(),
        body: n.body.clone(),
        document: doc_rec(&n.document),
        reminder: n.reminder,
        locked: n.locked,
        purged: n.purged,
        date: n.date,
        duration: n.duration,
        end_time: n.end_time,
        occurrence_completed: n.occurrence_completed.clone(),
        occurrence_comments: n.occurrence_comments.clone(),
        time: Some(n.time),
        done: n.done,
        completed: n.completed,
        starred: n.starred,
        recurring: n.recurring,
        repeat: n.repeat,
        project: n.project,
        parents: n.parents.clone(),
        created: Some(n.created),
        wiki: n.wiki,
        folder: n.folder,
        wiki_tab: n.wiki_tab,
        wiki_comments: n.wiki_comments.clone(),
        journal_comments: n.journal_comments.clone(),
        color: color(n.color),
        topmost: n.topmost,
        fold: n.fold,
        deleted: n.deleted,
        has_image: n.has_image,
        has_table: n.has_table,
        has_link: n.has_link,
        pos: [n.pos.x, n.pos.y],
    }
}

fn note_from(r: &NoteRec, i: usize) -> Note {
    let mut n = Note::new(&r.title, r.date, i);
    n.body = r.body.clone();
    n.document = doc_from(&r.document);
    n.reminder = r.reminder;
    n.locked = r.locked;
    n.purged = r.purged;
    n.duration = r.duration.max(1);
    n.end_time = r.end_time;
    n.occurrence_completed = r.occurrence_completed.clone();
    n.occurrence_comments = r.occurrence_comments.clone();
    if let Some(t) = r.time {
        n.time = t;
    }
    n.done = r.done;
    n.completed = r.completed;
    n.starred = r.starred;
    n.recurring = r.recurring;
    n.repeat = r.repeat;
    n.project = r.project;
    n.parents = r.parents.clone();
    if let Some(c) = r.created {
        n.created = c;
    }
    n.wiki = r.wiki;
    n.folder = r.folder;
    n.wiki_tab = r.wiki_tab;
    n.wiki_comments = r.wiki_comments.clone();
    n.journal_comments = r.journal_comments.clone();
    if r.color[3] > 0 {
        n.color = to_color(r.color);
    }
    n.topmost = r.topmost;
    n.fold = r.fold;
    n.deleted = r.deleted;
    n.has_image = r.has_image;
    n.has_table = r.has_table;
    n.has_link = r.has_link;
    n.pos = Vec2::new(r.pos[0], r.pos[1]);
    n
}

impl App {
    /// Everything worth keeping, ready to write. New pictures are written to
    /// the images folder as a side effect (once per picture).
    #[cfg(test)]
    pub fn snapshot(&mut self, dir: &Path) -> Saved {
        let mut saved = self.meta(dir);
        saved.notes = self.data.notes.iter().map(note_rec).collect();
        saved
    }
    /// Everything except the memos themselves. New pictures in any memo are
    /// written to the images folder first, so memos can name their files.
    fn meta(&mut self, dir: &Path) -> Saved {
        for n in self.data.notes.untracked_mut() {
            for tab in n
                .document
                .tabs
                .iter_mut()
                .chain(n.document.trash.iter_mut())
            {
                for b in &mut tab.blocks {
                    if let Block::Picture(p) = b {
                        save_picture(dir, p);
                    }
                }
            }
        }
        let mut settings = SettingsRec::default();
        settings.read_desktop(&self.desktop);
        settings.dark = self.dark;
        settings.shortcuts = self.shortcuts.to_vec();
        settings.show_done = self.extra.show_done;
        settings.cal_box_color = color(self.cal.box_color);
        settings.cal_box_size = self.cal.box_size;
        Saved {
            version: 2,
            notes: vec![],
            projects: self.data.projects.clone(),
            links: self.data.links.clone(),
            wiki_tabs: self.extra.wiki_tabs.clone(),
            wiki_folders: self
                .extra
                .folders
                .iter()
                .map(|f| WikiFolderRec {
                    title: f.title.clone(),
                    description: f.description.clone(),
                    parent: f.parent,
                    tab: f.tab,
                })
                .collect(),
            folder_open: sorted(self.extra.folder_open.iter().copied()),
            catalog_folders: self
                .catalog
                .folders
                .iter()
                .map(|f| CatalogFolderRec {
                    title: f.title.clone(),
                    parent: f.parent,
                    open: f.open,
                    deleted: f.deleted,
                })
                .collect(),
            catalog_assignments: self
                .catalog
                .assignments
                .iter()
                .map(|(a, b)| (*a, *b))
                .collect(),
            hidden_projects: sorted(self.extra.hidden_projects.iter().copied()),
            deleted_projects: sorted(self.extra.deleted_projects.iter().copied()),
            deleted_at: self
                .desktop
                .deleted_at
                .iter()
                .map(|(a, b)| (*a, *b))
                .collect(),
            password_hashes: self
                .desktop
                .passwords
                .iter()
                .map(|(a, b)| (*a, b.clone()))
                .collect(),
            settings,
            session: self
                .studio
                .windows
                .iter()
                .filter(|(_, w)| w.open)
                .map(|(&i, w)| w.session(i))
                .collect(),
        }
    }
    /// Replace the workspace with saved data.
    pub fn restore(&mut self, s: &Saved) {
        self.data.notes = s
            .notes
            .iter()
            .enumerate()
            .map(|(i, r)| note_from(r, i))
            .collect::<Vec<_>>()
            .into();
        self.data.projects = s.projects.clone();
        let count = self.data.notes.len();
        self.data.links = s
            .links
            .iter()
            .copied()
            .filter(|(a, b)| *a < count && *b < count)
            .collect();
        self.extra.wiki_tabs = if s.wiki_tabs.is_empty() {
            vec!["위키 1".into()]
        } else {
            s.wiki_tabs.clone()
        };
        self.extra.wiki_tab = 0;
        self.extra.wiki_panes = [None; 2];
        self.extra.wiki_blank = false;
        self.extra.folders = s
            .wiki_folders
            .iter()
            .map(|f| crate::extras::Folder {
                title: f.title.clone(),
                description: f.description.clone(),
                parent: f.parent,
                tab: f.tab,
            })
            .collect();
        self.extra.folder_open = s.folder_open.iter().copied().collect();
        self.extra.folder_selected = None;
        self.catalog.folders = s
            .catalog_folders
            .iter()
            .map(|f| crate::catalog::Folder {
                title: f.title.clone(),
                parent: f.parent,
                open: f.open,
                deleted: f.deleted,
            })
            .collect();
        self.catalog.assignments = s.catalog_assignments.iter().copied().collect();
        self.catalog.selected = None;
        self.extra.hidden_projects = s.hidden_projects.iter().copied().collect();
        self.extra.deleted_projects = s.deleted_projects.iter().copied().collect();
        self.desktop.deleted_at = s.deleted_at.iter().copied().collect();
        self.desktop.passwords = s.password_hashes.iter().cloned().collect();
        // Memos with a password start locked.
        for i in self.desktop.passwords.keys().copied().collect::<Vec<_>>() {
            if let Some(n) = self.data.notes.get_mut(i) {
                n.locked = true;
            }
        }
        let st = &s.settings;
        st.write_desktop(&mut self.desktop);
        self.dark = st.dark;
        for (k, v) in st.shortcuts.iter().enumerate().take(3) {
            if !v.trim().is_empty() {
                self.shortcuts[k] = v.clone();
            }
        }
        self.extra.show_done = st.show_done;
        if st.cal_box_color[3] > 0 {
            self.cal.box_color = to_color(st.cal_box_color);
        }
        if st.cal_box_size >= 32. {
            self.cal.box_size = st.cal_box_size.min(200.);
        }
        self.studio.windows.clear();
        // Memo windows left open (stickers on the desktop) come back.
        for r in &s.session {
            if self.data.notes.get(r.note).is_some_and(|n| !n.deleted) {
                self.studio
                    .windows
                    .insert(r.note, crate::studio::Window::from_session(r));
            }
        }
        self.active = 0;
    }
    /// Save what changed: after user input at most every two seconds, once a
    /// minute regardless, or now when `force` (e.g. on exit).
    pub fn autosave(&mut self, now: f64, force: bool) {
        if INPUT.swap(false, Ordering::Relaxed) {
            self.saving.pending = true;
        }
        if !force {
            if now - self.saving.checked < 2. {
                return;
            }
            if !self.saving.pending && now - self.saving.checked < 60. {
                return;
            }
        }
        self.saving.checked = now;
        self.saving.pending = false;
        let dir = data_dir(&self.prefs.data_path);
        if self.saving.dir.as_deref() != Some(dir.as_path()) {
            self.switch_data_dir(&dir);
        }
        let meta = self.meta(&dir);
        // Only memos handed out for changing since the last save are looked
        // at (all of them on exit, after a failed write or a new folder), and
        // only a fingerprint of each memo is kept, not a second copy.
        let count = self.data.notes.len();
        let touched = self.data.notes.take_touched();
        let look: Vec<usize> = match touched {
            Some(t) if !force && !std::mem::take(&mut self.saving.full) => t,
            _ => {
                self.saving.full = false;
                (0..count).collect()
            }
        };
        let mut hashes = self.saving.notes.clone();
        hashes.resize(count, 0);
        let mut changed = vec![];
        for i in look.into_iter().filter(|&i| i < count) {
            let rec = note_rec(&self.data.notes[i]);
            let h = fingerprint(&rec);
            if self.saving.notes.get(i) != Some(&h) {
                changed.push((i, rec));
            }
            hashes[i] = h;
        }
        let meta_hash = fingerprint(&meta);
        let meta_changed = self.saving.meta != Some(meta_hash);
        let shrank = self.saving.notes.len() > hashes.len();
        if changed.is_empty() && !meta_changed && !shrank {
            return;
        }
        let changed: Vec<(usize, &NoteRec)> = changed.iter().map(|(i, r)| (*i, r)).collect();
        let result = self
            .saving
            .db
            .as_mut()
            .ok_or_else(|| self.saving.error.clone())
            .and_then(|db| write(db, &changed, hashes.len(), meta_changed.then_some(&meta)));
        match result {
            Ok(()) => {
                self.saving.notes = hashes;
                self.saving.meta = Some(meta_hash);
                self.saving.error.clear();
            }
            Err(e) => {
                self.saving.full = true;
                let e = format!("저장하지 못했습니다: {e}");
                // Tell the user once, wherever they are; settings keeps showing it.
                if self.saving.error != e {
                    self.toast = e.clone();
                }
                self.saving.error = e;
            }
        }
    }
    /// What was just read from the database is what is saved: the first
    /// autosave then writes only what differs.
    fn mark_saved(&mut self, dir: &Path) {
        self.switch_data_dir(dir);
        let meta = self.meta(dir);
        self.saving.notes = self
            .data
            .notes
            .iter()
            .map(|n| fingerprint(&note_rec(n)))
            .collect();
        self.saving.meta = Some(fingerprint(&meta));
        self.saving.full = false;
        self.data.notes.take_touched();
    }
    /// Start saving into `dir`. Pictures of the previous data folder are
    /// copied in the background and stay readable meanwhile; everything is
    /// written in full to the new folder.
    fn switch_data_dir(&mut self, dir: &Path) {
        if let Some(old) = self.saving.dir.take() {
            // Data already in the new folder is kept as a backup there
            // before this workspace is written over it.
            if dir.join(DB).exists() {
                if let Err(e) = backup(dir) {
                    self.toast = format!("새 폴더의 기존 데이터를 백업하지 못했습니다: {e}");
                }
            }
            let (from, to) = (old.join("images"), dir.join("images"));
            std::thread::spawn(move || copy_missing(&from, &to));
        }
        use_image_dir(dir);
        self.saving.notes.clear();
        self.saving.meta = None;
        self.saving.full = true;
        self.saving.db = match open(dir) {
            Ok(c) => Some(c),
            Err(e) => {
                self.saving.error = format!("저장하지 못했습니다: {e}");
                None
            }
        };
        self.saving.dir = Some(dir.to_path_buf());
    }
}

/// Set by any user input (keys, clicks, wheel, drops) in any window; the
/// next autosave then looks for changes.
pub static INPUT: AtomicBool = AtomicBool::new(true);

/// Watch user input in every window for `INPUT` and `lists::CHANGES`.
pub fn watch_input(ctx: &egui::Context) {
    ctx.on_begin_pass(
        "autosave-input",
        std::sync::Arc::new(|ctx| {
            let input = ctx.input(|i| {
                !i.raw.dropped_files.is_empty()
                    || i.events.iter().any(|e| {
                        !matches!(
                            e,
                            egui::Event::PointerMoved(_)
                                | egui::Event::MouseMoved(_)
                                | egui::Event::PointerGone
                                | egui::Event::WindowFocused(_)
                        )
                    })
            });
            if input {
                INPUT.store(true, Ordering::Relaxed);
                FRAME_INPUT.store(true, Ordering::Relaxed);
                crate::lists::CHANGES.fetch_add(1, Ordering::Relaxed);
            }
        }),
    );
    // Input changes data later in the same frame, after lists were made:
    // count once more when the whole frame (the root pass) ends.
    ctx.on_end_pass(
        "list-changes",
        std::sync::Arc::new(|ctx| {
            if ctx.viewport_id() == egui::ViewportId::ROOT
                && FRAME_INPUT.swap(false, Ordering::Relaxed)
            {
                crate::lists::CHANGES.fetch_add(1, Ordering::Relaxed);
            }
        }),
    );
}

/// Input happened somewhere during the current frame.
static FRAME_INPUT: AtomicBool = AtomicBool::new(false);

/// Copy files of `from` missing in `to` (pictures are never changed in place).
pub fn copy_missing(from: &Path, to: &Path) {
    let Ok(entries) = std::fs::read_dir(from) else {
        return;
    };
    let _ = std::fs::create_dir_all(to);
    for e in entries.flatten() {
        let target = to.join(e.file_name());
        if e.path().is_file() && !target.exists() {
            let _ = std::fs::copy(e.path(), target);
        }
    }
}

impl App {
    /// At startup: load saved data, or start a new workspace with the usage
    /// guide. The guide page opens only on the very first run; later runs
    /// open the start page chosen in settings.
    pub fn load_or_start(&mut self) {
        if !self.prefs.first_run_done && self.prefs.data_path.trim().is_empty() {
            prompt_first_run_data_dir(&mut self.prefs.data_path);
        }
        let dir = data_dir(&self.prefs.data_path);
        use_image_dir(&dir);
        match read(&dir) {
            Ok(Some(saved)) => {
                self.restore(&saved);
                self.mark_saved(&dir);
            }
            Ok(None) => self.start_fresh(),
            Err(e) => {
                // Keep the unreadable file; never overwrite it.
                let stamp = Local::now().format("%Y%m%d-%H%M%S");
                let (from, bad) = if dir.join(DB).exists() {
                    (DB, dir.join(format!("data.broken-{stamp}.db")))
                } else {
                    (FILE, dir.join(format!("data.broken-{stamp}.json")))
                };
                let _ = std::fs::rename(dir.join(from), &bad);
                for extra in ["-wal", "-shm"] {
                    let _ = std::fs::remove_file(dir.join(format!("{DB}{extra}")));
                }
                self.start_fresh();
                self.toast = format!(
                    "저장 파일을 읽지 못해 {}(으)로 옮겨 두었습니다 ({e})",
                    bad.display()
                );
            }
        }
        if self.prefs.first_run_done {
            self.page = self.prefs.start_page;
        } else {
            self.prefs.first_run_done = true;
            self.prefs.save();
            self.open_guide();
        }
    }
    /// An empty workspace holding only the usage guide.
    pub fn start_fresh(&mut self) {
        self.data = Workspace {
            notes: vec![].into(),
            projects: vec!["개인 업무".into()],
            links: vec![],
        };
        self.extra.folders.clear();
        self.extra.folder_open.clear();
        self.extra.wiki_tabs = vec!["위키 1".into()];
        self.extra.wiki_tab = 0;
        self.catalog = Default::default();
        self.desktop.deleted_at.clear();
        self.desktop.passwords.clear();
        self.studio.windows.clear();
        crate::guide::install(self);
    }
}

fn open(dir: &Path) -> Result<rusqlite::Connection, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let db = rusqlite::Connection::open(dir.join(DB)).map_err(|e| e.to_string())?;
    db.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA journal_size_limit=4194304;
         PRAGMA wal_checkpoint(TRUNCATE);
         CREATE TABLE IF NOT EXISTS notes(id INTEGER PRIMARY KEY, rec TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    )
    .map_err(|e| e.to_string())?;
    Ok(db)
}

/// Write changed memos (and the rest, when given) in one transaction; memo
/// rows from `count` on are removed.
fn write(
    db: &mut rusqlite::Connection,
    notes: &[(usize, &NoteRec)],
    count: usize,
    meta: Option<&Saved>,
) -> Result<(), String> {
    let err = |e: rusqlite::Error| e.to_string();
    let tx = db.transaction().map_err(err)?;
    {
        let mut put = tx
            .prepare_cached("INSERT OR REPLACE INTO notes(id, rec) VALUES (?1, ?2)")
            .map_err(err)?;
        for (i, r) in notes {
            let json = serde_json::to_string(r).map_err(|e| e.to_string())?;
            put.execute(rusqlite::params![*i as i64, json])
                .map_err(err)?;
        }
    }
    tx.execute("DELETE FROM notes WHERE id >= ?1", [count as i64])
        .map_err(err)?;
    if let Some(m) = meta {
        let json = serde_json::to_string(m).map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT OR REPLACE INTO meta(key, value) VALUES ('workspace', ?1)",
            [json],
        )
        .map_err(err)?;
    }
    tx.commit().map_err(err)
}

/// Write a whole workspace into the database in `dir`.
pub fn write_all(dir: &Path, saved: &Saved) -> Result<(), String> {
    let mut db = open(dir)?;
    let notes: Vec<(usize, &NoteRec)> = saved.notes.iter().enumerate().collect();
    let meta = Saved {
        notes: vec![],
        ..saved.clone()
    };
    write(&mut db, &notes, notes.len(), Some(&meta))
}

/// Read the workspace from `dir`. `Ok(None)` when there is none yet. An
/// older `data.json` is moved into the database (and kept, renamed). If an
/// older version wrote `data.json` after the move (it is newer than the
/// database), that newer data wins; the database is backed up first.
pub fn read(dir: &Path) -> Result<Option<Saved>, String> {
    let json = dir.join(FILE);
    let modified = |name: &str| {
        std::fs::metadata(dir.join(name))
            .and_then(|m| m.modified())
            .ok()
    };
    if dir.join(DB).exists() {
        let db_time = modified(DB).max(modified(&format!("{DB}-wal")));
        if !json.exists() || modified(FILE) <= db_time {
            return read_db(&dir.join(DB)).map(Some);
        }
        // An unreadable newer file must not cost the good database.
        let Ok(saved) = read_file(&json) else {
            let _ = std::fs::rename(&json, dir.join("data.unreadable.json"));
            return read_db(&dir.join(DB)).map(Some);
        };
        backup(dir)?;
        write_all(dir, &saved)?;
        let _ = std::fs::rename(&json, dir.join("data.before-db.json"));
        return Ok(Some(saved));
    }
    if !json.exists() {
        return Ok(None);
    }
    let saved = read_file(&json)?;
    write_all(dir, &saved)?;
    let _ = std::fs::rename(&json, dir.join("data.before-db.json"));
    Ok(Some(saved))
}

fn read_db(path: &Path) -> Result<Saved, String> {
    let err = |e: rusqlite::Error| e.to_string();
    let db =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(err)?;
    let meta: String = db
        .query_row("SELECT value FROM meta WHERE key = 'workspace'", [], |r| {
            r.get(0)
        })
        .map_err(err)?;
    let mut saved: Saved = serde_json::from_str(&meta).map_err(|e| e.to_string())?;
    let mut rows = db
        .prepare("SELECT id, rec FROM notes ORDER BY id")
        .map_err(err)?;
    let rows = rows
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .map_err(err)?;
    for row in rows {
        let (id, json) = row.map_err(err)?;
        let rec: NoteRec = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        // Memo numbers are positions; a gap would shift every later memo.
        while (saved.notes.len() as i64) < id {
            saved.notes.push(NoteRec {
                purged: true,
                deleted: true,
                ..Default::default()
            });
        }
        saved.notes.push(rec);
    }
    Ok(saved)
}

/// Read a backup the user picked: a `data.db`, or a `data.json` from older
/// versions.
pub fn read_file(path: &Path) -> Result<Saved, String> {
    if path.extension().is_some_and(|e| e == "db") {
        return read_db(path);
    }
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// Copy the database into `backups\<time>\data.db`. Pictures are not copied:
/// they are never changed or deleted, so the images folder serves every
/// backup.
pub fn backup(dir: &Path) -> Result<PathBuf, String> {
    let stamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let mut target = dir.join("backups").join(&stamp);
    let mut n = 1;
    while target.join(DB).exists() {
        n += 1;
        target = dir.join("backups").join(format!("{stamp}-{n}"));
    }
    std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    let db = open(dir)?;
    db.execute("VACUUM INTO ?1", [target.join(DB).to_string_lossy()])
        .map_err(|e| e.to_string())?;
    Ok(target)
}

/// A fingerprint of a record as it would be written.
fn fingerprint(rec: &impl Serialize) -> u64 {
    struct Sink(std::collections::hash_map::DefaultHasher);
    impl std::io::Write for Sink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.write(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut sink = Sink(Default::default());
    let _ = serde_json::to_writer(&mut sink, rec);
    sink.0.finish()
}

/// Autosave bookkeeping kept in `App`.
#[derive(Default)]
pub struct Saving {
    /// Fingerprints of the memos and of the rest as last written.
    notes: Vec<u64>,
    meta: Option<u64>,
    /// Look at every memo at the next save, not only the touched ones.
    full: bool,
    pub checked: f64,
    /// Input happened since the last check.
    pending: bool,
    pub error: String,
    dir: Option<PathBuf>,
    db: Option<rusqlite::Connection>,
}

impl Saving {
    /// Write everything again at the next autosave.
    pub fn rewrite(&mut self) {
        self.notes.clear();
        self.meta = None;
        self.full = true;
        self.checked = f64::MIN;
        self.pending = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn memos_wiki_settings_and_pictures_survive_saving() {
        let dir = std::env::temp_dir().join(format!(
            "omnimemojang-store-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let mut a = App::sample(NaiveDate::from_ymd_opt(2026, 11, 11).unwrap());
        a.data.notes[1].duration = 3;
        a.data.notes[1].title = "기간 일정".into();
        a.data.notes[2].reminder = Some(
            NaiveDate::from_ymd_opt(2026, 11, 12)
                .unwrap()
                .and_hms_opt(9, 30, 0)
                .unwrap(),
        );
        let pic = Picture::from_rgba(0, "점".into(), vec![255, 0, 0, 255], [1, 1], 1., true);
        a.data.notes[0].document.tabs[0]
            .blocks
            .push(Block::Picture(pic));
        a.data.notes[0].body.push(crate::composer::placeholder(0));
        a.extra.wiki_tabs.push("사용법".into());
        a.desktop.passwords.insert(3, hash_password("1234"));
        a.data.notes[3].locked = true;
        a.dark = true;
        let snap = a.snapshot(&dir);
        write_all(&dir, &snap).unwrap();
        let loaded = read(&dir).unwrap().unwrap();
        assert!(loaded == snap);
        use_image_dir(&dir);
        let mut b = App::sample(NaiveDate::from_ymd_opt(2026, 11, 11).unwrap());
        b.restore(&loaded);
        assert_eq!(b.data.notes.len(), a.data.notes.len());
        assert_eq!(b.data.notes[1].title, "기간 일정");
        assert_eq!(b.data.notes[1].duration, 3);
        assert_eq!(b.data.notes[2].reminder, a.data.notes[2].reminder);
        assert_eq!(b.extra.wiki_tabs, a.extra.wiki_tabs);
        assert!(b.dark);
        assert!(b.data.notes[3].locked);
        assert!(check_password(&b.desktop.passwords[&3], "1234"));
        assert!(!check_password(&b.desktop.passwords[&3], "0000"));
        match &b.data.notes[0].document.tabs[0].blocks.last() {
            Some(Block::Picture(p)) => {
                // Not in memory until shown, then read from its file.
                assert!(p.pixels.is_empty());
                assert_eq!(&*p.rgba(), &[255, 0, 0, 255]);
                assert!(thumbnail(p).is_some());
            }
            _ => panic!("picture not restored"),
        }
        // Saving the restored workspace changes nothing.
        assert!(b.snapshot(&dir) == snap);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn table_round_trips_merges_and_legacy_data_migrates() {
        let mut table = Table::new(2, 2);
        table.id = 7;
        table.positioned = true;
        table.col_widths = vec![100., 150.];
        table.cells[0][0].text = "병합된 셀".into();
        table.cells[0][0].col_span = 2;
        table.cells[0][1].covered = true;
        let mut tab = Tab {
            title: "표".into(),
            body: String::new(),
            hidden: false,
            spans: vec![],
            blocks: vec![],
        };
        tab.blocks.push(Block::Table(table));
        let rec = tab_rec(&tab);
        match &rec.blocks[0] {
            BlockRec::TableV2 { positioned, .. } => assert!(positioned),
            _ => panic!("always saved as TableV2, never the legacy shape"),
        }
        let back = tab_from(&rec);
        let Block::Table(t) = &back.blocks[0] else {
            panic!("table restored")
        };
        assert_eq!(t.id, 7);
        assert!(t.positioned);
        assert_eq!(t.col_widths, vec![100., 150.]);
        assert_eq!(t.cells[0][0].col_span, 2);
        assert!(t.cells[0][1].covered);

        // A save made before merges/positioning existed: one uniform width,
        // no span fields at all.
        let legacy = TabRec {
            title: "legacy".into(),
            blocks: vec![BlockRec::Table {
                cells: vec![vec![("가".into(), [false; 4]), ("나".into(), [false; 4])]],
                width: 90.,
                row_height: 28.,
            }],
            ..Default::default()
        };
        let migrated = tab_from(&legacy);
        let Block::Table(t) = &migrated.blocks[0] else {
            panic!("table restored")
        };
        assert!(
            !t.positioned,
            "a legacy table always rendered below the text, so it stays that way until re-saved"
        );
        assert_eq!(t.col_widths, vec![90., 90.]);
        assert_eq!(t.cells[0][0].text, "가");
        assert_eq!(t.cells[0][0].col_span, 1);
        assert!(!t.cells[0][0].covered);
    }

    fn temp(tag: u32) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("omnimemojang-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn autosave_writes_changed_memos_and_reopens_stickers() {
        let dir = temp(line!());
        let day = NaiveDate::from_ymd_opt(2026, 11, 12).unwrap();
        let mut a = App::sample(day);
        a.prefs.data_path = dir.display().to_string();
        a.studio.open(2, None);
        a.studio.windows.get_mut(&2).unwrap().sticker();
        a.autosave(0., true);
        assert!(dir.join(DB).exists());
        // Nothing changed and no input: no write is attempted.
        a.data.notes[1].title = "바뀐 제목".into();
        INPUT.store(false, Ordering::Relaxed);
        a.autosave(10., false);
        assert_ne!(read(&dir).unwrap().unwrap().notes[1].title, "바뀐 제목");
        // After input the change is written.
        INPUT.store(true, Ordering::Relaxed);
        a.autosave(20., false);
        let loaded = read(&dir).unwrap().unwrap();
        assert_eq!(loaded.notes[1].title, "바뀐 제목");
        assert_eq!(loaded.notes.len(), a.data.notes.len());
        let mut b = App::sample(day);
        b.restore(&loaded);
        let w = &b.studio.windows[&2];
        assert!(w.open);
        assert_eq!(w.mode, crate::studio::Mode::Sticker);
        assert_eq!(b.studio.windows.len(), 1);
        drop(a);
        // Starting from the database: nothing is pending, and afterwards
        // only memos handed out for changing are looked at.
        let mut c = App::sample(day);
        c.prefs.data_path = dir.display().to_string();
        c.prefs.first_run_done = true;
        c.load_or_start();
        assert_eq!(c.saving.notes.len(), c.data.notes.len());
        assert_eq!(c.data.notes.take_touched(), Some(vec![]));
        let before = c.saving.notes.clone();
        c.data.notes[3].title = "세 번째만 바뀜".into();
        INPUT.store(true, Ordering::Relaxed);
        c.autosave(100., false);
        let changed: Vec<usize> = (0..before.len())
            .filter(|&i| before[i] != c.saving.notes[i])
            .collect();
        assert_eq!(changed, vec![3]);
        assert_eq!(
            read(&dir).unwrap().unwrap().notes[3].title,
            "세 번째만 바뀜"
        );
        drop(c);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn old_data_json_moves_into_the_database() {
        let dir = temp(line!());
        let mut a = App::sample(NaiveDate::from_ymd_opt(2026, 11, 12).unwrap());
        let snap = a.snapshot(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(FILE), serde_json::to_string(&snap).unwrap()).unwrap();
        let loaded = read(&dir).unwrap().unwrap();
        assert!(loaded == snap);
        assert!(dir.join(DB).exists());
        assert!(!dir.join(FILE).exists());
        assert!(dir.join("data.before-db.json").exists());
        assert!(read(&dir).unwrap().unwrap() == snap);
        let b = backup(&dir).unwrap();
        assert!(read_file(&b.join(DB)).unwrap() == snap);
        // An older version writing data.json later: the newer file wins.
        std::thread::sleep(std::time::Duration::from_millis(50));
        let mut newer = snap.clone();
        newer.notes[0].title = "이전 버전에서 쓴 메모".into();
        std::fs::write(dir.join(FILE), serde_json::to_string(&newer).unwrap()).unwrap();
        assert!(read(&dir).unwrap().unwrap() == newer);
        assert!(read(&dir).unwrap().unwrap() == newer);
        assert!(!dir.join(FILE).exists());
        // A newer file that cannot be read never replaces the database.
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(dir.join(FILE), "{ broken").unwrap();
        assert!(read(&dir).unwrap().unwrap() == newer);
        assert!(dir.join("data.unreadable.json").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
