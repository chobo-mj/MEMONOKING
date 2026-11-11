//! Saving and loading everything the user makes: memos, projects, links,
//! the wiki tree, folders, trash and settings.
//!
//! Data lives in one `data.json` in the data folder (by default
//! `%APPDATA%\OmniMemoJang\data`); pictures in memos are stored next to it as
//! PNG files named by their content (`images\<hash>.png`), so unchanged
//! pictures are written once. The UI never touches the disk: `App::autosave`
//! compares a fresh snapshot with the last one written, at most every two
//! seconds, and on exit.
use crate::composer::{Block, Cell, Document, Format, Picture, Span, Tab, Table};
use crate::*;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

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
    Table {
        cells: Vec<Vec<(String, [bool; 4])>>,
        width: f32,
        row_height: f32,
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
    pub image_limit: f32,
    pub image_age: usize,
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
            taskbar: true,
            retention: 1,
            auto_lock: 1,
            sound: true,
            popup: true,
            mute: false,
            image_limit: 0.,
            image_age: 0,
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
        self.image_limit = d.image_limit;
        self.image_age = d.image_age;
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
        d.image_limit = self.image_limit;
        d.image_age = self.image_age;
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

/// Writes pictures as PNG files named by content, once each.
#[derive(Default)]
pub struct Pictures {
    /// Content hash of the pixels → file name already on disk.
    known: HashMap<u64, String>,
}

impl Pictures {
    fn file_for(&mut self, dir: &Path, p: &Picture) -> String {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        p.pixels.hash(&mut h);
        (p.width, p.height).hash(&mut h);
        let key = h.finish();
        if let Some(name) = self.known.get(&key) {
            return name.clone();
        }
        let name = format!("{key:016x}.png");
        let path = dir.join("images").join(&name);
        if !path.exists() {
            let _ = std::fs::create_dir_all(dir.join("images"));
            if let Some(img) =
                image::RgbaImage::from_raw(p.width as u32, p.height as u32, p.pixels.clone())
            {
                let _ = img.save(&path);
            }
        }
        self.known.insert(key, name.clone());
        name
    }
}

fn tab_rec(dir: &Path, pics: &mut Pictures, t: &Tab) -> TabRec {
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
                Block::Table(t) => BlockRec::Table {
                    cells: t
                        .cells
                        .iter()
                        .map(|row| {
                            row.iter()
                                .map(|c| (c.text.clone(), format_bits(&c.format)))
                                .collect()
                        })
                        .collect(),
                    width: t.width,
                    row_height: t.row_height,
                },
                Block::Picture(p) => BlockRec::Picture {
                    id: p.id,
                    name: p.name.clone(),
                    file: pics.file_for(dir, p),
                    width: p.width,
                    height: p.height,
                    scale: p.scale,
                    fit: p.fit,
                },
            })
            .collect(),
    }
}

fn tab_from(dir: &Path, r: &TabRec) -> Tab {
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
            .filter_map(|b| match b {
                BlockRec::Table {
                    cells,
                    width,
                    row_height,
                } => Some(Block::Table(Table {
                    cells: cells
                        .iter()
                        .map(|row| {
                            row.iter()
                                .map(|(text, f)| Cell {
                                    text: text.clone(),
                                    format: bits_format(*f),
                                })
                                .collect()
                        })
                        .collect(),
                    width: *width,
                    row_height: *row_height,
                })),
                BlockRec::Picture {
                    id,
                    name,
                    file,
                    width,
                    height,
                    scale,
                    fit,
                } => {
                    let img = image::open(dir.join("images").join(file)).ok()?.to_rgba8();
                    // The file wins over the recorded size if they disagree.
                    let size = [img.width() as usize, img.height() as usize];
                    let _ = (width, height);
                    Some(Block::Picture(Picture::from_rgba(
                        *id,
                        name.clone(),
                        img.into_raw(),
                        size,
                        *scale,
                        *fit,
                    )))
                }
            })
            .collect(),
    }
}

fn doc_rec(dir: &Path, pics: &mut Pictures, d: &Document) -> DocRec {
    DocRec {
        tabs: d.tabs.iter().map(|t| tab_rec(dir, pics, t)).collect(),
        active: d.active,
        trash: d.trash.iter().map(|t| tab_rec(dir, pics, t)).collect(),
        font_size: d.font_size,
        font_name: d.font_name.clone(),
        opacity: d.opacity,
        toolbar_visible: d.toolbar_visible,
        zoom: d.zoom,
        image_scale: d.image_scale,
        image_fit: d.image_fit,
    }
}

