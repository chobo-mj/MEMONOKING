//! In-memory memo content editor. Tabs share the parent memo's metadata.
use eframe::egui::{
    self,
    text::{CCursor, CCursorRange, LayoutJob},
    Color32, FontFamily, FontId, Id, Key, Modifiers, RichText, Stroke, TextFormat, Vec2,
};
use std::{hash::Hash, ops::Range, path::Path};

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Format {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
}
#[derive(Clone, Debug)]
pub struct Span {
    pub range: Range<usize>,
    pub format: Format,
}
#[derive(Clone)]
pub struct Cell {
    pub text: String,
    pub format: Format,
}
#[derive(Clone)]
pub struct Table {
    pub cells: Vec<Vec<Cell>>,
    pub width: f32,
    pub row_height: f32,
}
impl Table {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            cells: vec![
                vec![
                    Cell {
                        text: String::new(),
                        format: Format::default()
                    };
                    cols.clamp(1, 20)
                ];
                rows.clamp(1, 100)
            ],
            width: 120.,
            row_height: 32.,
        }
    }
    pub fn tsv(&self) -> String {
        self.cells
            .iter()
            .map(|row| {
                row.iter()
                    .map(|c| c.text.replace(['\t', '\n'], " "))
                    .collect::<Vec<_>>()
                    .join("\t")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    pub fn from_tsv(s: &str) -> Self {
        let rows: Vec<_> = s
            .trim_end_matches(['\r', '\n'])
            .split('\n')
            .take(100)
            .map(|r| {
                r.trim_end_matches('\r')
                    .split('\t')
                    .take(20)
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .collect();
        let cols = rows.iter().map(Vec::len).max().unwrap_or(1);
        let mut t = Self::new(rows.len(), cols);
        for (r, row) in rows.into_iter().enumerate() {
            for (c, text) in row.into_iter().enumerate() {
                t.cells[r][c].text = text;
            }
        }
        t
    }
    /// Build a table from clipboard HTML (the real `<table>...</table>`
    /// markup that Excel/한글/Word place alongside their plain-text TSV
    /// rendering). Reading each `<td>`/`<th>` as its own cell means a cell's
    /// own tabs or line breaks never get mistaken for a column or row
    /// boundary the way they do with `from_tsv`. A horizontally merged cell
    /// (`colspan`) is padded with blank cells so every row still lines up
    /// column-for-column; a vertical merge (`rowspan`) is not reconstructed
    /// across rows, so only the merged cell's own text survives, not the
    /// merge itself — this type has no notion of a cell spanning rows.
    pub fn from_html(html: &str) -> Option<Self> {
        let doc = scraper::Html::parse_fragment(html);
        let table_sel = scraper::Selector::parse("table").ok()?;
        let table_el = doc.select(&table_sel).next()?;
        let row_sel = scraper::Selector::parse("tr").ok()?;
        let cell_sel = scraper::Selector::parse("td, th").ok()?;
        let rows: Vec<Vec<String>> = table_el
            .select(&row_sel)
            .take(100)
            .map(|tr| {
                let mut cells = Vec::new();
                for cell in tr.select(&cell_sel).take(20) {
                    let span = cell
                        .value()
                        .attr("colspan")
                        .and_then(|v| v.parse::<usize>().ok())
                        .unwrap_or(1)
                        .clamp(1, 20);
                    cells.push(cell_text(cell));
                    for _ in 1..span {
                        cells.push(String::new());
                    }
                }
                cells
            })
            .filter(|r| !r.is_empty())
            .collect();
        if rows.is_empty() {
            return None;
        }
        let cols = rows.iter().map(Vec::len).max().unwrap_or(1);
        let mut t = Self::new(rows.len(), cols);
        for (r, row) in rows.into_iter().enumerate() {
            for (c, text) in row.into_iter().enumerate() {
                t.cells[r][c].text = text;
            }
        }
        Some(t)
    }
}
/// A table cell's visible text, with `<br>` turned into a newline so a
/// wrapped cell survives the round trip instead of its lines running
/// together.
fn cell_text(cell: scraper::ElementRef) -> String {
    fn walk(node: ego_tree::NodeRef<scraper::node::Node>, out: &mut String) {
        match node.value() {
            scraper::node::Node::Text(t) => out.push_str(t),
            scraper::node::Node::Element(e) if e.name() == "br" => out.push('\n'),
            _ => {
                for child in node.children() {
                    walk(child, out);
                }
            }
        }
    }
    let mut out = String::new();
    walk(*cell, &mut out);
    out.trim().to_owned()
}
/// An image stored in a tab. It is shown inline wherever its placeholder
/// character (`placeholder(id)`) appears in the tab's text, so it moves,
/// wraps, copies and deletes exactly like a character.
#[derive(Clone)]
pub struct Picture {
    pub id: u32,
    pub name: String,
    pub pixels: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub scale: f32,
    pub fit: bool,
    /// File name in the data folder's `images` folder once saved. A saved
    /// picture keeps no pixels in memory: they are read from the file only
    /// while it is shown, copied or exported.
    pub file: String,
    texture: Option<egui::TextureHandle>,
    /// When the texture was last drawn (egui time).
    texture_used: f64,
}
impl Picture {
    /// A picture from RGBA pixels (used when loading saved memos).
    pub fn from_rgba(
        id: u32,
        name: String,
        pixels: Vec<u8>,
        size: [usize; 2],
        scale: f32,
        fit: bool,
    ) -> Self {
        Self {
            id,
            name,
            pixels,
            width: size[0],
            height: size[1],
            scale,
            fit,
            file: String::new(),
            texture: None,
            texture_used: 0.,
        }
    }
    /// A saved picture whose pixels stay in its file until shown.
    pub fn saved(
        id: u32,
        name: String,
        file: String,
        size: [usize; 2],
        scale: f32,
        fit: bool,
    ) -> Self {
        let mut p = Self::from_rgba(id, name, vec![], size, scale, fit);
        p.file = file;
        p
    }
    /// The RGBA pixels, read from the saved file when not in memory. Empty
    /// when neither is available.
    pub fn rgba(&self) -> std::borrow::Cow<'_, [u8]> {
        if self.width > 0 && self.pixels.len() == self.width * self.height * 4 {
            std::borrow::Cow::Borrowed(&self.pixels)
        } else {
            crate::storage::load_picture(&self.file, [self.width, self.height])
                .map(std::borrow::Cow::Owned)
                .unwrap_or(std::borrow::Cow::Borrowed(&[]))
        }
    }
    /// Free the texture of a saved picture not drawn since `before`; it is
    /// read from its file again when next shown.
    pub fn release_texture(&mut self, before: f64) {
        if self.texture_used < before && !self.file.is_empty() {
            self.texture = None;
        }
    }
    fn color_image(&self) -> Option<egui::ColorImage> {
        let px = self.rgba();
        (self.width > 0 && px.len() == self.width * self.height * 4)
            .then(|| egui::ColorImage::from_rgba_unmultiplied([self.width, self.height], &px))
    }
}
#[derive(Clone)]
pub enum Block {
    Table(Table),
    Picture(Picture),
}
#[derive(Clone)]
pub struct Tab {
    pub title: String,
    pub body: String,
    pub hidden: bool,
    pub spans: Vec<Span>,
    pub blocks: Vec<Block>,
}
pub const PICTURE_BASE: u32 = 0xE000;
pub fn placeholder(id: u32) -> char {
    char::from_u32(PICTURE_BASE + id.min(0x18FF)).unwrap_or('\u{E000}')
}
fn picture_id(c: char) -> Option<u32> {
    let v = c as u32;
    (PICTURE_BASE..=PICTURE_BASE + 0x18FF)
        .contains(&v)
        .then(|| v - PICTURE_BASE)
}
impl Tab {
    /// True when the picture's placeholder is still present in the text.
    pub fn shows(&self, block: &Block) -> bool {
        match block {
            Block::Picture(p) => self.body.contains(placeholder(p.id)),
            Block::Table(_) => true,
        }
    }
    fn next_picture_id(&self) -> u32 {
        self.blocks
            .iter()
            .filter_map(|b| match b {
                Block::Picture(p) => Some(p.id + 1),
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }
    /// Insert a picture at a character range, replacing it. Returns the caret.
    pub fn insert_picture(&mut self, range: Range<usize>, mut picture: Picture) -> usize {
        picture.id = self.next_picture_id();
        let c = placeholder(picture.id);
        self.blocks.push(Block::Picture(picture));
        replace_chars(&mut self.body, range.clone(), &c.to_string());
        range.start + 1
    }
    fn new(title: String) -> Self {
        Self {
            title,
            body: String::new(),
            hidden: false,
            spans: vec![],
            blocks: vec![],
        }
    }
}
#[derive(Clone)]
pub struct Document {
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub trash: Vec<Tab>,
    pub font_size: f32,
    pub font_name: String,
    pub opacity: f32,
    pub toolbar_visible: bool,
    pub zoom: f32,
    pub requested_selection: Option<Range<usize>>,
    pub requested_tab: Option<usize>,
    pub status: String,
    pub image_scale: f32,
    pub image_fit: bool,
    pub format: Format,
    pub editing: bool,
    selection: Range<usize>,
    selected_cell: Option<(usize, usize, usize)>,
    rename: Option<usize>,
    tab_scroll: usize,
    ime: bool,
    zoom_until: f64,
    aliases: Vec<(String, String)>,
    first_tab_changed: bool,
    selected_picture: Option<u32>,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            tabs: vec![Tab::new("본문".into())],
            active: 0,
            trash: vec![],
            font_size: 13.333,
            font_name: "맑은 고딕".into(),
            opacity: 1.,
            toolbar_visible: true,
            zoom: 1.,
            requested_selection: None,
            requested_tab: None,
            status: String::new(),
            image_scale: 1.,
            image_fit: false,
            format: Format::default(),
            editing: false,
            selection: 0..0,
            selected_cell: None,
            rename: None,
            tab_scroll: 0,
            ime: false,
            zoom_until: 0.,
            aliases: vec![],
            first_tab_changed: false,
            selected_picture: None,
        }
    }
}
impl Document {
    pub fn plain_text(&self, first_body: &str) -> String {
        let mut parts = vec![strip_pictures(first_body)];
        for (i, t) in self.tabs.iter().enumerate() {
            if i > 0 {
                parts.push(strip_pictures(&t.body));
            }
            for b in &t.blocks {
                if let Block::Table(t) = b {
                    parts.push(t.tsv());
                }
            }
        }
        parts.join("\n")
    }
    pub fn has_images(&self) -> bool {
        self.tabs.iter().any(|t| {
            t.blocks
                .iter()
                .any(|b| matches!(b, Block::Picture(_)) && t.shows(b))
        })
    }
    pub fn has_tables(&self) -> bool {
        self.tabs
            .iter()
            .any(|t| t.blocks.iter().any(|b| matches!(b, Block::Table(_))))
    }
    pub fn add_tab(&mut self) {
        let n = self.tabs.len() + 1;
        self.tabs.push(Tab::new(format!("탭 {n}")));
        self.active = self.tabs.len() - 1;
        self.selection = 0..0;
        self.selected_cell = None;
    }
    pub fn delete_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        self.trash.push(self.tabs.remove(index));
        self.first_tab_changed |= index == 0;
        if self.tabs.is_empty() {
            self.tabs.push(Tab::new("본문".into()));
        }
        self.active = self.active.min(self.tabs.len() - 1);
        if self.tabs[self.active].hidden {
            self.active = self.tabs.iter().position(|t| !t.hidden).unwrap_or(0);
        }
        self.selection = 0..0;
        self.selected_cell = None;
    }
    pub fn toggle_format(&mut self, kind: usize) {
        match kind {
            0 => self.format.bold = !self.format.bold,
            1 => self.format.italic = !self.format.italic,
            2 => self.format.underline = !self.format.underline,
            _ => self.format.strike = !self.format.strike,
        }
        if let Some((b, r, c)) = self.selected_cell {
            if let Some(Block::Table(t)) = self.tabs[self.active].blocks.get_mut(b) {
                if let Some(cell) = t.cells.get_mut(r).and_then(|r| r.get_mut(c)) {
                    cell.format = self.format.clone();
                }
            }
        } else if !self.selection.is_empty() {
            self.tabs[self.active].spans.push(Span {
                range: self.selection.clone(),
                format: self.format.clone(),
            });
        }
    }
}

/// Render an inline editor with its footer anchored to the bottom of its available panel.
/// The text widget id is shared with `content`; rendering order does not replace drafts.
pub fn show(
    ui: &mut egui::Ui,
    legacy_body: &mut String,
    doc: &mut Document,
    id: impl Hash,
) -> egui::Response {
    show_with(ui, legacy_body, doc, id, 0.)
}
/// Like [`show`] for panes inside the main window: no window-opacity tool,
/// and `trailing_space` kept free at the right of the tool row for the
/// caller's own buttons.
pub fn show_with(
    ui: &mut egui::Ui,
    legacy_body: &mut String,
    doc: &mut Document,
    id: impl Hash,
    trailing_space: f32,
) -> egui::Response {
    let id = ui.make_persistent_id(id);
    let top = ui.cursor().min;
    let bottom = ui.clip_rect().bottom().min(ui.ctx().screen_rect().bottom());
    let width = ui.available_width().min(ui.clip_rect().width()).max(32.);
    let height = ui
        .available_height()
        .min((bottom - top.y).max(120.))
        .max(120.);
    let (bounds, _) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
    let mut footer_ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt((id, "footer_region"))
            .max_rect(bounds)
            .layout(egui::Layout::bottom_up(egui::Align::Min)),
    );
    render_footer(&mut footer_ui, doc, id, trailing_space, true, true, false);
    let content_bottom =
        (footer_ui.min_rect().top() - ui.spacing().item_spacing.y).max(bounds.top());
    let content_rect =
        egui::Rect::from_min_max(bounds.min, egui::pos2(bounds.right(), content_bottom));
    let mut content_ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt((id, "content_region"))
            .max_rect(content_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    content_impl(&mut content_ui, legacy_body, doc, id)
}
/// Render only editable content inside the parent's finite available area.
/// Use `footer` in the parent's bottom panel to keep all controls outside scrolling content.
pub fn content(
    ui: &mut egui::Ui,
    legacy_body: &mut String,
    doc: &mut Document,
    id: impl Hash,
) -> egui::Response {
    let id = ui.make_persistent_id(id);
    content_impl(ui, legacy_body, doc, id)
}
/// Render tabs, a divider, then the existing icon tools. `trailing_space` reserves
/// the right end of the icon row for the parent's related-note control.
pub fn footer(
    ui: &mut egui::Ui,
    doc: &mut Document,
    id: impl Hash,
    trailing_space: f32,
    divider: bool,
) -> egui::Response {
    let id = ui.make_persistent_id(id);
    render_footer(ui, doc, id, trailing_space, false, divider, true)
}
fn content_impl(
    ui: &mut egui::Ui,
    legacy_body: &mut String,
    doc: &mut Document,
    id: Id,
) -> egui::Response {
    if doc.tabs.is_empty() {
        doc.tabs.push(Tab::new("본문".into()));
    }
    doc.active = doc.active.min(doc.tabs.len() - 1);
    if doc.first_tab_changed {
        *legacy_body = doc.tabs[0].body.clone();
        doc.first_tab_changed = false;
    }
    if doc.tabs[0].body != *legacy_body {
        let tab = &mut doc.tabs[0];
        transform_spans(&tab.body, legacy_body, &mut tab.spans, &Format::default());
        tab.body = legacy_body.clone();
    }
    let requested = doc.requested_selection.take();
    let requested_tab = doc.requested_tab.take();
    let requested_focus = requested.is_some();
    if requested.is_some() || requested_tab.is_some() {
        doc.active = requested_tab.filter(|&i| i < doc.tabs.len()).unwrap_or(0);
    }
    let text_id = id.with(("text", doc.active));
    let before = doc.tabs[doc.active].body.clone();
    let hover = ui.rect_contains_pointer(ui.available_rect_before_wrap());
    let zoom_delta = ui.input(|i| {
        if hover && i.modifiers.ctrl {
            i.raw_scroll_delta.y
        } else {
            0.
        }
    });
    if zoom_delta != 0. {
        doc.zoom = (doc.zoom + zoom_delta.signum() * 0.05).clamp(0.3, 2.);
        doc.zoom_until = ui.input(|i| i.time) + 0.9;
        ui.input_mut(|i| {
            i.raw_scroll_delta = Vec2::ZERO;
            i.smooth_scroll_delta = Vec2::ZERO;
        });
    }
    ui.input(|i| {
        for e in &i.events {
            if let egui::Event::Ime(ev) = e {
                match ev {
                    egui::ImeEvent::Preedit(_) => doc.ime = true,
                    egui::ImeEvent::Commit(_) | egui::ImeEvent::Disabled => doc.ime = false,
                    _ => {}
                }
            }
        }
    });
    let focused = ui.memory(|m| m.has_focus(text_id));
    let had_preedit = ui.input(|i| {
        i.events
            .iter()
            .any(|e| matches!(e, egui::Event::Ime(egui::ImeEvent::Preedit(_))))
    });
    let editing = focused || hover;
    doc.editing = editing;
    if focused && !doc.ime && !had_preedit {
        for (key, kind) in [(Key::B, 0), (Key::I, 1), (Key::U, 2)] {
            if ui.input_mut(|i| i.consume_key(Modifiers::CTRL, key)) {
                doc.toggle_format(kind);
            }
        }
    }
    let mut pending_cursor = None;
    if let Some(r) = requested {
        let len = doc.tabs[doc.active].body.chars().count();
        let r = r.start.min(len)..r.end.min(len);
        let mut state = egui::TextEdit::load_state(ui.ctx(), text_id).unwrap_or_default();
        state.cursor.set_char_range(Some(CCursorRange::two(
            CCursor::new(r.start),
            CCursor::new(r.end),
        )));
        state.store(ui.ctx(), text_id);
        ui.memory_mut(|m| m.request_focus(text_id));
    }
    if focused && !doc.ime && !had_preedit {
        let old_range = egui::TextEdit::load_state(ui.ctx(), text_id)
            .and_then(|s| s.cursor.char_range())
            .map(|r| {
                let [a, b] = r.sorted();
                a.index..b.index
            })
            .unwrap_or_else(|| doc.selection.clone());
        if old_range.is_empty() && ui.input(|i| i.key_pressed(Key::Backspace)) {
            let strike = doc.tabs[doc.active]
                .spans
                .iter_mut()
                .rev()
                .find(|s| s.range.contains(&old_range.start.saturating_sub(1)));
            if let Some(s) = strike.filter(|s| s.format.strike) {
                s.format.strike = false;
                ui.input_mut(|i| {
                    i.consume_key(Modifiers::NONE, Key::Backspace);
                });
            }
        }
        let enter = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
        let outdent = ui.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::Tab));
        let indent = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Tab));
        let stamp =
            ui.input_mut(|i| i.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::Semicolon));
        let date = !stamp && ui.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::Semicolon));
        if enter || outdent || indent || date || stamp {
            let text = &mut doc.tabs[doc.active].body;
            let insert = if enter {
                continuation(text, old_range.start)
            } else if indent {
                "  ".into()
            } else if stamp {
                chrono::Local::now().format("%y-%m-%d %H:%M:%S").to_string()
            } else if date {
                chrono::Local::now().format("%y-%m-%d").to_string()
            } else {
                String::new()
            };
            if outdent {
                pending_cursor = Some(outdent_line(text, old_range.start));
            } else {
                replace_chars(text, old_range.clone(), &insert);
                pending_cursor = Some(old_range.start + insert.chars().count());
            }
            if enter {
                doc.format.strike = false;
            }
        }
        let paste = ui.input(|i| {
            i.events.iter().find_map(|e| {
                if let egui::Event::Paste(t) = e {
                    Some(t.clone())
                } else {
                    None
                }
            })
        });
        // Placeholders only come from real images; never accept stray ones.
        ui.input_mut(|i| {
            for e in &mut i.events {
                if let egui::Event::Paste(t) | egui::Event::Text(t) = e {
                    if t.chars().any(|c| picture_id(c).is_some()) {
                        *t = strip_pictures(t);
                    }
                }
            }
        });
        let mut table_pasted = false;
        let text_pasted = paste.is_some();
        if let Some(paste) = paste.filter(|t| t.contains('\t') && t.len() < 1024 * 1024) {
            // Excel/한글/Word put both a plain TSV rendering and the real
            // `<table>` markup on the clipboard. TSV alone breaks on a cell
            // that itself contains a tab or line break (common with wrapped
            // cells), so prefer the HTML when it is there and parses.
            let table = arboard::Clipboard::new()
                .and_then(|mut c| c.get().html())
                .ok()
                .and_then(|html| Table::from_html(&html))
                .unwrap_or_else(|| Table::from_tsv(&paste));
            doc.tabs[doc.active].blocks.push(Block::Table(table));
            ui.input_mut(|i| i.events.retain(|e| !matches!(e, egui::Event::Paste(_))));
            table_pasted = true;
        }
        if !table_pasted && !text_pasted && paste_chord(ui, text_id) {
            // egui's own Paste(text) event can arrive a frame or more after
            // this key-down edge, so `text_pasted` above does not yet know
            // whether this same Ctrl+V is about to deliver table/text too.
            // A table clipboard (e.g. from Excel) also carries an image
            // representation; without this check that image would land here
            // first and the text would still turn into a second, separate
            // paste once its event finally showed up. Read the clipboard's
            // own text synchronously instead of waiting on that event.
            let clipboard_has_text = arboard::Clipboard::new()
                .and_then(|mut c| c.get_text())
                .is_ok_and(|t| !t.trim().is_empty());
            if !clipboard_has_text {
                if let Ok(picture) = picture_from_clipboard(doc.image_scale, doc.image_fit) {
                    pending_cursor =
                        Some(doc.tabs[doc.active].insert_picture(old_range.clone(), picture));
                    ui.input_mut(|i| {
                        i.events.retain(|e| {
                            !matches!(
                                e,
                                egui::Event::Paste(_) | egui::Event::Key { key: Key::V, .. }
                            )
                        })
                    });
                }
            }
        }
        if let Some(id) = doc.selected_picture {
            if ui.input_mut(|i| {
                i.consume_key(Modifiers::NONE, Key::Delete)
                    || i.consume_key(Modifiers::NONE, Key::Backspace)
            }) {
                let c = placeholder(id);
                let tab = &mut doc.tabs[doc.active];
                if let Some(at) = tab.body.chars().position(|x| x == c) {
                    replace_chars(&mut tab.body, at..at + 1, "");
                    pending_cursor = Some(at);
                }
                doc.selected_picture = None;
            }
        }
    }
    let font = font_id(ui, doc.font_size * doc.zoom, &doc.font_name, false);
    let bold_font = font_id(ui, doc.font_size * doc.zoom, &doc.font_name, true);
    let spans = doc.tabs[doc.active].spans.clone();
    let base = Format::default();
    let color = ui.visuals().text_color();
    let pictures = picture_sizes(&doc.tabs[doc.active], doc.zoom);
    let space_ratio = ui
        .fonts(|f| f.glyph_width(&FontId::new(100., font.family.clone()), '\u{3000}'))
        .max(1.)
        / 100.;
    let mut layouter = |ui: &egui::Ui, text: &str, width: f32| {
        let mut job = layout(text, &spans, &base, font.clone(), bold_font.clone(), color);
        inline_pictures(&mut job, &pictures, width, space_ratio);
        job.wrap.max_width = width;
        ui.fonts(|f| f.layout_job(job))
    };
    let bottom = ui.ctx().screen_rect().bottom().min(ui.clip_rect().bottom());
    let body_height = ui
        .available_height()
        .min((bottom - ui.cursor().top()).max(1.))
        .max(1.);
    let rows = (body_height / (doc.font_size * doc.zoom * 1.4)).clamp(1., 18.) as usize;
    let mut output = egui::ScrollArea::vertical()
        .id_salt(id.with(("text_scroll", doc.active)))
        .max_height(body_height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let text = &mut doc.tabs[doc.active].body;
            let output = egui::TextEdit::multiline(text)
                .id(text_id)
                .font(font.clone())
                .frame(false)
                .desired_width(f32::INFINITY)
                .desired_rows(rows)
                .lock_focus(true)
                .hint_text("여기에 메모하세요…")
                .layouter(&mut layouter)
                .show(ui);
            if requested_focus {
                if let Some(range) = output.cursor_range {
                    ui.scroll_to_rect(
                        output
                            .galley
                            .pos_from_cursor(&range.primary)
                            .translate(output.galley_pos.to_vec2()),
                        Some(egui::Align::Center),
                    );
                }
            }
            paint_pictures(ui, doc, id, &output, &pictures);
            show_links(ui, doc, id);
            show_blocks(ui, doc, id);
            output
        })
        .inner;
    if let Some(cursor) = pending_cursor {
        output
            .state
            .cursor
            .set_char_range(Some(CCursorRange::one(CCursor::new(cursor))));
    }
    doc.selection = output
        .state
        .cursor
        .char_range()
        .map(|r| {
            let [a, b] = r.sorted();
            a.index..b.index
        })
        .unwrap_or(0..0);
    if output.response.has_focus() {
        doc.selected_cell = None;
    }
    let mut new_strike = None;
    if output.response.has_focus() && !doc.ime && !had_preedit {
        let space = ui.input(|i| {
            i.events
                .iter()
                .any(|e| matches!(e,egui::Event::Text(t) if t.ends_with(' ')))
        });
        let text = &mut doc.tabs[doc.active].body;
        if space && output.response.changed() {
            let cursor = doc.selection.end;
            if let Some((range, replace, style_range)) = shortcut_substitution(text, cursor) {
                replace_chars(text, range.clone(), &replace);
                let new_cursor = range.start + replace.chars().count();
                new_strike = style_range;
                output
                    .state
                    .cursor
                    .set_char_range(Some(CCursorRange::one(CCursor::new(new_cursor))));
                doc.format.strike = false;
            } else if let Some(cursor) = normalize_outline_line(text, cursor) {
                output
                    .state
                    .cursor
                    .set_char_range(Some(CCursorRange::one(CCursor::new(cursor))));
            }
        }
    }
    if before != doc.tabs[doc.active].body {
        let tab = &mut doc.tabs[doc.active];
        transform_spans(&before, &tab.body, &mut tab.spans, &doc.format);
    }
    if let Some(range) = new_strike {
        let mut format = doc.format.clone();
        format.strike = true;
        doc.tabs[doc.active].spans.push(Span { range, format });
    }
    output.state.store(ui.ctx(), text_id);
    let marks: Vec<_> = doc.tabs[doc.active]
        .body
        .chars()
        .enumerate()
        .filter(|(_, c)| matches!(c, '☐' | '☑'))
        .collect();
    for (idx, ch) in marks {
        let caret = output
            .galley
            .pos_from_ccursor(CCursor::new(idx))
            .translate(output.galley_pos.to_vec2());
        let rect = egui::Rect::from_min_size(caret.min, Vec2::splat((font.size * 1.1).max(13.)));
        if output.text_clip_rect.contains_rect(rect)
            && ui
                .interact(
                    rect,
                    id.with(("check", doc.active, idx)),
                    egui::Sense::click(),
                )
                .on_hover_text("체크 상태 바꾸기")
                .clicked()
        {
            replace_chars(
                &mut doc.tabs[doc.active].body,
                idx..idx + 1,
                if ch == '☐' { "☑" } else { "☐" },
            );
        }
    }
    let response = output.response;
    response.context_menu(|ui| {
        if ui.button("원본 배율 100%").clicked() {
            doc.zoom = 1.;
            ui.close_menu();
        }
        if ui.button("체크박스 삽입").clicked() {
            let pos = doc.selection.end;
            replace_chars(&mut doc.tabs[doc.active].body, pos..pos, "☐ ");
            ui.close_menu();
        }
    });
    if hover {
        let paths = ui.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect::<Vec<_>>()
        });
        for path in paths {
            match picture_from_file(&path, doc.image_scale, doc.image_fit) {
                Ok(p) => {
                    let range = doc.selection.clone();
                    let caret = doc.tabs[doc.active].insert_picture(range, p);
                    doc.selection = caret..caret;
                }
                Err(e) => doc.status = e,
            }
        }
    }
    if ui.input(|i| i.time) < doc.zoom_until {
        let rect = response.rect;
        let text = format!("{}%", (doc.zoom * 100.).round() as i32);
        let p = rect.center();
        ui.painter().rect_filled(
            egui::Rect::from_center_size(p, Vec2::new(76., 38.)),
            8.,
            Color32::from_black_alpha(155),
        );
        ui.painter().text(
            p,
            egui::Align2::CENTER_CENTER,
            text,
            FontId::proportional(20.),
            Color32::WHITE,
        );
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(100));
    }
    *legacy_body = doc.tabs[0].body.clone();
    response
}

/// Read-only rendering for a saved comment. The document and legacy text are
/// never changed. Tab selection and image textures belong to temporary UI state.
pub fn view(ui: &mut egui::Ui, legacy: &str, doc: &Document, id: impl Hash) {
    let id = ui.make_persistent_id(id);
    if doc.tabs.is_empty() {
        ui.label(legacy);
        return;
    }
    let visible: Vec<_> = doc
        .tabs
        .iter()
        .enumerate()
        .filter(|(_, t)| !t.hidden)
        .map(|(i, _)| i)
        .collect();
    let fallback = if visible.contains(&doc.active) {
        doc.active
    } else {
        visible.first().copied().unwrap_or(0)
    };
    let selection_id = id.with("view_tab");
    let mut active = ui
        .ctx()
        .data(|d| d.get_temp::<usize>(selection_id))
        .filter(|&i| i < doc.tabs.len() && !doc.tabs[i].hidden)
        .unwrap_or(fallback);
    if visible.len() > 1 {
        ui.horizontal_wrapped(|ui| {
            for &i in &visible {
                if ui
                    .selectable_label(active == i, &doc.tabs[i].title)
                    .clicked()
                {
                    active = i;
                }
            }
        });
    }
    ui.ctx().data_mut(|d| d.insert_temp(selection_id, active));
    let tab = &doc.tabs[active];
    let body = if active == 0 {
        legacy
    } else {
        tab.body.as_str()
    };
    let font = font_id(ui, doc.font_size, &doc.font_name, false);
    let bold = font_id(ui, doc.font_size, &doc.font_name, true);
    let mut job = layout(
        body,
        &tab.spans,
        &Format::default(),
        font.clone(),
        bold.clone(),
        ui.visuals().text_color(),
    );
    job.wrap.max_width = ui.available_width();
    ui.add(egui::Label::new(job).selectable(true))
        .context_menu(|ui| {
            if ui.button("본문 복사").clicked() {
                ui.ctx().copy_text(body.to_owned());
                ui.close_menu();
            }
        });
    let links = urls(body);
    if !links.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for url in links {
                let name = doc
                    .aliases
                    .iter()
                    .find(|(address, _)| address == &url)
                    .map(|(_, name)| name.as_str())
                    .unwrap_or(&url);
                let r = ui
                    .link(short(name, 36))
                    .on_hover_text(format!("{url}\nCtrl+클릭: 주소 복사"));
                if r.clicked() {
                    if ui.input(|i| i.modifiers.ctrl) {
                        ui.ctx().copy_text(url);
                    } else {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                    }
                }
            }
        });
    }
    for (bi, block) in tab.blocks.iter().enumerate() {
        ui.add_space(6.);
        match block {
            Block::Table(table) => {
                if table.cells.is_empty() {
                    continue;
                }
                egui::ScrollArea::horizontal()
                    .id_salt(id.with(("view_table", active, bi)))
                    .show(ui, |ui| {
                        egui::Grid::new(id.with(("view_grid", active, bi)))
                            .spacing([1., 1.])
                            .striped(true)
                            .show(ui, |ui| {
                                for row in &table.cells {
                                    for cell in row {
                                        egui::Frame::NONE
                                            .inner_margin(6.)
                                            .stroke(Stroke::new(
                                                1.0_f32,
                                                ui.visuals().widgets.noninteractive.bg_stroke.color,
                                            ))
                                            .show(ui, |ui| {
                                                ui.set_width(table.width.max(60.));
                                                let mut job = layout(
                                                    &cell.text,
                                                    &[],
                                                    &cell.format,
                                                    font.clone(),
                                                    bold.clone(),
                                                    ui.visuals().text_color(),
                                                );
                                                job.wrap.max_width = table.width.max(60.);
                                                ui.add(egui::Label::new(job).selectable(true));
                                            });
                                    }
                                    ui.end_row();
                                }
                            });
                    });
            }
            Block::Picture(picture) => {
                if picture.width == 0 || picture.height == 0 {
                    continue;
                }
                let available = ui.available_width().max(16.);
                let width = if picture.fit {
                    available
                } else {
                    (picture.width as f32 * picture.scale).min(available)
                };
                let size = Vec2::new(width, width * picture.height as f32 / picture.width as f32);
                let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
                // Pictures are read from disk only once they scroll into view.
                if !ui.is_rect_visible(rect) {
                    continue;
                }
                let texture_id =
                    id.with(("view_picture", active, bi, picture.width, picture.height));
                let texture = picture.texture.clone().or_else(|| {
                    crate::lists::texture(ui.ctx(), texture_id, || picture.color_image())
                });
                if let Some(texture) = texture {
                    ui.painter().image(
                        texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1., 1.)),
                        Color32::WHITE,
                    );
                }
                response.on_hover_text(&picture.name);
            }
        }
    }
}