fn doc_from(dir: &Path, r: &DocRec) -> Document {
    let mut d = Document::default();
    if !r.tabs.is_empty() {
        d.tabs = r.tabs.iter().map(|t| tab_from(dir, t)).collect();
    }
    d.active = r.active.min(d.tabs.len() - 1);
    d.trash = r.trash.iter().map(|t| tab_from(dir, t)).collect();
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

fn note_rec(dir: &Path, pics: &mut Pictures, n: &Note) -> NoteRec {
    NoteRec {
        title: n.title.clone(),
        body: n.body.clone(),
        document: doc_rec(dir, pics, &n.document),
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

fn note_from(dir: &Path, r: &NoteRec, i: usize) -> Note {
    let mut n = Note::new(&r.title, r.date, i);
    n.body = r.body.clone();
    n.document = doc_from(dir, &r.document);
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
    /// Everything worth keeping, ready to write. Pictures are written to the
    /// images folder as a side effect (once per picture).
    pub fn snapshot(&mut self, dir: &Path) -> Saved {
        let pics = &mut self.saving.pictures;
        let mut settings = SettingsRec::default();
        settings.read_desktop(&self.desktop);
        settings.dark = self.dark;
        settings.shortcuts = self.shortcuts.to_vec();
        settings.show_done = self.extra.show_done;
        settings.cal_box_color = color(self.cal.box_color);
        settings.cal_box_size = self.cal.box_size;
        Saved {
            version: 1,
            notes: self
                .data
                .notes
                .iter()
                .map(|n| note_rec(dir, pics, n))
                .collect(),
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
        }
    }
    /// Replace the workspace with saved data.
    pub fn restore(&mut self, s: &Saved, dir: &Path) {
        self.data.notes = s
            .notes
            .iter()
            .enumerate()
            .map(|(i, r)| note_from(dir, r, i))
            .collect();
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
        self.active = 0;
    }
    /// Save when something changed, at most every two seconds (or now when
    /// `force`, e.g. on exit).
    pub fn autosave(&mut self, now: f64, force: bool) {
        if !force && now - self.saving.checked < 2. {
            return;
        }
        self.saving.checked = now;
        let dir = data_dir(&self.prefs.data_path);
        let snap = self.snapshot(&dir);
        if self.saving.last.as_ref() == Some(&snap) {
            return;
        }
        match write(&dir, &snap) {
            Ok(()) => {
                self.saving.last = Some(snap);
                self.saving.error.clear();
            }
            Err(e) => self.saving.error = format!("저장하지 못했습니다: {e}"),
        }
    }
}

impl App {
    /// At startup: load saved data, or start a new workspace with the usage
    /// guide. The guide page opens only on the very first run; later runs
    /// open the start page chosen in settings.
    pub fn load_or_start(&mut self) {
        let dir = data_dir(&self.prefs.data_path);
        match read(&dir) {
            Ok(Some(saved)) => self.restore(&saved, &dir),
            Ok(None) => self.start_fresh(),
            Err(e) => {
                // Keep the unreadable file; never overwrite it.
                let stamp = Local::now().format("%Y%m%d-%H%M%S");
                let bad = dir.join(format!("data.broken-{stamp}.json"));
                let _ = std::fs::rename(dir.join(FILE), &bad);
                self.start_fresh();
                self.toast = format!(
                    "저장 파일을 읽지 못해 {}(으)로 옮겨 두었습니다 ({e})",
                    bad.display()
                );
            }
        }
        self.purge_expired_trash(Local::now().naive_local());
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
            notes: vec![],
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

/// Write `data.json` atomically (temp file, then rename).
pub fn write(dir: &Path, snap: &Saved) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string(snap).map_err(|e| e.to_string())?;
    let tmp = dir.join(format!("{FILE}.tmp"));
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, dir.join(FILE)).map_err(|e| e.to_string())
}

/// Read `data.json` from `dir`. `Ok(None)` when there is none yet.
pub fn read(dir: &Path) -> Result<Option<Saved>, String> {
    let path = dir.join(FILE);
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|e| e.to_string())
}

/// Read a backed-up `data.json` the user picked.
pub fn read_file(path: &Path) -> Result<Saved, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// Copy the data file and its pictures into `backups\<time>\`.
pub fn backup(dir: &Path) -> Result<PathBuf, String> {
    let stamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let target = dir.join("backups").join(stamp);
    std::fs::create_dir_all(target.join("images")).map_err(|e| e.to_string())?;
    std::fs::copy(dir.join(FILE), target.join(FILE)).map_err(|e| e.to_string())?;
    if let Ok(entries) = std::fs::read_dir(dir.join("images")) {
        for e in entries.flatten() {
            let _ = std::fs::copy(e.path(), target.join("images").join(e.file_name()));
        }
    }
    Ok(target)
}

/// Autosave bookkeeping kept in `App`.
#[derive(Default)]
pub struct Saving {
    pub last: Option<Saved>,
    pub checked: f64,
    pub error: String,
    pub pictures: Pictures,
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
        write(&dir, &snap).unwrap();
        let loaded = read(&dir).unwrap().unwrap();
        assert!(loaded == snap);
        let mut b = App::sample(NaiveDate::from_ymd_opt(2026, 11, 11).unwrap());
        b.restore(&loaded, &dir);
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
            Some(Block::Picture(p)) => assert_eq!(p.pixels, vec![255, 0, 0, 255]),
            _ => panic!("picture not restored"),
        }
        // Saving the restored workspace changes nothing.
        assert!(b.snapshot(&dir) == snap);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