fn font_id(ui: &egui::Ui, size: f32, name: &str, bold: bool) -> FontId {
    let wanted = if bold {
        format!("{name} Bold")
    } else {
        name.into()
    };
    let family = FontFamily::Name(wanted.clone().into());
    let found = ui.fonts(|f| f.families().contains(&family));
    FontId::new(
        size,
        if found {
            family
        } else {
            FontFamily::Proportional
        },
    )
}
fn layout(
    text: &str,
    spans: &[Span],
    base: &Format,
    font: FontId,
    bold_font: FontId,
    color: Color32,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut token_start = 0;
    for (i, (byte, ch)) in text.char_indices().enumerate() {
        let f = spans
            .iter()
            .rev()
            .find(|s| s.range.contains(&i))
            .map(|s| &s.format)
            .unwrap_or(base);
        let stroke = Stroke::new(1.0_f32, color);
        let mut style = TextFormat {
            font_id: if f.bold {
                bold_font.clone()
            } else {
                font.clone()
            },
            color,
            italics: f.italic,
            extra_letter_spacing: if f.bold { 0.25 } else { 0. },
            underline: if f.underline { stroke } else { Stroke::NONE },
            strikethrough: if f.strike { stroke } else { Stroke::NONE },
            ..Default::default()
        };
        // Automatic link emphasis uses the URL's exact lexical extent.
        if ch.is_whitespace() {
            token_start = byte + ch.len_utf8();
        }
        if !ch.is_whitespace()
            && (text[token_start..].starts_with("https://")
                || text[token_start..].starts_with("http://"))
        {
            style.color = Color32::from_rgb(61, 105, 166);
            style.underline = Stroke::new(1.0_f32, style.color);
        }
        job.append(&ch.to_string(), 0., style);
    }
    job
}
fn render_footer(
    ui: &mut egui::Ui,
    doc: &mut Document,
    id: Id,
    trailing_space: f32,
    bottom_up: bool,
    divider: bool,
    opacity: bool,
) -> egui::Response {
    if doc.tabs.is_empty() {
        doc.tabs.push(Tab::new("본문".into()));
    }
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 4.;
        let status = |ui: &mut egui::Ui, doc: &Document| {
            if !doc.status.is_empty() {
                ui.add(
                    egui::Label::new(
                        RichText::new(&doc.status)
                            .small()
                            .color(Color32::from_rgb(155, 75, 45)),
                    )
                    .truncate(),
                )
                .on_hover_text(&doc.status);
            }
        };
        let tools = |ui: &mut egui::Ui, doc: &mut Document| {
            ui.scope(|ui| {
                ui.set_max_width((ui.available_width() - trailing_space.max(0.)).max(28.));
                toolbar(ui, doc, id, opacity);
            });
        };
        let line = |ui: &mut egui::Ui| {
            if divider {
                crate::ui::divider(ui);
            } else {
                ui.add_space(1.);
            }
        };
        if bottom_up {
            if doc.toolbar_visible {
                tools(ui, doc);
                line(ui);
            }
            tabs(ui, doc, id);
            status(ui, doc);
        } else {
            status(ui, doc);
            tabs(ui, doc, id);
            if doc.toolbar_visible {
                line(ui);
                tools(ui, doc);
            }
        }
    })
    .response
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Tool {
    Font,
    Size,
    Bold,
    Italic,
    Underline,
    Strike,
    Opacity,
}
// Images and tables arrive from the clipboard; zoom stays on Ctrl+wheel.
const TOOLS: [Tool; 7] = [
    Tool::Font,
    Tool::Size,
    Tool::Bold,
    Tool::Italic,
    Tool::Underline,
    Tool::Strike,
    Tool::Opacity,
];
fn toolbar_height(width: f32, count: usize) -> f32 {
    let columns = (((width.max(28.) + 2.) / 30.).floor() as usize).clamp(1, count);
    let rows = count.div_ceil(columns);
    rows as f32 * 28. + rows.saturating_sub(1) as f32 * 2.
}
fn toolbar(ui: &mut egui::Ui, doc: &mut Document, id: Id, opacity: bool) -> egui::Response {
    let width = ui.available_width().min(ui.clip_rect().width()).max(28.);
    let tools: Vec<Tool> = TOOLS
        .into_iter()
        .filter(|t| opacity || *t != Tool::Opacity)
        .collect();
    let height = toolbar_height(width, tools.len());
    ui.allocate_ui_with_layout(
        Vec2::new(width, height),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(2.);
            ui.spacing_mut().button_padding = Vec2::new(6., 4.);
            ui.set_max_width(ui.available_width().min(ui.clip_rect().width()).max(28.));
            ui.horizontal_wrapped(|ui| {
                for tool in tools {
                    let (label, selected) = match tool {
                        Tool::Font => (format!("글꼴 · {}", doc.font_name), false),
                        Tool::Size => {
                            (format!("글자 크기 · {:.0} pt", doc.font_size * 0.75), false)
                        }
                        Tool::Bold => ("굵게 · Ctrl+B".into(), doc.format.bold),
                        Tool::Italic => ("기울임 · Ctrl+I".into(), doc.format.italic),
                        Tool::Underline => ("밑줄 · Ctrl+U".into(), doc.format.underline),
                        Tool::Strike => ("취소선".into(), doc.format.strike),
                        Tool::Opacity => {
                            (format!("메모 투명도 · {:.0}%", doc.opacity * 100.), false)
                        }
                    };
                    let size_text =
                        (tool == Tool::Size).then(|| format!("{:.0}", doc.font_size * 0.75));
                    let response = tool_button(ui, tool, &label, selected, size_text);
                    match tool {
                        Tool::Bold | Tool::Italic | Tool::Underline | Tool::Strike => {
                            if response.clicked() {
                                doc.toggle_format(match tool {
                                    Tool::Bold => 0,
                                    Tool::Italic => 1,
                                    Tool::Underline => 2,
                                    _ => 3,
                                });
                            }
                        }
                        _ => {
                            let popup_id = id.with(("tool_popup", tool));
                            if response.clicked() {
                                ui.memory_mut(|m| m.toggle_popup(popup_id));
                            }
                            egui::popup_below_widget(
                                ui,
                                popup_id,
                                &response,
                                egui::PopupCloseBehavior::CloseOnClickOutside,
                                |ui| {
                                    ui.set_min_width(170.);
                                    match tool {
                                        Tool::Font => {
                                            ui.strong("글꼴");
                                            for name in [
                                                "맑은 고딕",
                                                "굴림",
                                                "돋움",
                                                "바탕",
                                                "궁서체",
                                                "개구쟁이",
                                            ] {
                                                if ui
                                                    .selectable_label(doc.font_name == name, name)
                                                    .clicked()
                                                {
                                                    doc.font_name = name.into();
                                                    ui.memory_mut(|m| m.close_popup());
                                                }
                                            }
                                        }
                                        Tool::Size => {
                                            ui.strong("글자 크기");
                                            ui.horizontal_wrapped(|ui| {
                                                for pt in [
                                                    8., 10., 11., 12., 14., 16., 18., 20., 24., 32.,
                                                ] {
                                                    if ui
                                                        .selectable_label(
                                                            (doc.font_size * 0.75 - pt).abs() < 0.1,
                                                            format!("{pt:.0} pt"),
                                                        )
                                                        .clicked()
                                                    {
                                                        doc.font_size = pt / 0.75;
                                                        ui.memory_mut(|m| m.close_popup());
                                                    }
                                                }
                                            });
                                            ui.separator();
                                            let mut pt = (doc.font_size * 0.75).round();
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut pt)
                                                        .range(6.0..=48.0)
                                                        .speed(1.)
                                                        .fixed_decimals(0)
                                                        .suffix(" pt"),
                                                )
                                                .changed()
                                            {
                                                doc.font_size = pt.round() / 0.75;
                                            }
                                        }
                                        Tool::Opacity => {
                                            crate::ui::caption(ui, "메모 투명도");
                                            let mut pct = (doc.opacity * 100.).round();
                                            if ui
                                                .add(
                                                    egui::Slider::new(&mut pct, 20.0..=100.)
                                                        .integer()
                                                        .suffix("%"),
                                                )
                                                .changed()
                                            {
                                                doc.opacity = pct / 100.;
                                            }
                                            ui.horizontal(|ui| {
                                                for v in [40., 60., 80., 100.] {
                                                    if ui
                                                        .selectable_label(
                                                            (pct - v).abs() < 0.5,
                                                            format!("{v:.0}%"),
                                                        )
                                                        .clicked()
                                                    {
                                                        doc.opacity = v / 100.;
                                                    }
                                                }
                                            });
                                        }
                                        _ => {}
                                    }
                                },
                            );
                        }
                    }
                }
            });
        },
    )
    .response
}
fn tool_button(
    ui: &mut egui::Ui,
    tool: Tool,
    label: &str,
    selected: bool,
    text: Option<String>,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(28.), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    if selected || response.hovered() || response.has_focus() {
        ui.painter().rect_filled(
            rect,
            5.,
            if selected {
                ui.visuals().selection.bg_fill
            } else {
                ui.visuals().widgets.hovered.weak_bg_fill
            },
        );
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            5.,
            Stroke::new(1.0_f32, ui.visuals().selection.stroke.color),
            egui::StrokeKind::Inside,
        );
    }
    let color = ui
        .visuals()
        .text_color()
        .gamma_multiply(if selected || response.hovered() {
            1.
        } else {
            0.78
        });
    if let Some(text) = text {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            FontId::proportional(12.5),
            color,
        );
    } else {
        paint_tool(ui.painter(), rect.shrink(5.), tool, color);
    }
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(label)
}
fn paint_tool(painter: &egui::Painter, rect: egui::Rect, tool: Tool, color: Color32) {
    let unit = rect.width().min(rect.height()) / 24.;
    let origin = rect.center() - Vec2::splat(12. * unit);
    let p = |x: f32, y: f32| origin + Vec2::new(x, y) * unit;
    let stroke = Stroke::new(1.6_f32 * unit, color);
    let line = |a: (f32, f32), b: (f32, f32)| {
        painter.line_segment([p(a.0, a.1), p(b.0, b.1)], stroke);
    };
    let path = |points: &[(f32, f32)]| {
        painter.add(egui::Shape::line(
            points.iter().map(|&(x, y)| p(x, y)).collect(),
            stroke,
        ));
    };
    match tool {
        Tool::Font => {
            path(&[(4., 20.), (11., 4.), (18., 20.)]);
            line((7., 13.), (15., 13.));
            line((17., 5.), (22., 5.));
            line((19.5, 5.), (19.5, 10.));
        }
        Tool::Size => {
            line((3., 5.), (15., 5.));
            line((9., 5.), (9., 20.));
            line((5., 20.), (13., 20.));
            line((17., 10.), (23., 10.));
            line((20., 10.), (20., 20.));
        }
        Tool::Bold => {
            path(&[
                (6., 20.),
                (6., 4.),
                (13., 4.),
                (17., 6.),
                (17., 9.),
                (13., 12.),
                (6., 12.),
            ]);
            path(&[(13., 12.), (18., 14.), (18., 18.), (14., 20.), (6., 20.)]);
        }
        Tool::Italic => {
            line((10., 4.), (20., 4.));
            line((5., 20.), (15., 20.));
            line((15., 4.), (10., 20.));
        }
        Tool::Underline => {
            path(&[
                (6., 4.),
                (6., 13.),
                (8., 17.),
                (12., 18.),
                (16., 17.),
                (18., 13.),
                (18., 4.),
            ]);
            line((4., 22.), (20., 22.));
        }
        Tool::Strike => {
            path(&[
                (18., 6.),
                (15., 3.),
                (9., 3.),
                (6., 6.),
                (7., 9.),
                (16., 15.),
                (18., 18.),
                (15., 21.),
                (9., 21.),
                (6., 19.),
            ]);
            line((3., 12.), (21., 12.));
        }
        Tool::Opacity => {
            painter.circle_stroke(p(12., 12.), 8. * unit, stroke);
            let mut points = vec![p(12., 4.), p(12., 20.)];
            for i in 0..=12 {
                let angle = std::f32::consts::FRAC_PI_2 - i as f32 * std::f32::consts::PI / 12.;
                points.push(p(12. + 8. * angle.cos(), 12. + 8. * angle.sin()));
            }
            painter.add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
        }
    }
}

/// A compact tab chip: the selected one is lifted, the rest are plain text.
/// A document tab: plain text, the selected one carries an underline.
fn tab_chip(ui: &mut egui::Ui, text: &str, selected: bool) -> egui::Response {
    let ink = ui.visuals().text_color();
    let galley = ui.painter().layout_no_wrap(
        text.into(),
        FontId::proportional(12.5),
        if selected {
            ink
        } else {
            ink.gamma_multiply(0.55)
        },
    );
    let (rect, r) =
        ui.allocate_exact_size(Vec2::new(galley.size().x + 16., 24.), egui::Sense::click());
    if r.hovered() && !selected {
        ui.painter().rect_filled(rect, 6., ink.gamma_multiply(0.07));
    }
    let text_rect = egui::Rect::from_center_size(rect.center() - Vec2::new(0., 1.), galley.size());
    ui.painter().galley(text_rect.min, galley, ink);
    if selected {
        let y = rect.bottom() - 1.5;
        ui.painter().line_segment(
            [
                egui::pos2(text_rect.left(), y),
                egui::pos2(text_rect.right(), y),
            ],
            Stroke::new(2.0_f32, ink.gamma_multiply(0.8)),
        );
    }
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
}
/// A small round icon button used beside the tabs.
fn tab_icon(ui: &mut egui::Ui, icon: crate::icons::Icon, tip: &str) -> egui::Response {
    let ink = ui.visuals().text_color();
    let (rect, r) = ui.allocate_exact_size(Vec2::new(22., 24.), egui::Sense::click());
    if r.hovered() {
        ui.painter()
            .circle_filled(rect.center(), 10., ink.gamma_multiply(0.08));
    }
    crate::icons::paint_icon(
        ui.painter(),
        egui::Rect::from_center_size(rect.center(), Vec2::splat(12.)),
        icon,
        ink.gamma_multiply(if r.hovered() { 0.85 } else { 0.5 }),
    );
    r.on_hover_text(tip)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}
fn tabs(ui: &mut egui::Ui, doc: &mut Document, id: Id) {
    ui.add_space(2.);
    let mut delete = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.;
        let visible: Vec<_> = doc
            .tabs
            .iter()
            .enumerate()
            .filter(|(_, t)| !t.hidden)
            .map(|(i, _)| i)
            .collect();
        let limit = ((ui.available_width() - 82.) / 82.).floor().max(1.) as usize;
        let max_scroll = visible.len().saturating_sub(limit);
        doc.tab_scroll = doc.tab_scroll.min(max_scroll);
        if max_scroll > 0 && tab_icon(ui, crate::icons::Icon::ChevronLeft, "이전 탭").clicked() {
            doc.tab_scroll = doc.tab_scroll.saturating_sub(1);
        }
        for &i in visible.iter().skip(doc.tab_scroll).take(limit) {
            let r = tab_chip(ui, &short(&doc.tabs[i].title, 10), doc.active == i);
            if r.clicked() {
                doc.active = i;
                doc.selected_cell = None;
                doc.selection = 0..0;
            }
            if r.double_clicked() {
                doc.rename = Some(i);
            }
            r.context_menu(|ui| {
                if ui.button("이름 바꾸기").clicked() {
                    doc.rename = Some(i);
                    ui.close_menu();
                }
                if visible.len() > 1 && ui.button("이 탭 숨기기").clicked() {
                    doc.tabs[i].hidden = true;
                    if doc.active == i {
                        doc.active = doc.tabs.iter().position(|t| !t.hidden).unwrap_or(0);
                    }
                    ui.close_menu();
                }
                if ui.button("탭 삭제 · 휴지통으로").clicked() {
                    delete = Some(i);
                    ui.close_menu();
                }
                ui.menu_button("숨긴 탭 보이기", |ui| {
                    for t in doc.tabs.iter_mut().filter(|t| t.hidden) {
                        if ui.button(&t.title).clicked() {
                            t.hidden = false;
                            ui.close_menu();
                        }
                    }
                });
            });
        }
        if max_scroll > 0 && tab_icon(ui, crate::icons::Icon::ChevronRight, "다음 탭").clicked()
        {
            doc.tab_scroll = (doc.tab_scroll + 1).min(max_scroll);
        }
        if tab_icon(
            ui,
            crate::icons::Icon::Plus,
            "본문 탭 추가 · 메모 속성은 함께 사용",
        )
        .clicked()
        {
            doc.add_tab();
            doc.tab_scroll = max_scroll + 1;
        }
    });
    if let Some(i) = delete {
        doc.delete_tab(i);
    }
    if let Some(i) = doc.rename {
        if i < doc.tabs.len() {
            let mut open = true;
            crate::ui::dialog(
                ui.ctx(),
                id.with("rename_popup"),
                "탭 이름 바꾸기",
                [300., 150.],
                true,
                &mut open,
                |ui| {
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut doc.tabs[i].title)
                            .id(id.with("rename"))
                            .desired_width(f32::INFINITY)
                            .margin(egui::Margin::symmetric(10, 6)),
                    );
                    r.request_focus();
                    let enter = ui.input(|i| i.key_pressed(Key::Enter));
                    let (ok, _) = crate::ui::dialog_buttons(ui, "확인", "");
                    if ok || enter {
                        doc.rename = None;
                    }
                },
            );
            if !open {
                doc.rename = None;
            }
        } else {
            doc.rename = None;
        }
    }
}
fn show_links(ui: &mut egui::Ui, doc: &mut Document, id: Id) {
    let urls = urls(&doc.tabs[doc.active].body);
    if urls.is_empty() {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        for url in urls {
            let pos = doc
                .aliases
                .iter()
                .position(|(u, _)| u == &url)
                .unwrap_or_else(|| {
                    doc.aliases.push((url.clone(), url.clone()));
                    doc.aliases.len() - 1
                });
            let r = ui
                .link(short(&doc.aliases[pos].1, 32))
                .on_hover_text(format!("{url}\nCtrl+클릭: 주소 복사 · 우클릭: 이름 변경"));
            if r.clicked() {
                if ui.input(|i| i.modifiers.ctrl) {
                    ui.ctx().copy_text(url.clone());
                    doc.status = "링크 주소를 복사했습니다".into();
                } else {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(url.clone()));
                }
            }
            r.context_menu(|ui| {
                ui.label("하이퍼링크 이름");
                ui.add(
                    egui::TextEdit::singleline(&mut doc.aliases[pos].1)
                        .id(id.with(("alias", pos)))
                        .desired_width(170.),
                );
                if ui.button("주소 복사").clicked() {
                    ui.ctx().copy_text(url.clone());
                    ui.close_menu();
                }
            });
        }
    });
}
/// True on the frame Ctrl+V goes down. egui turns Ctrl+V into a text paste
/// event and drops the key entirely, so an image-only clipboard produces no
/// event at all; read the key state directly instead.
fn paste_chord(ui: &egui::Ui, id: Id) -> bool {
    let key = id.with("paste_chord");
    let down = chord_down() || ui.input(|i| i.modifiers.command && i.key_pressed(Key::V));
    let was = ui.data(|d| d.get_temp::<bool>(key)).unwrap_or(false);
    ui.data_mut(|d| d.insert_temp(key, down));
    down && !was
}
#[cfg(windows)]
fn chord_down() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL};
    // Reads the current physical key state only; no pointers are involved.
    unsafe {
        (GetAsyncKeyState(i32::from(VK_CONTROL)) as u16 & 0x8000) != 0
            && (GetAsyncKeyState(0x56) as u16 & 0x8000) != 0
    }
}
#[cfg(not(windows))]
fn chord_down() -> bool {
    false
}
/// Inline size inputs for each picture of a tab, keyed by placeholder.
#[derive(Clone, Copy)]
struct PictureSize {
    key: char,
    width: f32,
    ratio: f32,
    fit: bool,
}
impl PictureSize {
    fn at(&self, wrap: f32) -> Vec2 {
        let room = (wrap - 4.).max(16.);
        let w = if self.fit { room } else { self.width.min(room) };
        // Quantized so continuous window resizing reuses cached font sizes.
        let w = ((w / 4.).floor() * 4.).max(8.);
        Vec2::new(w, (w * self.ratio).max(4.))
    }
}
fn picture_sizes(tab: &Tab, zoom: f32) -> Vec<PictureSize> {
    tab.blocks
        .iter()
        .filter_map(|b| match b {
            Block::Picture(p) if p.width > 0 && p.height > 0 => Some(PictureSize {
                key: placeholder(p.id),
                width: p.width as f32 * p.scale * zoom,
                ratio: p.height as f32 / p.width as f32,
                fit: p.fit,
            }),
            _ => None,
        })
        .collect()
}
/// Replace each picture placeholder with an invisible full-width space whose
/// advance and row height equal the picture, so text flows around it.
fn inline_pictures(job: &mut LayoutJob, pictures: &[PictureSize], wrap: f32, ratio: f32) {
    if pictures.is_empty() {
        return;
    }
    let mut changed = false;
    for section in &mut job.sections {
        let Some(c) = job.text[section.byte_range.clone()].chars().next() else {
            continue;
        };
        let Some(size) = pictures.iter().find(|p| p.key == c) else {
            continue;
        };
        let s = size.at(wrap);
        section.format.font_id = FontId::new(s.x / ratio, section.format.font_id.family.clone());
        section.format.line_height = Some(s.y + 4.);
        section.format.color = Color32::TRANSPARENT;
        section.format.underline = Stroke::NONE;
        section.format.strikethrough = Stroke::NONE;
        section.format.italics = false;
        section.format.valign = egui::Align::BOTTOM;
        changed = true;
    }
    if changed {
        // Placeholders and U+3000 are both three UTF-8 bytes, so every
        // section's byte range stays valid after the substitution.
        job.text = job
            .text
            .chars()
            .map(|c| {
                if pictures.iter().any(|p| p.key == c) {
                    '\u{3000}'
                } else {
                    c
                }
            })
            .collect();
    }
}
/// Where the picture whose placeholder is character `idx` is drawn: on the
/// bottom of its own row. A soft-wrapped picture starts a new row; without
/// `prefer_next_row` its position would be the end of the row above, which
/// drew the picture over the preceding text.
fn picture_rect(galley: &egui::Galley, idx: usize, s: Vec2) -> egui::Rect {
    let caret = galley.pos_from_ccursor(CCursor {
        index: idx,
        prefer_next_row: true,
    });
    egui::Rect::from_min_size(egui::pos2(caret.min.x, caret.max.y - s.y - 2.), s)
}
fn paint_pictures(
    ui: &mut egui::Ui,
    doc: &mut Document,
    id: Id,
    output: &egui::text_edit::TextEditOutput,
    pictures: &[PictureSize],
) {
    if pictures.is_empty() {
        return;
    }
    let wrap = output.galley.job.wrap.max_width;
    let origin = output.galley_pos.to_vec2();
    let body: Vec<char> = doc.tabs[doc.active].body.chars().collect();
    let painter = ui.painter().with_clip_rect(output.text_clip_rect);
    let mut clicked_any = false;
    for (idx, c) in body.iter().enumerate() {
        let Some(size) = pictures.iter().find(|p| p.key == *c) else {
            continue;
        };
        let pid = picture_id(*c).unwrap_or(0);
        let rect = picture_rect(&output.galley, idx, size.at(wrap)).translate(origin);
        let Some(Block::Picture(p)) = doc.tabs[doc.active]
            .blocks
            .iter_mut()
            .find(|b| matches!(b, Block::Picture(p) if p.id == pid))
        else {
            continue;
        };
        // Pictures are read from disk only once they scroll into view.
        if p.texture.is_none() && ui.is_rect_visible(rect) {
            if let Some(image) = p.color_image() {
                p.texture = Some(
                    ui.ctx().load_texture(
                        id.with(("inline_picture", doc.active, pid))
                            .short_debug_format(),
                        image,
                        egui::TextureOptions::LINEAR,
                    ),
                );
            }
        }
        if let Some(t) = &p.texture {
            p.texture_used = ui.input(|i| i.time);
            painter.image(
                t.id(),
                rect,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1., 1.)),
                Color32::WHITE,
            );
        }
        if !output.text_clip_rect.intersects(rect) {
            continue;
        }
        let r = ui
            .interact(
                rect.intersect(output.text_clip_rect),
                id.with(("picture_hit", doc.active, pid, idx)),
                egui::Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if r.clicked() {
            doc.selected_picture = Some(pid);
            clicked_any = true;
        }
        if doc.selected_picture != Some(pid) {
            continue;
        }
        painter.rect_stroke(
            rect.expand(1.),
            2.,
            Stroke::new(2.0_f32, crate::ui::ACCENT),
            egui::StrokeKind::Outside,
        );
        let mut close = false;
        let mut delete = false;
        egui::Area::new(id.with(("picture_tools", pid)))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.left_top() - Vec2::new(0., 34.))
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .inner_margin(egui::Margin::same(4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.;
                            for (value, label) in
                                [(0.25, "25%"), (0.5, "50%"), (0.75, "75%"), (1., "100%")]
                            {
                                if ui
                                    .selectable_label(
                                        !p.fit && (p.scale - value).abs() < 0.01,
                                        label,
                                    )
                                    .clicked()
                                {
                                    p.scale = value;
                                    p.fit = false;
                                }
                            }
                            if ui.selectable_label(p.fit, "너비 맞춤").clicked() {
                                p.fit = true;
                            }
                            ui.separator();
                            if ui.button("복사").clicked() {
                                if let Ok(mut c) = arboard::Clipboard::new() {
                                    let _ = c.set_image(arboard::ImageData {
                                        width: p.width,
                                        height: p.height,
                                        bytes: p.rgba(),
                                    });
                                }
                                close = true;
                            }
                            if ui.button("삭제").clicked() {
                                delete = true;
                                close = true;
                            }
                        });
                    });
            });
        if delete {
            let tab = &mut doc.tabs[doc.active];
            let key = placeholder(pid);
            tab.body = tab.body.chars().filter(|c| *c != key).collect();
        }
        if close {
            doc.selected_picture = None;
            return;
        }
    }
    if !clicked_any
        && doc.selected_picture.is_some()
        && ui.input(|i| i.pointer.any_pressed())
        && output.response.hovered()
    {
        doc.selected_picture = None;
    }
}
fn show_blocks(ui: &mut egui::Ui, doc: &mut Document, id: Id) {
    let font = font_id(ui, doc.font_size * doc.zoom, &doc.font_name, false);
    let bold_font = font_id(ui, doc.font_size * doc.zoom, &doc.font_name, true);
    let mut remove = None;
    let mut selected = doc.selected_cell;
    for (bi, block) in doc.tabs[doc.active].blocks.iter_mut().enumerate() {
        match block {
            Block::Table(t) => {
                ui.add_space(8.);
                let mut action = None;
                egui::ScrollArea::horizontal()
                    .id_salt(id.with(("table_scroll", bi)))
                    .show(ui, |ui| {
                        egui::Grid::new(id.with(("table", bi)))
                            .spacing([1., 1.])
                            .show(ui, |ui| {
                                for (r, row) in t.cells.iter_mut().enumerate() {
                                    for (c, cell) in row.iter_mut().enumerate() {
                                        let cf = cell.format.clone();
                                        let col = ui.visuals().text_color();
                                        let f = font.clone();
                                        let mut lay = |ui: &egui::Ui, txt: &str, w: f32| {
                                            let mut j = layout(
                                                txt,
                                                &[],
                                                &cf,
                                                f.clone(),
                                                bold_font.clone(),
                                                col,
                                            );
                                            j.wrap.max_width = w;
                                            ui.fonts(|f| f.layout_job(j))
                                        };
                                        let out = ui.add_sized(
                                            [t.width * doc.zoom, t.row_height * doc.zoom],
                                            egui::TextEdit::multiline(&mut cell.text)
                                                .font(font.clone())
                                                .margin(Vec2::splat(6.))
                                                .layouter(&mut lay),
                                        );
                                        if out.has_focus() {
                                            selected = Some((bi, r, c));
                                        }
                                        out.context_menu(|ui| {
                                            for (k, label) in [
                                                (0, "위에 행 추가"),
                                                (1, "아래에 행 추가"),
                                                (2, "왼쪽에 열 추가"),
                                                (3, "오른쪽에 열 추가"),
                                                (4, "행 삭제"),
                                                (5, "열 삭제"),
                                                (6, "열 너비 일정하게"),
                                                (7, "행 높이 일정하게"),
                                            ] {
                                                if ui.button(label).clicked() {
                                                    action = Some((k, r, c));
                                                    ui.close_menu();
                                                }
                                            }
                                        });
                                    }
                                    ui.end_row();
                                }
                            });
                    });
                ui.horizontal(|ui| {
                    if ui.small_button("+ 행").clicked() {
                        action = Some((1, t.cells.len() - 1, 0));
                    }
                    if ui.small_button("+ 열").clicked() {
                        action = Some((3, 0, t.cells[0].len() - 1));
                    }
                    if ui.small_button("표 복사").clicked() {
                        ui.ctx().copy_text(t.tsv());
                    }
                    if ui.small_button("표 삭제").clicked() {
                        remove = Some(bi);
                    }
                });
                if let Some((k, r, c)) = action {
                    table_action(t, k, r, c);
                }
            }
            Block::Picture(_) => {}
        }
    }
    doc.selected_cell = selected;
    if let Some(i) = remove {
        doc.tabs[doc.active].blocks.remove(i);
        doc.selected_cell = None;
    }
}
fn table_action(t: &mut Table, kind: usize, row: usize, col: usize) {
    let empty = || Cell {
        text: String::new(),
        format: Format::default(),
    };
    match kind {
        0 | 1 if t.cells.len() < 100 => t.cells.insert(
            (row + usize::from(kind == 1)).min(t.cells.len()),
            vec![empty(); t.cells[0].len()],
        ),
        2 | 3 if t.cells[0].len() < 20 => {
            for r in &mut t.cells {
                r.insert((col + usize::from(kind == 3)).min(r.len()), empty());
            }
        }
        4 if t.cells.len() > 1 => {
            t.cells.remove(row.min(t.cells.len() - 1));
        }
        5 if t.cells[0].len() > 1 => {
            for r in &mut t.cells {
                r.remove(col.min(r.len() - 1));
            }
        }
        6 => t.width = 120.,
        7 => t.row_height = 32.,
        _ => {}
    }
}
fn picture_from_file(path: &Path, scale: f32, fit: bool) -> Result<Picture, String> {
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if meta.len() > 64 * 1024 * 1024 {
        return Err("64 MB 이하 이미지를 선택하세요".into());
    }
    let mut reader = image::ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let img = reader
        .decode()
        .map_err(|_| "PNG/JPEG 이미지를 읽을 수 없습니다")?
        .to_rgba8();
    Ok(Picture {
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        width: img.width() as usize,
        height: img.height() as usize,
        pixels: img.into_raw(),
        scale,
        fit,
        file: String::new(),
        texture: None,
        texture_used: 0.,
        id: 0,
    })
}
fn picture_from_clipboard(scale: f32, fit: bool) -> Result<Picture, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    let image = clipboard
        .get_image()
        .map_err(|_| "클립보드에 이미지가 없습니다".to_string())?;
    if image.width > 8192 || image.height > 8192 || image.bytes.len() > 256 * 1024 * 1024 {
        return Err("8192×8192 이하 이미지를 사용하세요".into());
    }
    Ok(Picture {
        name: "붙여 넣은 이미지".into(),
        width: image.width,
        height: image.height,
        pixels: image.bytes.into_owned(),
        scale,
        fit,
        file: String::new(),
        texture: None,
        texture_used: 0.,
        id: 0,
    })
}
pub fn is_picture(c: char) -> bool {
    picture_id(c).is_some()
}
pub fn strip_pictures(text: &str) -> String {
    text.chars().filter(|c| picture_id(*c).is_none()).collect()
}
fn char_byte(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map_or(text.len(), |(b, _)| b)
}
fn replace_chars(text: &mut String, range: Range<usize>, replacement: &str) {
    let a = char_byte(text, range.start);
    let b = char_byte(text, range.end);
    text.replace_range(a..b, replacement);
}
fn short(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        format!("{}…", s.chars().take(n).collect::<String>())
    } else {
        s.into()
    }
}
fn transform_spans(before: &str, after: &str, spans: &mut Vec<Span>, typing: &Format) {
    let a: Vec<_> = before.chars().collect();
    let b: Vec<_> = after.chars().collect();
    let head = a.iter().zip(&b).take_while(|(a, b)| a == b).count();
    let tail = a[head..]
        .iter()
        .rev()
        .zip(b[head..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let old_end = a.len() - tail;
    let new_end = b.len() - tail;
    let delta = new_end as isize - old_end as isize;
    for s in spans.iter_mut() {
        s.range.start = if s.range.start >= old_end {
            (s.range.start as isize + delta).max(0) as usize
        } else {
            s.range.start.min(head)
        };
        s.range.end = if s.range.end >= old_end {
            (s.range.end as isize + delta).max(0) as usize
        } else {
            s.range.end.min(head)
        };
    }
    spans.retain(|s| s.range.start < s.range.end && s.range.end <= b.len());
    if new_end > head
        && !spans
            .last()
            .is_some_and(|s| s.format == *typing && s.range.start <= head && s.range.end >= new_end)
    {
        spans.push(Span {
            range: head..new_end,
            format: typing.clone(),
        });
    }
}
fn urls(text: &str) -> Vec<String> {
    let mut out = vec![];
    for t in text.split_whitespace() {
        let t = t.trim_matches(|c: char| matches!(c, ')' | ']' | '>' | ',' | ';' | '"' | '\''));
        if (t.starts_with("http://") || t.starts_with("https://")) && !out.iter().any(|s| s == t) {
            out.push(t.to_owned());
        }
    }
    out
}
#[derive(Clone, Debug, PartialEq)]
struct Marker {
    family: String,
    value: String,
    indent: usize,
}
fn marker(line: &str) -> Option<Marker> {
    let indent = line.chars().take_while(|c| *c == ' ').count();
    let token = line.split_whitespace().next()?;
    if matches!(token, "-" | "*") {
        return Some(Marker {
            family: token.into(),
            value: token.into(),
            indent,
        });
    }
    let (left, inner, right) = if token.starts_with('(') && token.ends_with(')') {
        ("(", &token[1..token.len() - 1], ")")
    } else if let Some(v) = token.strip_suffix(')') {
        ("", v, ")")
    } else {
        ("", token.strip_suffix('.')?, ".")
    };
    let kind = if inner.chars().all(|c| c.is_ascii_digit()) {
        "number"
    } else if inner
        .chars()
        .all(|c| matches!(c, 'i' | 'v' | 'x' | 'l' | 'c' | 'd' | 'm'))
        && (inner.starts_with('i') || inner.len() > 1)
    {
        "roman"
    } else if inner
        .chars()
        .all(|c| matches!(c, 'I' | 'V' | 'X' | 'L' | 'C' | 'D' | 'M'))
        && (inner.starts_with('I') || inner.len() > 1)
    {
        "ROMAN"
    } else if inner.len() == 1 && inner.chars().all(|c| c.is_ascii_lowercase()) {
        "alpha"
    } else if inner.len() == 1 && inner.chars().all(|c| c.is_ascii_uppercase()) {
        "ALPHA"
    } else if inner.chars().count() == 1 && "ㄱㄴㄷㄹㅁㅂㅅㅇㅈㅊㅋㅌㅍㅎ".contains(inner)
    {
        "korean"
    } else if inner.chars().count() == 1 && "가나다라마바사아자차카타파하".contains(inner)
    {
        "hangul"
    } else {
        return None;
    };
    Some(Marker {
        family: format!("{left}{kind}{right}"),
        value: token.into(),
        indent,
    })
}
fn continuation(text: &str, cursor: usize) -> String {
    let prefix = &text[..char_byte(text, cursor)];
    let line = prefix.rsplit('\n').next().unwrap_or("");
    if marker(line).is_none() {
        return "\n".into();
    }
    let Some(m) = contextual_markers(prefix).last().cloned() else {
        return "\n".into();
    };
    if line.trim() == m.value {
        return "\n".into();
    }
    format!(
        "\n{}{} ",
        " ".repeat(m.indent),
        next_marker(&m.value, &m.family)
    )
}
// Single-letter i/v/x can be either alphabetic or Roman. Keep the established
// sequence at this indentation level; a fresh i/I starts a Roman sequence.
fn contextual_markers(text: &str) -> Vec<Marker> {
    let mut out: Vec<Marker> = vec![];
    for line in text.lines() {
        let Some(mut m) = marker(line) else {
            continue;
        };
        let inner = m.value.trim_matches(['(', ')', '.']);
        if inner.len() == 1 && inner.chars().all(|c| "ivxlcdmIVXLCDM".contains(c)) {
            let upper = inner.chars().next().is_some_and(char::is_uppercase);
            let shape = (m.value.starts_with('('), m.value.ends_with('.'));
            if let Some(previous) = out.iter().rev().find(|p| {
                p.indent == m.indent && (p.value.starts_with('('), p.value.ends_with('.')) == shape
            }) {
                let roman = previous.family.contains("roman") || previous.family.contains("ROMAN");
                let alpha_next = next_marker(&previous.value, &previous.family) == m.value
                    && (previous.family.contains("alpha") || previous.family.contains("ALPHA"));
                let kind = if roman {
                    if upper {
                        "ROMAN"
                    } else {
                        "roman"
                    }
                } else if alpha_next {
                    if upper {
                        "ALPHA"
                    } else {
                        "alpha"
                    }
                } else if inner.eq_ignore_ascii_case("i") {
                    if upper {
                        "ROMAN"
                    } else {
                        "roman"
                    }
                } else {
                    if upper {
                        "ALPHA"
                    } else {
                        "alpha"
                    }
                };
                m.family = format!(
                    "{}{}{}",
                    if shape.0 { "(" } else { "" },
                    kind,
                    if shape.1 { "." } else { ")" }
                );
            }
        }
        out.push(m);
    }
    out
}
fn next_marker(value: &str, family: &str) -> String {
    if matches!(value, "-" | "*") {
        return value.into();
    }
    let start = usize::from(value.starts_with('('));
    let end = value.len() - 1;
    let inner = &value[start..end];
    let next = if family.contains("number") {
        inner
            .parse::<u32>()
            .ok()
            .and_then(|n| n.checked_add(1))
            .map(|n| n.to_string())
            .unwrap_or_else(|| inner.into())
    } else if family.contains("roman") || family.contains("ROMAN") {
        let upper = inner.chars().next().is_some_and(char::is_uppercase);
        let n = roman_number(inner);
        let s = roman(n + 1);
        if upper {
            s.to_uppercase()
        } else {
            s
        }
    } else if family.contains("alpha") || family.contains("ALPHA") {
        inner
            .chars()
            .next()
            .and_then(|c| char::from_u32(c as u32 + 1))
            .unwrap_or('a')
            .to_string()
    } else {
        let seq = if family.contains("korean") {
            "ㄱㄴㄷㄹㅁㅂㅅㅇㅈㅊㅋㅌㅍㅎ"
        } else {
            "가나다라마바사아자차카타파하"
        };
        let v: Vec<_> = seq.chars().collect();
        let n = v.iter().position(|c| c.to_string() == inner).unwrap_or(0);
        v.get(n + 1).unwrap_or(&v[0]).to_string()
    };
    format!(
        "{}{}{}",
        if start == 1 { "(" } else { "" },
        next,
        &value[end..]
    )
}
fn roman_number(s: &str) -> u32 {
    let mut sum = 0;
    let mut prev = 0;
    for c in s.to_ascii_lowercase().chars().rev() {
        let n = match c {
            'i' => 1,
            'v' => 5,
            'x' => 10,
            'l' => 50,
            'c' => 100,
            'd' => 500,
            'm' => 1000,
            _ => 0,
        };
        if n < prev {
            sum -= n
        } else {
            sum += n;
            prev = n;
        }
    }
    sum
}
fn roman(mut n: u32) -> String {
    let mut out = String::new();
    for (v, t) in [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ] {
        while n >= v {
            out.push_str(t);
            n -= v;
        }
    }
    out
}
fn normalize_outline_line(text: &mut String, cursor: usize) -> Option<usize> {
    let prefix = &text[..char_byte(text, cursor)];
    let byte_start = prefix.rfind('\n').map_or(0, |x| x + 1);
    let line = &prefix[byte_start..];
    let m = contextual_markers(prefix).last()?.clone();
    if marker(line).is_none() || line.trim() != m.value || !line.ends_with(' ') {
        return None;
    }
    let ancestors = &text[..byte_start];
    let prev = contextual_markers(ancestors);
    let indent = prev
        .iter()
        .rev()
        .find(|p| p.family == m.family)
        .map(|p| p.indent)
        .unwrap_or_else(|| prev.last().map_or(2, |p| p.indent + 2));
    if indent == m.indent {
        return None;
    }
    let replacement = format!("{}{} ", " ".repeat(indent), m.value);
    let line_start = text[..byte_start].chars().count();
    replace_chars(text, line_start..cursor, &replacement);
    Some(line_start + replacement.chars().count())
}
fn outdent_line(text: &mut String, cursor: usize) -> usize {
    let prefix = &text[..char_byte(text, cursor)];
    let start = prefix.rfind('\n').map_or(0, |b| b + 1);
    let n = text[start..]
        .chars()
        .take_while(|c| *c == ' ')
        .count()
        .min(2);
    text.replace_range(start..start + n, "");
    cursor.saturating_sub(n)
}
fn shortcut_substitution(
    text: &str,
    cursor: usize,
) -> Option<(Range<usize>, String, Option<Range<usize>>)> {
    let prefix = &text[..char_byte(text, cursor)];
    if prefix.ends_with("[] ") {
        return Some((cursor - 3..cursor, "☐ ".into(), None));
    }
    if prefix.ends_with("~~~ ") {
        let end = cursor - 4;
        let start = prefix[..char_byte(prefix, end)]
            .rfind('\n')
            .map_or(0, |b| prefix[..b + 1].chars().count());
        return Some((end..cursor, String::new(), Some(start..end)));
    }
    if prefix.ends_with("~~ ") {
        let end = cursor - 3;
        let chars: Vec<_> = text.chars().collect();
        let mut start = end;
        while start > 0 && !chars[start - 1].is_whitespace() {
            start -= 1;
        }
        return Some((end..cursor, " ".into(), Some(start..end)));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tabs_keep_bodies_and_deleted_tab_content() {
        let mut d = Document::default();
        d.tabs[0].body = "처음".into();
        d.add_tab();
        d.tabs[1].body = "둘째".into();
        d.delete_tab(1);
        assert_eq!(d.tabs[0].body, "처음");
        assert_eq!(d.trash[0].body, "둘째");
        assert_eq!(d.active, 0);
    }
    #[test]
    fn html_table_keeps_multiline_cells_and_pads_merged_columns() {
        let html = "<table>\
            <tr><td colspan=\"2\">제목</td></tr>\
            <tr><td>첫 줄<br>둘째 줄</td><td>가\t나</td></tr>\
            </table>";
        let t = Table::from_html(html).expect("table parses");
        assert_eq!(t.cells.len(), 2);
        assert_eq!(t.cells[0].len(), 2, "colspan=2 pads a blank second cell");
        assert_eq!(t.cells[0][0].text, "제목");
        assert_eq!(t.cells[0][1].text, "");
        assert_eq!(
            t.cells[1][0].text, "첫 줄\n둘째 줄",
            "<br> becomes a newline instead of running the lines together"
        );
        assert_eq!(
            t.cells[1][1].text, "가\t나",
            "a tab inside a cell's own text is not mistaken for a column boundary"
        );
    }
    #[test]
    fn non_table_html_falls_back_to_none() {
        assert!(Table::from_html("<p>표가 아닙니다</p>").is_none());
    }
    #[test]
    fn indentation_continues_same_level_and_returns_to_known_family() {
        assert_eq!(continuation("  1) 한글", 7), "\n  2) ");
        let mut t = "  1. 부모\n    a) 자식\n      2. ".to_string();
        let n = t.chars().count();
        normalize_outline_line(&mut t, n);
        assert!(t.ends_with("\n  2. "));
        let n = t.chars().count();
        outdent_line(&mut t, n);
        assert!(t.ends_with("\n2. "));
    }
    #[test]
    fn roman_and_alphabet_families_are_deterministic() {
        assert_eq!(next_marker("i)", "roman)"), "ii)");
        assert_eq!(next_marker("(iv)", "(roman)"), "(v)");
        assert_eq!(next_marker("h)", "alpha)"), "i)");
        assert_ne!(
            marker("i) x").unwrap().family,
            marker("h) x").unwrap().family
        );
        let roman = "  i) 하나\n  ii) 둘\n  iii) 셋\n  iv) 넷\n  v) 다섯";
        assert_eq!(continuation(roman, roman.chars().count()), "\n  vi) ");
        let alphabet = "  h) 여덟\n  i) 아홉";
        assert_eq!(continuation(alphabet, alphabet.chars().count()), "\n  j) ");
        assert_eq!(continuation("  i) 항목\n일반 문장", 17), "\n");
    }
    #[test]
    fn unicode_shortcuts_do_not_split_korean_chars() {
        let s = "한글단어~~ ";
        let n = s.chars().count();
        let (r, rep, style) = shortcut_substitution(s, n).unwrap();
        let mut t = s.to_owned();
        replace_chars(&mut t, r, &rep);
        assert_eq!(t, "한글단어 ");
        assert_eq!(style, Some(0..4));
        let mut s = "안녕[] ".to_string();
        let (r, rep, _) = shortcut_substitution(&s, s.chars().count()).unwrap();
        replace_chars(&mut s, r, &rep);
        assert_eq!(s, "안녕☐ ");
    }
    #[test]
    fn table_edits_keep_rectangular_shape() {
        let mut t = Table::from_tsv("가\t나\n다\t라");
        table_action(&mut t, 3, 0, 0);
        table_action(&mut t, 1, 0, 0);
        assert_eq!(t.cells.len(), 3);
        assert!(t.cells.iter().all(|r| r.len() == 3));
        table_action(&mut t, 5, 0, 1);
        table_action(&mut t, 4, 1, 0);
        assert_eq!(t.tsv(), "가\t나\n다\t라");
    }
    #[test]
    fn spans_follow_unicode_insertions() {
        let mut s = vec![Span {
            range: 2..4,
            format: Format {
                strike: true,
                ..Default::default()
            },
        }];
        transform_spans("가나다라", "한글가나다라", &mut s, &Format::default());
        assert_eq!(s[0].range, 4..6);
    }
    #[test]
    fn deleting_the_first_or_last_tab_never_revives_its_content() {
        let mut d = Document::default();
        d.tabs[0].body = "첫 탭".into();
        d.add_tab();
        d.tabs[1].body = "남은 탭".into();
        d.delete_tab(0);
        assert_eq!(d.tabs[0].body, "남은 탭");
        assert_eq!(d.trash[0].body, "첫 탭");
        let legacy = d.tabs[0].body.clone();
        assert_eq!(legacy, "남은 탭");
        d.delete_tab(0);
        assert_eq!(d.tabs.len(), 1);
        assert!(d.tabs[0].body.is_empty());
        assert_eq!(d.trash[1].body, "남은 탭");
    }
    #[test]
    fn formatting_selection_does_not_change_unselected_text() {
        let mut d = Document::default();
        d.tabs[0].body = "처음 선택 나중".into();
        d.selection = 3..5;
        d.toggle_format(2);
        assert!(d.tabs[0].spans[0].format.underline);
        assert_eq!(d.tabs[0].spans[0].range, 3..5);
        let font = FontId::proportional(14.);
        let job = layout(
            &d.tabs[0].body,
            &d.tabs[0].spans,
            &Format::default(),
            font.clone(),
            font,
            Color32::BLACK,
        );
        assert_eq!(job.sections[0].format.underline, Stroke::NONE);
        assert_ne!(job.sections[3].format.underline, Stroke::NONE);
        assert_eq!(job.sections[6].format.underline, Stroke::NONE);
    }
    #[test]
    fn compact_sticker_keeps_tabs_below_two_tool_rows_inside_viewport() {
        let ctx = egui::Context::default();
        let mut doc = Document::default();
        let mut text = "짧은 메모".to_string();
        let mut bottom = 0.;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::splat(360.),
            )),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(14.))
                .show(ctx, |ui| {
                    ui.add_space(96.);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        show(ui, &mut text, &mut doc, "sticker_test");
                        bottom = ui.cursor().top();
                    });
                });
        });
        assert!(
            bottom < 350.,
            "footer bottom {bottom} exceeds the compact sticker"
        );
    }
    #[test]
    fn search_selection_targets_the_requested_tab_without_replacing_first_body() {
        let ctx = egui::Context::default();
        let mut doc = Document::default();
        let mut body = "첫 탭 원문".to_owned();
        doc.add_tab();
        doc.tabs[1].body = "둘째 탭의 검색어".into();
        doc.active = 0;
        doc.requested_tab = Some(1);
        doc.requested_selection = Some(2..5);
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(ui, &mut body, &mut doc, "requested_tab_test");
            });
        });
        assert_eq!(doc.active, 1);
        assert_eq!(doc.selection, 2..5);
        assert_eq!(body, "첫 탭 원문");
        assert!(doc.requested_tab.is_none());
        assert!(doc.requested_selection.is_none());
    }
    #[test]
    fn saved_rich_view_keeps_the_comment_and_formats_immutable() {
        let ctx = egui::Context::default();
        let mut doc = Document::default();
        doc.tabs[0].body = "읽기 전용 댓글".into();
        doc.tabs[0].spans.push(Span {
            range: 0..2,
            format: Format {
                underline: true,
                ..Default::default()
            },
        });
        doc.tabs[0]
            .blocks
            .push(Block::Table(Table::from_tsv("가\t나\n다\t라")));
        let before = doc.tabs[0].body.clone();
        let spans = doc.tabs[0].spans.len();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                view(ui, &before, &doc, "comment_view_test");
            });
        });
        assert_eq!(doc.tabs[0].body, before);
        assert_eq!(doc.tabs[0].spans.len(), spans);
        assert!(doc.tabs[0].spans[0].format.underline);
        assert_eq!(doc.active, 0);
        assert!(matches!(&doc.tabs[0].blocks[0],Block::Table(t) if t.tsv()=="가\t나\n다\t라"));
    }
    #[test]
    fn icon_toolbar_fits_one_row_and_wraps_without_visible_text_controls() {
        for (width, max_height) in [(360., 32.), (200., 64.)] {
            let ctx = egui::Context::default();
            let mut doc = Document::default();
            let mut height = 0.;
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(width, 360.),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::NONE.inner_margin(14.))
                        .show(ctx, |ui| {
                            height = toolbar(ui, &mut doc, Id::new("icon_toolbar_test"), true)
                                .rect
                                .height();
                        });
                },
            );
            assert!(
                height <= max_height,
                "toolbar at width {width} used {height} px"
            );
            // Only the font size number is drawn as text, as the design asks.
            assert!(
                !output.shapes.iter().any(|s| matches!(&s.shape,
                    egui::Shape::Text(t) if !t.galley.job.text.chars().all(|c| c.is_ascii_digit()))),
                "closed toolbar contains a text control"
            );
            assert_eq!(
                toolbar_height(width - 28., TOOLS.len()),
                if width == 360. { 28. } else { 58. }
            );
        }
    }
    #[test]
    fn split_footer_stays_at_panel_bottom_without_consuming_body_scroll_height() {
        let ctx = egui::Context::default();
        let mut doc = Document::default();
        let mut text = "본문\n".repeat(40);
        let mut footer_rect = egui::Rect::NOTHING;
        let mut body_rect = egui::Rect::NOTHING;
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::splat(360.),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::TopBottomPanel::bottom("separate_footer")
                    .default_height(80.)
                    .frame(egui::Frame::NONE.inner_margin(8.))
                    .show(ctx, |ui| {
                        footer_rect = footer(ui, &mut doc, "split_memo", 32., false).rect;
                    });
                egui::CentralPanel::default().show(ctx, |ui| {
                    body_rect = ui.available_rect_before_wrap();
                    content(ui, &mut text, &mut doc, "split_memo");
                });
            },
        );
        assert!(
            footer_rect.bottom() >= 345. && footer_rect.bottom() <= 360.,
            "footer {footer_rect:?}, body {body_rect:?}"
        );
        assert!(
            footer_rect.height() < 100.,
            "compact footer became taller than expected: {footer_rect:?}"
        );
        assert!(
            body_rect.bottom() <= footer_rect.top(),
            "body and pinned footer overlap"
        );
        assert_eq!(text.lines().count(), 40);
    }
    #[test]
    fn footer_first_tab_delete_cannot_revive_legacy_content_in_separate_body() {
        let ctx = egui::Context::default();
        let mut doc = Document::default();
        let mut legacy = "지워진 첫 본문".to_owned();
        doc.tabs[0].body = legacy.clone();
        doc.add_tab();
        doc.tabs[1].body = "살아 있는 본문".into();
        doc.delete_tab(0);
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::TopBottomPanel::bottom("delete_footer").show(ctx, |ui| {
                footer(ui, &mut doc, "delete_memo", 0., false);
            });
            egui::CentralPanel::default().show(ctx, |ui| {
                content(ui, &mut legacy, &mut doc, "delete_memo");
            });
        });
        assert_eq!(legacy, "살아 있는 본문");
        assert_eq!(doc.tabs[0].body, "살아 있는 본문");
        assert_eq!(doc.trash[0].body, "지워진 첫 본문");
    }
    #[test]
    fn wrapped_tools_stay_inside_a_bottom_up_inline_preview() {
        for width in [220., 260., 300.] {
            let ctx = egui::Context::default();
            let mut doc = Document::default();
            let mut bounds = egui::Rect::NOTHING;
            let mut tools = egui::Rect::NOTHING;
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(width, 240.),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::NONE.inner_margin(20.))
                        .show(ctx, |ui| {
                            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                                bounds = ui.max_rect();
                                tools = toolbar(ui, &mut doc, Id::new("inline_tools_bounds"), true)
                                    .rect;
                            });
                        });
                },
            );
            assert!(
                bounds.expand(0.5).contains_rect(tools),
                "width={width}: tools={tools:?}, bounds={bounds:?}"
            );
        }
    }
    #[test]
    fn status_line_footer_keeps_tools_tabs_and_body_inside_active_and_passive_bounds() {
        let mut errors = vec![];
        for (active, width, base_height) in
            [(true, 360., 92.), (true, 320., 122.), (false, 360., 48.)]
        {
            let ctx = egui::Context::default();
            if let Ok(bytes) = std::fs::read("C:/Windows/Fonts/malgun.ttf") {
                let mut fonts = egui::FontDefinitions::default();
                fonts.font_data.insert(
                    "actual_windows_malgun".into(),
                    egui::FontData::from_owned(bytes).into(),
                );
                fonts
                    .families
                    .get_mut(&FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "actual_windows_malgun".into());
                ctx.set_fonts(fonts);
            }
            ctx.style_mut(|s| {
                s.spacing.item_spacing = Vec2::splat(8.);
                s.spacing.button_padding = Vec2::new(9., 6.);
                s.text_styles
                    .insert(egui::TextStyle::Button, FontId::proportional(15.));
                s.text_styles
                    .insert(egui::TextStyle::Small, FontId::proportional(12.));
            });
            let mut doc = Document {
                toolbar_visible: active,
                status: "이미지 처리 안내가 긴 한 줄로 표시됩니다".into(),
                ..Default::default()
            };
            let mut legacy = "원본 본문\n".repeat(30);
            let original = legacy.clone();
            let mut bounds = egui::Rect::NOTHING;
            let mut footer_rect = egui::Rect::NOTHING;
            let mut body = egui::Rect::NOTHING;
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(width, 360.),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    let small_font = egui::TextStyle::Small.resolve(&ctx.style());
                    let status_height = ctx.fonts(|f| f.row_height(&small_font)) + 4.;
                    egui::TopBottomPanel::bottom("status_footer")
                        .exact_height(base_height + status_height)
                        .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(12, 6)))
                        .show(ctx, |ui| {
                            bounds = ui.max_rect().intersect(ui.clip_rect());
                            footer_rect = footer(
                                ui,
                                &mut doc,
                                "status_document",
                                if active { 32. } else { 0. },
                                false,
                            )
                            .rect;
                        });
                    egui::CentralPanel::default().show(ctx, |ui| {
                        body = ui.available_rect_before_wrap();
                        content(ui, &mut legacy, &mut doc, "status_document");
                    });
                },
            );
            if !bounds.expand(0.5).contains_rect(footer_rect) {
                errors.push(format!(
                    "active={active} width={width}: footer={footer_rect:?}, bounds={bounds:?}"
                ));
            }
            if body.bottom() > footer_rect.top() {
                errors.push(format!(
                    "active={active} width={width}: body overlaps footer"
                ));
            }
            assert_eq!(legacy, original);
        }
        assert!(errors.is_empty(), "{}", errors.join("\n"));
    }
}
#[cfg(test)]
mod inline_picture_tests {
    use super::*;
    fn picture(w: usize, h: usize) -> Picture {
        Picture {
            id: 0,
            name: "test".into(),
            pixels: vec![255; w * h * 4],
            width: w,
            height: h,
            scale: 1.,
            fit: false,
            file: String::new(),
            texture: None,
            texture_used: 0.,
        }
    }
    #[test]
    fn pasted_picture_sits_in_the_text_and_follows_deletion() {
        let mut tab = Tab::new("본문".into());
        tab.body = "앞뒤".into();
        let caret = tab.insert_picture(1..1, picture(40, 20));
        assert_eq!(caret, 2);
        assert_eq!(tab.body.chars().count(), 3);
        assert!(is_picture(tab.body.chars().nth(1).unwrap()));
        assert!(tab.shows(&tab.blocks[0]));
        assert_eq!(strip_pictures(&tab.body), "앞뒤");
        let second = tab.insert_picture(3..3, picture(10, 10));
        assert_eq!(second, 4);
        assert_ne!(tab.body.chars().nth(1), tab.body.chars().nth(3));
        // Deleting the placeholder hides the picture from the note and its collections.
        replace_chars(&mut tab.body, 1..2, "");
        assert!(!tab.shows(&tab.blocks[0]));
        assert!(tab.shows(&tab.blocks[1]));
    }
    #[test]
    fn a_wrapped_fit_picture_is_drawn_below_the_text_before_it() {
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |_| {});
        let mut tab = Tab::new("본문".into());
        tab.body = "가나다라마바사아자차카타파하".into();
        let at = tab.body.chars().count();
        let mut p = picture(400, 200);
        p.fit = true;
        tab.insert_picture(at..at, p);
        let font = FontId::proportional(14.);
        let ratio = ctx.fonts(|f| f.glyph_width(&FontId::proportional(100.), '\u{3000}')) / 100.;
        let mut job = layout(
            &tab.body,
            &[],
            &Format::default(),
            font.clone(),
            font,
            Color32::BLACK,
        );
        let wrap = 200.;
        job.wrap.max_width = wrap;
        let sizes = picture_sizes(&tab, 1.);
        inline_pictures(&mut job, &sizes, wrap, ratio);
        let galley = ctx.fonts(|f| f.layout_job(job));
        assert!(galley.rows.len() >= 2, "the picture wraps onto its own row");
        let text_bottom = galley.rows[0].rect.max.y;
        let rect = picture_rect(&galley, at, sizes[0].at(wrap));
        assert!(
            rect.min.y >= text_bottom - 0.5,
            "{rect:?} overlaps text ending at {text_bottom}"
        );
    }
    #[test]
    fn layout_reserves_the_picture_size_and_keeps_byte_ranges_valid() {
        let mut tab = Tab::new("본문".into());
        tab.body = "가".into();
        tab.insert_picture(1..1, picture(80, 40));
        tab.body.push('나');
        let sizes = picture_sizes(&tab, 1.);
        let font = FontId::proportional(14.);
        let mut job = layout(
            &tab.body,
            &[],
            &Format::default(),
            font.clone(),
            font,
            Color32::BLACK,
        );
        inline_pictures(&mut job, &sizes, 300., 1.);
        assert_eq!(job.text, "가\u{3000}나");
        for s in &job.sections {
            assert!(job.text.get(s.byte_range.clone()).is_some());
        }
        let image = &job.sections[1].format;
        assert_eq!(image.font_id.size, 80.);
        assert_eq!(image.line_height, Some(44.));
        assert_eq!(image.color, Color32::TRANSPARENT);
        // Fit-to-width follows the wrap width.
        if let Block::Picture(p) = &mut tab.blocks[0] {
            p.fit = true;
        }
        let fit = picture_sizes(&tab, 1.)[0].at(201.);
        assert_eq!(fit, Vec2::new(196., 98.));
    }
    #[test]
    fn plain_text_and_title_inference_skip_picture_placeholders() {
        let mut doc = Document::default();
        doc.tabs[0].insert_picture(0..0, picture(4, 4));
        let mut body = doc.tabs[0].body.clone() + "제목 줄\n본문";
        assert_eq!(doc.plain_text(&body), "제목 줄\n본문");
        let mut title = String::new();
        crate::desktop::infer_title(&mut title, &mut body);
        assert_eq!(title, "제목 줄");
        assert_eq!(body.chars().filter(|c| is_picture(*c)).count(), 1);
        assert!(body.ends_with("본문"));
    }
}
