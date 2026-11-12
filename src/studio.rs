use crate::icons::{icon_button, Icon};
use crate::*;
use egui::{Pos2, Rect, Sense};
use std::collections::BTreeMap;

/// The color of the "중요 메모" star shown before titles.
pub const STAR: Color32 = Color32::from_rgb(236, 168, 32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Editor,
    Title,
    Swatch,
    Sticker,
}
pub struct Window {
    pub mode: Mode,
    pub sticker_preferred: bool,
    pub open: bool,
    pub occurrence: Option<NaiveDate>,
    pub schedule_start: Option<NaiveDate>,
    full_size: Vec2,
    title_width: f32,
    skip_title: bool,
    pub sticker_size: Vec2,
    swatch_size: Vec2,
    applied: Option<(Mode, bool)>,
    focus: bool,
    center: bool,
    help: bool,
    native: crate::native_window::State,
    place: Option<Pos2>,
    swatch_drag: Option<(Vec2, Pos2, Vec2)>,
    menu: Option<Menu>,
    /// Where the window was last seen on screen, for reopening it there.
    last_pos: Option<Pos2>,
    /// Set when Enter is pressed in the sticker title; applied at the start
    /// of the *next* frame, never the one Enter was pressed on. The title's
    /// return_key handling does not remove that Enter (or a pending IME
    /// commit) from this frame's input, so requesting focus on the body in
    /// the same frame let its own TextEdit see the same event again and
    /// insert a stray newline (or duplicate the just-composed character).
    move_caret_to_body: bool,
}
/// Desktop arrangement requested from a sticker's context menu.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Layout {
    Right,
    Left,
    Cascade,
}
#[derive(Clone, Copy, Debug)]
pub struct Arrange {
    pub layout: Layout,
    pub fold: Option<Mode>,
    pub area: Rect,
}
impl Window {
    fn new(mode: Mode) -> Self {
        Self {
            mode,
            sticker_preferred: mode == Mode::Sticker,
            open: true,
            occurrence: None,
            schedule_start: None,
            full_size: Vec2::new(620., 720.),
            title_width: 360.,
            skip_title: false,
            sticker_size: Vec2::splat(360.),
            swatch_size: Vec2::splat(48.),
            applied: None,
            focus: true,
            center: true,
            help: false,
            native: Default::default(),
            place: None,
            swatch_drag: None,
            menu: None,
            last_pos: None,
            move_caret_to_body: false,
        }
    }
    /// What is needed to reopen this window after a restart.
    pub fn session(&self, note: usize) -> crate::storage::SessionRec {
        crate::storage::SessionRec {
            note,
            mode: match self.mode {
                Mode::Editor => 0,
                Mode::Title => 1,
                Mode::Swatch => 2,
                Mode::Sticker => 3,
            },
            sticker_preferred: self.sticker_preferred,
            pos: self.last_pos.map(|p| [p.x, p.y]),
            full_size: self.full_size.into(),
            sticker_size: self.sticker_size.into(),
            title_width: self.title_width,
            swatch_size: self.swatch_size.into(),
        }
    }
    /// A window reopened where it was, without taking focus.
    pub fn from_session(r: &crate::storage::SessionRec) -> Self {
        let mode = match r.mode {
            1 => Mode::Title,
            2 => Mode::Swatch,
            3 => Mode::Sticker,
            _ => Mode::Editor,
        };
        let mut w = Self::new(mode);
        w.sticker_preferred = r.sticker_preferred;
        let size =
            |s: [f32; 2], min: Vec2| (s[0] > 0. && s[1] > 0.).then(|| Vec2::from(s).max(min));
        if let Some(s) = size(r.full_size, Vec2::new(480., 480.)) {
            w.full_size = s;
        }
        if let Some(s) = size(r.sticker_size, Vec2::new(340., 220.)) {
            w.sticker_size = s;
        }
        if r.title_width > 0. {
            w.title_width = r.title_width.max(160.);
        }
        if let Some(s) = size(r.swatch_size, Vec2::splat(16.)) {
            w.swatch_size = s;
        }
        w.focus = false;
        if let Some(p) = r.pos {
            w.center = false;
            w.place = Some(Pos2::new(p[0], p[1]));
            w.last_pos = w.place;
        }
        w
    }
    fn reconcile(&mut self, n: &Note) {
        if let (Some(old), Some(new), Some(d)) = (self.schedule_start, n.date, self.occurrence) {
            if old != new {
                self.occurrence = Some(d + (new - old));
            }
        }
        self.schedule_start = n.date;
        if n.duration > 1 {
            if let (Some(d), Some(start)) = (self.occurrence, n.date) {
                let end = start + chrono::Duration::days(n.duration.max(1) as i64 - 1);
                self.occurrence = Some(d.clamp(start, end));
            }
        }
    }
    pub fn fold(&mut self) {
        self.mode = match self.mode {
            Mode::Editor | Mode::Sticker if self.skip_title => Mode::Swatch,
            Mode::Editor => Mode::Title,
            Mode::Sticker => Mode::Title,
            Mode::Title => Mode::Swatch,
            Mode::Swatch => {
                if self.sticker_preferred {
                    Mode::Sticker
                } else {
                    Mode::Editor
                }
            }
        };
    }
    pub fn sticker(&mut self) {
        self.sticker_preferred = true;
        self.mode = Mode::Sticker;
    }
    pub fn edit(&mut self) {
        self.mode = Mode::Editor;
    }
    fn outer_size(&self, locked: bool) -> Vec2 {
        if locked {
            Vec2::splat(64.)
        } else {
            self.size()
        }
    }
    fn size(&self) -> Vec2 {
        match self.mode {
            Mode::Editor => self.full_size,
            Mode::Title => Vec2::new(self.title_width, 44.),
            Mode::Swatch => self.swatch_size,
            Mode::Sticker => self.sticker_size,
        }
    }
}
#[derive(Default)]
pub struct Studio {
    pub windows: BTreeMap<usize, Window>,
    pub help: bool,
}
impl Studio {
    pub fn open(&mut self, i: usize, occurrence: Option<NaiveDate>) {
        let w = self
            .windows
            .entry(i)
            .or_insert_with(|| Window::new(Mode::Editor));
        if !w.open {
            w.applied = None;
            w.native.cancel();
        }
        w.open = true;
        w.mode = Mode::Editor;
        w.occurrence = occurrence;
        w.focus = true;
    }
}
/// Positions for windows of the given sizes, packed in columns from one edge
/// of `area` (or cascaded), keeping a small gap and margin.
pub fn plan(layout: Layout, area: Rect, sizes: &[Vec2]) -> Vec<Pos2> {
    let margin = 14.;
    let gap = 8.;
    if layout == Layout::Cascade {
        return sizes
            .iter()
            .enumerate()
            .map(|(k, size)| {
                let step = 30. * (k % 12) as f32;
                crate::native_window::fit_position(
                    area.min + Vec2::new(80. + step, 60. + step),
                    *size,
                    area,
                )
            })
            .collect();
    }
    let mut out = Vec::with_capacity(sizes.len());
    let mut y = area.top() + margin;
    let mut edge = if layout == Layout::Right {
        area.right() - margin
    } else {
        area.left() + margin
    };
    let mut column = 0f32;
    for size in sizes {
        if y + size.y > area.bottom() - margin && y > area.top() + margin {
            edge += if layout == Layout::Right {
                -(column + gap)
            } else {
                column + gap
            };
            y = area.top() + margin;
            column = 0.;
        }
        let x = if layout == Layout::Right {
            edge - size.x
        } else {
            edge
        };
        out.push(Pos2::new(x, y));
        y += size.y + gap;
        column = column.max(size.x);
    }
    out
}
impl App {
    /// Apply a pending desktop arrangement to every open sticker window.
    pub fn apply_arrangement(&mut self, _ctx: &egui::Context) {
        let Some(a) = self.arrange.take() else {
            return;
        };
        let ids: Vec<_> = self
            .studio
            .windows
            .iter()
            .filter(|(&i, w)| w.open && !self.data.notes[i].deleted && self.visible(i))
            .map(|(&i, _)| i)
            .collect();
        let mut sizes = vec![];
        for &i in &ids {
            let locked = self.data.notes[i].locked;
            let w = self.studio.windows.get_mut(&i).unwrap();
            match a.fold {
                Some(Mode::Title) if !locked => w.mode = Mode::Title,
                Some(Mode::Swatch) => w.mode = Mode::Swatch,
                Some(Mode::Sticker) if !locked => {
                    w.mode = if w.sticker_preferred {
                        Mode::Sticker
                    } else {
                        Mode::Editor
                    }
                }
                _ => {}
            }
            if w.mode == Mode::Swatch && self.desktop.unpin_minimized {
                self.data.notes[i].topmost = false;
            }
            sizes.push(w.outer_size(locked));
        }
        for (i, p) in ids.iter().zip(plan(a.layout, a.area, &sizes)) {
            let w = self.studio.windows.get_mut(i).unwrap();
            w.place = Some(p);
            w.center = false;
        }
        self.toast = format!("스티커 {}개를 정렬했습니다", ids.len());
    }
    /// Arrangement and window actions. Returns true once an item was used.
    /// Where stickers are arranged: the monitor chosen in the settings, or
    /// the one the current window is on.
    pub fn arrange_area(&self, ctx: &egui::Context) -> Rect {
        self.desktop
            .arrange_monitor
            .and_then(|m| crate::native_window::monitors(ctx).get(m).copied())
            .or_else(|| crate::native_window::work_area(ctx))
            .unwrap_or_else(|| ctx.screen_rect())
    }
    /// Once a day from 05:00, open unfinished past memos and today's memos as
    /// titles stacked on the right of the arrange monitor.
    pub fn daily_today(&mut self, ctx: &egui::Context) {
        if !self.desktop.daily_today {
            return;
        }
        let today = daily_day(daily_now());
        if self.desktop.daily_shown == Some(today) {
            return;
        }
        self.desktop.daily_shown = Some(today);
        let due = self.daily_due(today);
        if due.is_empty() {
            return;
        }
        for i in due {
            self.open_sticker(i, Some(today));
        }
        self.arrange = Some(Arrange {
            layout: Layout::Right,
            fold: Some(Mode::Title),
            area: self.arrange_area(ctx),
        });
    }
    /// Uncompleted memos of earlier days plus the memos of `today`.
    fn daily_due(&self, today: NaiveDate) -> Vec<usize> {
        self.data
            .notes
            .iter()
            .enumerate()
            .filter(|(i, n)| {
                self.visible(*i)
                    && !n.locked
                    && n.date.is_some_and(|d| {
                        (d < today && !n.done_on(Some(d)))
                            || n.occurs(today) && !n.done_on(Some(today))
                    })
            })
            .map(|(i, _)| i)
            .collect()
    }
    /// Big numbered windows on every monitor; clicking one makes it the
    /// monitor stickers are arranged on.
    pub fn monitor_picker(&mut self, ctx: &egui::Context) {
        if !self.desktop.monitor_pick {
            return;
        }
        let monitors = crate::native_window::monitors(ctx);
        if monitors.len() < 2 {
            self.desktop.monitor_pick = false;
            return;
        }
        let mut picked = None;
        let mut cancel = false;
        for (k, area) in monitors.iter().enumerate() {
            let size = Vec2::new(360., 300.);
            let builder = egui::ViewportBuilder::default()
                .with_title(format!("모니터 {}", k + 1))
                .with_position(area.center() - size / 2.)
                .with_inner_size(size)
                .with_decorations(false)
                .with_resizable(false)
                .with_taskbar(false)
                .with_transparent(true)
                .with_window_level(egui::WindowLevel::AlwaysOnTop);
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of(("monitor_pick", k)),
                builder,
                |ctx, _| {
                    crate::win_style::shape(ctx, 0.78);
                    egui::CentralPanel::default()
                        .frame(egui::Frame::NONE.fill(Color32::WHITE))
                        .show(ctx, |ui| {
                            let rect = ui.max_rect();
                            let r = ui
                                .interact(rect, ui.id().with("pick"), Sense::click())
                                .on_hover_cursor(egui::CursorIcon::PointingHand);
                            ui.painter().text(
                                rect.center() - Vec2::new(0., 18.),
                                egui::Align2::CENTER_CENTER,
                                (k + 1).to_string(),
                                egui::FontId::proportional(170.),
                                if r.hovered() { ACCENT } else { INK },
                            );
                            ui.painter().text(
                                Pos2::new(rect.center().x, rect.bottom() - 34.),
                                egui::Align2::CENTER_CENTER,
                                "이 모니터에 스티커를 정렬하려면 누르세요",
                                egui::FontId::proportional(14.),
                                MUTED,
                            );
                            if r.clicked() {
                                picked = Some(k);
                            }
                            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                cancel = true;
                            }
                        });
                },
            );
        }
        if let Some(k) = picked {
            self.desktop.arrange_monitor = Some(k);
            self.desktop.monitor_pick = false;
            self.toast = format!("스티커 정렬 모니터를 {}번으로 정했습니다", k + 1);
        } else if cancel {
            self.desktop.monitor_pick = false;
        }
    }
    fn sticker_menu(&mut self, ui: &mut egui::Ui, i: usize, w: &mut Window, own: bool) -> bool {
        let ctx = ui.ctx().clone();
        let area = self.arrange_area(&ctx);
        let locked = self.data.notes[i].locked;
        ui.set_min_width(210.);
        let mut done = false;
        if own && w.mode != Mode::Sticker && w.mode != Mode::Editor {
            if locked {
                if ui.button("잠금 해제…").clicked() {
                    self.ask_unlock(i, true);
                    done = true;
                }
            } else if ui.button("스티커로 펼치기").clicked() {
                w.sticker();
                done = true;
            }
            ui.separator();
        }
        crate::ui::caption(ui, "스티커 정렬");
        let mut pick = None;
        for (label, layout, fold) in [
            ("화면 오른쪽에 정렬", Layout::Right, None),
            ("화면 왼쪽에 정렬", Layout::Left, None),
            ("계단식으로 정렬", Layout::Cascade, None),
        ] {
            if ui.button(label).clicked() {
                pick = Some((layout, fold));
            }
        }
        ui.separator();
        for (label, layout, fold) in [
            (
                "모두 제목만 보이게 · 오른쪽 정렬",
                Layout::Right,
                Some(Mode::Title),
            ),
            (
                "모두 상자로 접기 · 오른쪽 정렬",
                Layout::Right,
                Some(Mode::Swatch),
            ),
            (
                "모두 펼치기 · 계단식 정렬",
                Layout::Cascade,
                Some(Mode::Sticker),
            ),
        ] {
            if ui.button(label).clicked() {
                pick = Some((layout, fold));
            }
        }
        if let Some((layout, fold)) = pick {
            self.arrange = Some(Arrange { layout, fold, area });
            done = true;
        }
        // "닫기" belongs to the sticker's own right-click menu, not to the
        // "더보기 › 스티커 정렬" submenu: that submenu is specifically about
        // arranging stickers, and repeating the close actions there just
        // duplicated what the main context menu already offers.
        if own {
            ui.separator();
            if ui.button("이 스티커 닫기").clicked() {
                w.open = false;
                done = true;
            }
            if ui.button("모든 스티커 닫기  Ctrl+Shift+H").clicked() {
                w.open = false;
                for other in self.studio.windows.values_mut() {
                    other.open = false;
                }
                done = true;
            }
        }
        done
    }
    /// Shows the sticker menu as its own small window at the pointer, so it
    /// is never clipped by a title bar or a tiny color box.
    fn sticker_menu_window(&mut self, ctx: &egui::Context, i: usize, w: &mut Window) {
        let Some(mut m) = w.menu.take() else {
            return;
        };
        // A click back on the sticker itself dismisses the menu.
        if !std::mem::take(&mut m.fresh) && ctx.input(|v| v.pointer.any_pressed()) {
            return;
        }
        let area = crate::native_window::work_area(ctx).unwrap_or_else(|| ctx.screen_rect());
        let pos = fit_in(m.at, m.size, area);
        let mut sub = m.sub;
        let mut done = false;
        let paper = (m.kind == MenuKind::Linked).then_some(self.data.notes[i].color);
        let shown = menu_window(
            ctx,
            egui::ViewportId::from_hash_of(("sticker_menu", i)),
            pos,
            m.size,
            paper,
            |ui| match m.kind {
                MenuKind::Context => self.sticker_menu(ui, i, w, true),
                MenuKind::More => self.more_items(ui, i, w, pos, &mut sub),
                MenuKind::Project => self.project_items(ui, i),
                MenuKind::Linked => self.linked_items(ui, i),
            },
        );
        m.size = shown.size;
        done |= shown.result == Some(true);
        let mut focused = shown.focused;
        let mut escape = shown.escape;
        if let Some((kind, top)) = sub {
            let parent = Rect::from_min_size(pos, m.size);
            let spos = submenu_pos(parent, m.sub_size, top, area);
            let shown = menu_window(
                ctx,
                egui::ViewportId::from_hash_of(("sticker_submenu", i)),
                spos,
                m.sub_size,
                None,
                |ui| match kind {
                    SubMenu::Project => self.project_items(ui, i),
                    SubMenu::Arrange => self.sticker_menu(ui, i, w, false),
                },
            );
            m.sub_size = shown.size;
            done |= shown.result == Some(true);
            focused |= shown.focused;
            escape |= shown.escape;
        }
        m.sub = sub;
        // Focus moves between the menu and its submenu window; only close when
        // neither has had it for a moment.
        let now = ctx.input(|v| v.time);
        if focused {
            m.seen_focus = true;
            m.away_since = None;
        } else if m.seen_focus {
            let since = *m.away_since.get_or_insert(now);
            done |= now - since > 0.3;
        }
        if !(done || escape) {
            w.menu = Some(m);
        }
    }
    /// The memo's "더보기" items. Rows with a submenu open it beside this
    /// menu window instead of inside it.
    fn more_items(
        &mut self,
        ui: &mut egui::Ui,
        i: usize,
        w: &mut Window,
        origin: Pos2,
        sub: &mut Option<(SubMenu, f32)>,
    ) -> bool {
        let ctx = ui.ctx().clone();
        let mut done = false;
        let mut hovered_plain = false;
        let mut plain = |r: &egui::Response| {
            hovered_plain |= r.hovered();
            r.clicked()
        };
        if plain(&ui.button("일정 설정…")) {
            self.open_schedule(i);
            done = true;
        }
        if plain(&ui.button("메모 잠금")) {
            self.ask_lock(i);
            done = true;
        }
        let mut wiki = self.data.notes[i].wiki;
        let r = ui.checkbox(&mut wiki, "위키에 보관");
        plain(&r);
        if r.changed() {
            self.data.notes[i].wiki = wiki;
        }
        for (label, kind) in [
            ("프로젝트 설정", SubMenu::Project),
            ("스티커 정렬", SubMenu::Arrange),
        ] {
            let open = sub.is_some_and(|(k, _)| k == kind);
            let r = submenu_row(ui, label, open);
            if r.hovered() || r.clicked() {
                *sub = Some((kind, origin.y + r.rect.top() - 6.));
            }
        }
        if plain(&ui.button("동일 프로젝트 보기")) {
            self.project_filter = Some(self.data.notes[i].project.unwrap_or(NO_PROJECT));
            self.page = 0;
            self.main_open = true;
            ctx.send_viewport_cmd_to(main_viewport(), egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd_to(main_viewport(), egui::ViewportCommand::Focus);
            done = true;
        }
        if plain(&ui.button("메모 복제본 생성")) {
            self.duplicate_note(i);
            done = true;
        }
        ui.separator();
        if plain(&ui.button("메모 삭제 · 휴지통")) {
            self.trash_note(i);
            w.open = false;
            done = true;
        }
        if plain(&ui.button(RichText::new("메모 영구삭제…").color(ACCENT))) {
            self.request_purge(i);
            done = true;
        }
        ui.separator();
        if plain(&ui.button("도움말")) {
            w.help = true;
            done = true;
        }
        if plain(&ui.button("메모창 설정")) {
            self.settings = true;
            crate::ui::raise(&ctx, "settings");
            self.desktop.settings_tab = 2;
            done = true;
        }
        if hovered_plain {
            *sub = None;
        }
        done
    }
    /// Memos linked to `i`, each in a bar of its own color; a click opens it
    /// as a sticker.
    pub(crate) fn linked_items(&mut self, ui: &mut egui::Ui, i: usize) -> bool {
        let linked: Vec<_> = self
            .data
            .notes
            .iter()
            .enumerate()
            .filter(|(j, n)| {
                self.visible(*j)
                    && *j != i
                    && (n.parents.contains(&i)
                        || self.data.notes[i].parents.contains(j)
                        || self.data.links.contains(&(i.min(*j), i.max(*j))))
            })
            .map(|(j, _)| j)
            .collect();
        ui.horizontal(|ui| {
            ui.label(RichText::new("연결된 메모").strong().size(13.).color(INK));
            ui.label(
                RichText::new(linked.len().to_string())
                    .size(12.)
                    .color(MUTED),
            );
        });
        ui.add_space(4.);
        if linked.is_empty() {
            crate::ui::hint(ui, "연결된 메모가 없습니다");
            crate::ui::hint(ui, "전체메모 › 관계 보드에서 연결할 수 있습니다");
            ui.add_space(4.);
            return false;
        }
        let mut open = None;
        ui.spacing_mut().item_spacing.y = 6.;
        for j in linked {
            let n = &self.data.notes[j];
            let (rect, r) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.), Sense::click());
            let fill = if r.hovered() {
                mix(n.color, Color32::BLACK, 0.1)
            } else {
                n.color
            };
            ui.painter().rect(
                rect,
                8.,
                fill,
                egui::Stroke::new(
                    1.0_f32,
                    Color32::from_black_alpha(if r.hovered() { 40 } else { 22 }),
                ),
                egui::StrokeKind::Inside,
            );
            let title = if n.title.is_empty() {
                "새 메모"
            } else {
                &n.title
            };
            let text = if n.starred {
                format!("★  {title}")
            } else {
                title.to_string()
            };
            let painter = ui.painter().with_clip_rect(rect.shrink(4.));
            painter.text(
                Pos2::new(rect.left() + 12., rect.center().y),
                egui::Align2::LEFT_CENTER,
                text,
                egui::FontId::proportional(13.5),
                INK,
            );
            let r = r
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text("스티커로 열기");
            if r.clicked() {
                open = Some(j);
            }
        }
        if let Some(j) = open {
            self.open_sticker(j, self.data.notes[j].date);
        }
        open.is_some()
    }
    /// Pick the memo's project. Returns true once one is chosen.
    pub(crate) fn project_items(&mut self, ui: &mut egui::Ui, i: usize) -> bool {
        crate::ui::caption(ui, "프로젝트");
        let mut picked = false;
        let current = self.data.notes[i].project;
        let mut choices = vec![(None, "프로젝트 없음".to_string())];
        for (p, name) in self.data.projects.iter().enumerate() {
            if !self.extra.hidden_projects.contains(&p) && !self.extra.deleted_projects.contains(&p)
            {
                choices.push((Some(p), name.clone()));
            }
        }
        for (p, name) in choices {
            let label = if p.is_some() {
                format!("#  {name}")
            } else {
                name
            };
            if ui.selectable_label(current == p, label).clicked() {
                self.data.notes[i].project = p;
                picked = true;
            }
        }
        picked
    }
}

/// The day the daily memos belong to: a new day starts at 05:00.
fn daily_day(now: chrono::NaiveDateTime) -> NaiveDate {
    (now - chrono::Duration::hours(5)).date()
}
/// Local time for the daily memos. `OMNIMEMOJANG_TEST_NOW`
/// (`YYYY-MM-DD HH:MM:SS`) starts a test clock at that time instead of the
/// system clock, so the 05:00 rollover can be tried without changing Windows.
fn daily_now() -> chrono::NaiveDateTime {
    static START: std::sync::OnceLock<Option<(chrono::NaiveDateTime, std::time::Instant)>> =
        std::sync::OnceLock::new();
    let start = START.get_or_init(|| {
        let text = std::env::var("OMNIMEMOJANG_TEST_NOW").ok()?;
        let at = chrono::NaiveDateTime::parse_from_str(text.trim(), "%Y-%m-%d %H:%M:%S").ok()?;
        Some((at, std::time::Instant::now()))
    });
    match start {
        Some((at, since)) => *at + chrono::Duration::from_std(since.elapsed()).unwrap_or_default(),
        None => Local::now().naive_local(),
    }
}
/// Which menu a sticker shows in its menu window.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuKind {
    /// Right click: arrange and close stickers.
    Context,
    /// The "더보기" button.
    More,
    /// The footer's project picker.
    Project,
    /// The footer's linked memos, in the sticker's own color.
    Linked,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SubMenu {
    Project,
    Arrange,
}
/// A sticker menu shown in its own always-on-top window, so short stickers
/// never clip it; its submenu gets a second window beside it.
#[derive(Clone, Copy, Debug)]
pub struct Menu {
    kind: MenuKind,
    at: Pos2,
    size: Vec2,
    sub: Option<(SubMenu, f32)>,
    sub_size: Vec2,
    seen_focus: bool,
    away_since: Option<f64>,
    /// Opened this frame: the click that opened it may still read as a press.
    fresh: bool,
}
impl Menu {
    fn new(kind: MenuKind, at: Pos2) -> Self {
        Self {
            kind,
            at,
            size: Vec2::new(228., 120.),
            sub: None,
            sub_size: Vec2::new(228., 120.),
            seen_focus: false,
            away_since: None,
            fresh: true,
        }
    }
}
/// Toggle `kind` opening below `anchor` (a rect in the sticker's points).
fn toggle_menu(ctx: &egui::Context, w: &mut Window, kind: MenuKind, anchor: Rect) {
    if w.menu.is_some_and(|m| m.kind == kind) {
        w.menu = None;
        return;
    }
    let origin = ctx
        .input(|v| v.viewport().inner_rect.map(|r| r.min))
        .unwrap_or_default();
    w.menu = Some(Menu::new(
        kind,
        origin + anchor.left_bottom().to_vec2() + Vec2::new(0., 2.),
    ));
}
/// Keep a window of `size` at `at` inside `area`.
fn fit_in(at: Pos2, size: Vec2, area: Rect) -> Pos2 {
    Pos2::new(
        at.x.min(area.right() - size.x).max(area.left()),
        at.y.min(area.bottom() - size.y).max(area.top()),
    )
}
/// Place a submenu beside `parent`: on the right, or on the left when the
/// right side would leave the work area.
fn submenu_pos(parent: Rect, size: Vec2, top: f32, area: Rect) -> Pos2 {
    let right = parent.right() + 2.;
    let x = if right + size.x <= area.right() {
        right
    } else {
        (parent.left() - 2. - size.x).max(area.left())
    };
    fit_in(Pos2::new(x, top), size, area)
}
fn submenu_row(ui: &mut egui::Ui, label: &str, open: bool) -> egui::Response {
    let r = ui.add(egui::Button::new(label).selected(open));
    ui.painter().text(
        Pos2::new(r.rect.right() - 10., r.rect.center().y),
        egui::Align2::RIGHT_CENTER,
        "›",
        egui::FontId::proportional(15.),
        crate::ui::t(ui).muted,
    );
    r
}
pub(crate) struct Shown<R> {
    pub size: Vec2,
    pub focused: bool,
    pub escape: bool,
    pub result: Option<R>,
}
/// One borderless always-on-top menu window at `pos`; it resizes itself to
/// its content.
pub(crate) fn menu_window<R>(
    ctx: &egui::Context,
    id: egui::ViewportId,
    pos: Pos2,
    size: Vec2,
    paper: Option<Color32>,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> Shown<R> {
    // A borderless always-on-top window is placed by raw screen coordinates
    // with no OS-level clamping, so a menu opened near a monitor's edge
    // would otherwise open partly or fully off-screen and be unreachable.
    let area = crate::native_window::work_area(ctx).unwrap_or_else(|| ctx.screen_rect());
    let pos = fit_in(pos, size, area);
    let builder = egui::ViewportBuilder::default()
        .with_title("메모 메뉴")
        .with_position(pos)
        .with_inner_size(size)
        .with_decorations(false)
        .with_resizable(false)
        .with_taskbar(false)
        .with_transparent(true)
        .with_window_level(egui::WindowLevel::AlwaysOnTop);
    let mut shown = Shown {
        size,
        focused: false,
        escape: false,
        result: None,
    };
    let mut add = Some(add);
    ctx.show_viewport_immediate(id, builder, |ctx, _| {
        shown.focused = ctx.input(|v| v.viewport().focused == Some(true));
        shown.escape = ctx.input(|v| v.key_pressed(egui::Key::Escape));
        crate::win_style::shape(ctx, 1.);
        let mut frame = egui::Frame::popup(&ctx.style())
            .shadow(egui::epaint::Shadow::NONE)
            .inner_margin(egui::Margin::same(6));
        if crate::win_style::NATIVE {
            frame = frame.corner_radius(0.);
        }
        if let Some(paper) = paper {
            frame = frame
                .fill(paper)
                .stroke(egui::Stroke::new(1.0_f32, Color32::from_black_alpha(28)));
        }
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            if let Some(paper) = paper {
                paper_style(ui, paper);
            }
            let v = &mut ui.style_mut().visuals.widgets;
            v.inactive.weak_bg_fill = Color32::TRANSPARENT;
            v.inactive.bg_fill = Color32::TRANSPARENT;
            ui.style_mut().spacing.button_padding = Vec2::new(10., 5.);
            ui.style_mut().spacing.item_spacing.y = 2.;
            let r = ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                ui.set_width(size.x - 12.);
                add.take().map(|f| f(ui))
            });
            shown.result = r.inner;
            let want = Vec2::new(size.x, (r.response.rect.height() + 14.).ceil());
            if (want.y - size.y).abs() > 1. {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(want));
            }
            shown.size = want;
        });
    });
    shown
}
fn open_menu(ctx: &egui::Context, w: &mut Window) {
    let (origin, pointer) = ctx.input(|v| {
        (
            v.viewport().inner_rect.map(|r| r.min),
            v.pointer.interact_pos(),
        )
    });
    if let (Some(origin), Some(pointer)) = (origin, pointer) {
        w.menu = Some(Menu::new(MenuKind::Context, origin + pointer.to_vec2()));
    }
}
pub fn viewport(i: usize) -> egui::ViewportId {
    egui::ViewportId::from_hash_of(("memo", i))
}
impl App {
    pub fn new_sticker(&mut self) -> usize {
        let i = self.data.notes.len();
        let mut n = Note::new("", None, i);
        n.body.clear();
        n.color = self.desktop.default_color;
        n.document.font_name = self.desktop.note_font.clone();
        n.document.font_size = self.desktop.note_font_size * 4. / 3.;
        n.document.opacity = self.desktop.opacity / 100.;
        n.document.image_scale = [0.25, 0.5, 0.75, 1., 1.][self.desktop.image_scale];
        n.document.image_fit = self.desktop.image_scale == 4;
        // The caret starts in the body, so a quick memo can be typed at once.
        n.document.requested_selection = Some(0..0);
        self.data.notes.push(n);
        self.studio.windows.insert(i, Window::new(Mode::Sticker));
        i
    }
    pub fn render_note_windows(&mut self, root: &egui::Context) {
        let ids: Vec<_> = self
            .studio
            .windows
            .iter()
            .filter(|(_, w)| w.open)
            .map(|(&i, _)| i)
            .collect();
        for i in ids {
            let Some(mut w) = self.studio.windows.remove(&i) else {
                continue;
            };
            if self.data.notes[i].deleted
                || (!self.visible(i) && !(self.extra.search_mode && self.extra.include_hidden))
            {
                w.open = false;
                self.studio.windows.insert(i, w);
                continue;
            }
            w.reconcile(&self.data.notes[i]);
            w.skip_title = self.desktop.skip_title;
            let locked = self.data.notes[i].locked;
            if locked {
                w.mode = Mode::Swatch;
            }
            if self.data.notes[i].duration > 1 && w.occurrence.is_none() {
                let today = Local::now().date_naive();
                w.occurrence = if self.data.notes[i].occurs(today) {
                    Some(today)
                } else {
                    self.data.notes[i].date
                };
            }
            let previous_active = self.active;
            let previous_occurrence = self.extra.edit_occurrence;
            self.active = i;
            self.extra.edit_occurrence = w.occurrence;
            let mode = w.mode;
            let key = (mode, locked);
            let size = if locked { Vec2::splat(64.) } else { w.size() };
            let min = if locked {
                size
            } else {
                match mode {
                    Mode::Editor => Vec2::new(480., 480.),
                    Mode::Sticker => Vec2::new(340., 220.),
                    Mode::Title => Vec2::new(160., 40.),
                    Mode::Swatch => Vec2::splat(16.),
                }
            };
            let max = if locked {
                size
            } else {
                match mode {
                    Mode::Title => Vec2::new(1600., 40.),
                    Mode::Swatch => Vec2::splat(640.),
                    _ => Vec2::splat(8192.),
                }
            };
            let top = self.data.notes[i].topmost
                && !(mode == Mode::Swatch && self.desktop.unpin_minimized);
            let title = if self.data.notes[i].title.is_empty() {
                "새 메모"
            } else {
                &self.data.notes[i].title
            };
            let mut builder = egui::ViewportBuilder::default()
                .with_title(title)
                .with_min_inner_size(min)
                .with_max_inner_size(max)
                .with_resizable(!locked && mode != Mode::Swatch)
                .with_transparent(true)
                .with_decorations(false)
                .with_taskbar(self.desktop.taskbar)
                .with_icon(crate::app_icon::window())
                .with_window_level(if top {
                    egui::WindowLevel::AlwaysOnTop
                } else {
                    egui::WindowLevel::Normal
                });
            // The builder is a creation request, never a live geometry feedback loop.
            if w.applied.is_none() {
                builder = builder.with_inner_size(size);
            }
            root.show_viewport_immediate(viewport(i), builder, |ctx, _| {
                self.theme(ctx);
                if w.move_caret_to_body {
                    w.move_caret_to_body = false;
                    self.data.notes[i].document.requested_selection = Some(0..0);
                }
                let maximized = ctx.input(|v| v.viewport().maximized.unwrap_or(false));
                let changed = w.applied != Some(key);
                if changed {
                    w.native.cancel();
                    if maximized {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(min));
                    ctx.send_viewport_cmd(egui::ViewportCommand::MaxInnerSize(max));
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(
                        !locked && mode != Mode::Swatch,
                    ));
                    w.applied = Some(key);
                } else if !maximized {
                    if let Some(rect) = ctx.input(|v| v.viewport().inner_rect) {
                        if rect.width() >= min.x && rect.height() >= min.y {
                            match mode {
                                Mode::Editor => w.full_size = rect.size(),
                                Mode::Sticker => w.sticker_size = rect.size(),
                                Mode::Title => w.title_width = rect.width(),
                                Mode::Swatch => w.swatch_size = rect.size(),
                            }
                        }
                    }
                }
                if changed || w.center {
                    if let Some(area) = crate::native_window::work_area(ctx) {
                        let position = if w.center {
                            // New windows step down-right so they do not
                            // cover the ones already open.
                            let open = self.studio.windows.values().filter(|o| o.open).count();
                            area.center() - size * 0.5 + Vec2::splat(28. * (open % 8) as f32)
                        } else {
                            ctx.input(|v| v.viewport().outer_rect.map(|r| r.min))
                                .unwrap_or(area.min)
                        };
                        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                            crate::native_window::fit_position(position, size, area),
                        ));
                    } else if w.center {
                        if let Some(cmd) = egui::ViewportCommand::center_on_screen(ctx) {
                            ctx.send_viewport_cmd(cmd);
                        }
                    }
                    w.center = false;
                }
                if let Some(mut p) = w.place.take() {
                    // A window saved on a monitor that is gone comes back on
                    // the current one.
                    let monitors = crate::native_window::monitors(ctx);
                    let grip = p + Vec2::new(40., 12.);
                    if !monitors.is_empty() && !monitors.iter().any(|m| m.contains(grip)) {
                        if let Some(area) = crate::native_window::work_area(ctx) {
                            p = crate::native_window::fit_position(p, size, area);
                        }
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(p));
                }
                if w.focus {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    w.focus = false;
                }
                if ctx.input(|v| v.viewport().close_requested()) {
                    w.open = false;
                }
                if let Some(p) = ctx.input(|v| v.viewport().outer_rect.map(|r| r.min)) {
                    w.last_pos = Some(p);
                }
                self.memo_shortcuts(ctx, i, &mut w);
                // The color box is resized by its own handle: Windows enforces a
                // minimum tracking width on natively resizable windows, which
                // stopped boxes from shrinking below a wide rectangle.
                w.native.input(
                    ctx,
                    mode == Mode::Title,
                    false,
                    !locked && !maximized && mode != Mode::Swatch,
                    min,
                    max,
                );
                let base = self.data.notes[i].color;
                let opacity = self.data.notes[i].document.opacity.clamp(0.2, 1.);
                // On Windows the OS applies opacity and rounds the corners, so
                // the paper is painted opaque and square (see `win_style`).
                crate::win_style::shape(ctx, opacity);
                let alpha = if crate::win_style::NATIVE {
                    1.
                } else {
                    opacity
                };
                let radius: u8 = if crate::win_style::NATIVE { 0 } else { 12 };
                let paper = translucent(base, alpha);
                if mode == Mode::Title {
                    egui::CentralPanel::default()
                        .frame(
                            egui::Frame::NONE
                                .fill(translucent(mix(base, Color32::WHITE, 0.15), alpha))
                                .corner_radius(radius.min(8))
                                .inner_margin(egui::Margin::symmetric(6, 5)),
                        )
                        .show(ctx, |ui| {
                            paper_style(ui, paper);
                            // Tooltips cannot fit in a 44 px tall window: egui kept
                            // flipping them above and below the pointer (flicker).
                            ui.style_mut().interaction.tooltip_delay = f32::INFINITY;
                            let hover = !self.desktop.auto_toolbar
                                || ui.rect_contains_pointer(ui.max_rect());
                            let button = 28. + ui.spacing().item_spacing.x;
                            ui.horizontal(|ui| {
                                // The buttons' space is always reserved so the title
                                // does not move when they appear on hover.
                                if hover {
                                    if icon_button(ui, Icon::Edit, "메모 상세 편집").clicked()
                                    {
                                        w.edit();
                                    }
                                } else {
                                    ui.add_space(button);
                                }
                                let starred = self.data.notes[i].starred;
                                if starred {
                                    ui.label(RichText::new("★").color(STAR).size(15.));
                                }
                                let width = (ui.available_width() - 2. * button).max(40.);
                                let r = ui.add_sized(
                                    [width, 26.],
                                    egui::Label::new(if self.data.notes[i].title.is_empty() {
                                        "새 메모"
                                    } else {
                                        &self.data.notes[i].title
                                    })
                                    .truncate()
                                    .sense(Sense::click_and_drag()),
                                );
                                if r.clicked() {
                                    w.sticker();
                                } else if r.drag_started() || r.drag_stopped() {
                                    w.native.start_drag(ctx);
                                }
                                if r.secondary_clicked() {
                                    open_menu(ctx, &mut w);
                                }
                                if hover {
                                    if icon_button(ui, Icon::Fold, "색상 상자로 접기").clicked()
                                    {
                                        w.fold();
                                    }
                                    pin_button(ui, &mut self.data.notes[i].topmost);
                                }
                            });
                        });
                } else if mode == Mode::Swatch {
                    egui::CentralPanel::default()
                        .frame(
                            egui::Frame::NONE
                                .fill(translucent(base, alpha))
                                .corner_radius(radius.min(8)),
                        )
                        .show(ctx, |ui| {
                            let rect = ui.max_rect();
                            if !locked {
                                swatch_resize(ctx, ui, &mut w);
                            }
                            let r = ui.interact(
                                rect.shrink(7.),
                                ui.id().with("swatch"),
                                Sense::click_and_drag(),
                            );
                            if locked {
                                crate::icons::paint_icon(
                                    ui.painter(),
                                    Rect::from_center_size(rect.center(), Vec2::splat(28.)),
                                    Icon::Lock,
                                    INK,
                                );
                            }
                            if r.drag_started() || r.drag_stopped() {
                                w.native.start_drag(ctx);
                            }
                            // A click (no movement) unfolds; moving the pointer
                            // first turns the press into a window drag instead.
                            if r.clicked() {
                                if locked {
                                    self.ask_unlock(i, true);
                                } else {
                                    w.sticker();
                                }
                            }
                            if r.on_hover_cursor(egui::CursorIcon::Grab)
                                .secondary_clicked()
                            {
                                open_menu(ctx, &mut w);
                            }
                        });
                } else {
                    let focused = ctx.input(|v| v.viewport().focused.unwrap_or(false));
                    let engaged = mode == Mode::Editor
                        || !self.desktop.auto_toolbar
                        || focused
                        || ctx.input(|v| v.pointer.hover_pos().is_some());
                    egui::TopBottomPanel::top(egui::Id::new(("memo_header", i)))
                        .exact_height(38.)
                        .show_separator_line(false)
                        .frame(
                            egui::Frame::NONE
                                .fill(paper)
                                .corner_radius(egui::CornerRadius {
                                    nw: radius,
                                    ne: radius,
                                    sw: 0,
                                    se: 0,
                                })
                                .inner_margin(egui::Margin::symmetric(10, 4)),
                        )
                        .show(ctx, |ui| {
                            paper_style(ui, paper);
                            self.memo_header(ui, i, &mut w, mode, engaged, maximized);
                        });
                    if self.active != i {
                        self.active = i;
                        self.extra.edit_occurrence = w.occurrence;
                    }
                    egui::TopBottomPanel::top(egui::Id::new(("memo_title", i)))
                        .show_separator_line(false)
                        .frame(egui::Frame::NONE.fill(paper).inner_margin(
                            if mode == Mode::Sticker {
                                egui::Margin {
                                    left: 14,
                                    right: 14,
                                    top: 0,
                                    bottom: 2,
                                }
                            } else {
                                egui::Margin {
                                    left: 18,
                                    right: 18,
                                    top: 4,
                                    bottom: 6,
                                }
                            },
                        ))
                        .show(ctx, |ui| {
                            paper_style(ui, paper);
                            if mode == Mode::Sticker {
                                let font = egui::FontId::new(
                                    18.,
                                    crate::desktop::family(
                                        ui,
                                        &self.data.notes[i].document.font_name,
                                    ),
                                );
                                ui.horizontal(|ui| {
                                    if self.data.notes[i].starred {
                                        ui.label(RichText::new("★").color(STAR).size(17.))
                                            .on_hover_text("중요 메모");
                                    }
                                    let title = ui.add(
                                        egui::TextEdit::singleline(&mut self.data.notes[i].title)
                                            .font(font)
                                            .frame(false)
                                            .desired_width(f32::INFINITY)
                                            .hint_text("제목"),
                                    );
                                    if title.lost_focus()
                                        && ui.input(|v| v.key_pressed(egui::Key::Enter))
                                    {
                                        // Deferred to next frame; see
                                        // `move_caret_to_body`'s doc comment.
                                        w.move_caret_to_body = true;
                                    }
                                });
                            } else {
                                self.note_editor(ui);
                            }
                        });
                    let width = ctx
                        .input(|v| v.viewport().inner_rect.map(|r| r.width()))
                        .unwrap_or(size.x);
                    let status_height = if self.data.notes[i].document.status.is_empty() {
                        0.
                    } else {
                        let small_font = egui::TextStyle::Small.resolve(&ctx.style());
                        ctx.fonts(|f| f.row_height(&small_font)) + 4.
                    };
                    let footer_height = (if engaged {
                        if width >= 300. {
                            72.
                        } else {
                            102.
                        }
                    } else {
                        40.
                    }) + status_height;
                    egui::TopBottomPanel::bottom(egui::Id::new(("memo_footer", i)))
                        .exact_height(footer_height)
                        .show_separator_line(false)
                        .frame(
                            egui::Frame::NONE
                                .fill(paper)
                                .corner_radius(egui::CornerRadius {
                                    nw: 0,
                                    ne: 0,
                                    sw: radius,
                                    se: radius,
                                })
                                .inner_margin(egui::Margin {
                                    left: 10,
                                    right: 8,
                                    top: 2,
                                    bottom: 6,
                                }),
                        )
                        .show(ctx, |ui| {
                            paper_style(ui, paper);
                            self.data.notes[i].document.toolbar_visible = engaged;
                            let rect = ui.max_rect();
                            crate::composer::footer(
                                ui,
                                &mut self.data.notes[i].document,
                                ("memo_document", i),
                                if engaged { 62. } else { 0. },
                                mode == Mode::Editor,
                            );
                            if engaged {
                                let arrows = Rect::from_min_size(
                                    Pos2::new(rect.right() - 60., rect.bottom() - 28.),
                                    Vec2::new(60., 28.),
                                );
                                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(arrows), |ui| {
                                    ui.horizontal(|ui| {
                                        let r = icon_button(ui, Icon::ChevronDown, "프로젝트 선택");
                                        if r.clicked() {
                                            toggle_menu(ctx, &mut w, MenuKind::Project, r.rect);
                                        }
                                        let r = icon_button(ui, Icon::ChevronRight, "연결된 메모");
                                        if r.clicked() {
                                            toggle_menu(ctx, &mut w, MenuKind::Linked, r.rect);
                                        }
                                    });
                                });
                            }
                        });
                    egui::CentralPanel::default()
                        .frame(egui::Frame::NONE.fill(paper).inner_margin(
                            if mode == Mode::Sticker {
                                egui::Margin::symmetric(14, 2)
                            } else {
                                egui::Margin::symmetric(18, 4)
                            },
                        ))
                        .show(ctx, |ui| {
                            paper_style(ui, paper);
                            if mode == Mode::Sticker {
                                let n = &mut self.data.notes[i];
                                crate::composer::content(
                                    ui,
                                    &mut n.body,
                                    &mut n.document,
                                    ("memo_document", i),
                                );
                            } else {
                                egui::Frame::NONE
                                    .fill(Color32::from_white_alpha(70))
                                    .stroke(egui::Stroke::new(
                                        1.0_f32,
                                        Color32::from_black_alpha(30),
                                    ))
                                    .corner_radius(8.)
                                    .inner_margin(egui::Margin::symmetric(12, 8))
                                    .show(ui, |ui| {
                                        ui.set_min_size(ui.available_size());
                                        let n = &mut self.data.notes[i];
                                        crate::composer::content(
                                            ui,
                                            &mut n.body,
                                            &mut n.document,
                                            ("memo_document", i),
                                        );
                                    });
                            }
                        });
                    self.consume_body_project(i);
                    if mode == Mode::Sticker
                        && self.data.notes[i].document.active == 0
                        && (!focused || self.data.notes[i].body.contains('\n'))
                    {
                        let n = &mut self.data.notes[i];
                        crate::desktop::infer_title(&mut n.title, &mut n.body);
                    }
                }
                if w.mode != mode || !w.open {
                    w.native.cancel();
                    w.help = false;
                    w.menu = None;
                    if w.mode == Mode::Swatch && self.desktop.unpin_minimized {
                        self.data.notes[i].topmost = false;
                    }
                    if self
                        .extra
                        .picker
                        .as_ref()
                        .is_some_and(|p| p.owner == ctx.viewport_id())
                    {
                        self.extra.picker = None;
                    }
                    if self.extra.scope_owner == ctx.viewport_id() {
                        self.extra.completion_scope = None;
                    }
                }
                if self.active != i {
                    self.active = i;
                    self.extra.edit_occurrence = w.occurrence;
                }
                self.picker_window(ctx);
                self.scope_dialog(ctx);
                self.sticker_menu_window(ctx, i, &mut w);
                let mut help = w.help;
                crate::ui::dialog(
                    ctx,
                    ("memo_help", i),
                    "메모 도움말",
                    [340., 300.],
                    true,
                    &mut help,
                    |ui| {
                        for line in [
                            "윗부분을 끌어 이동하고 가장자리로 크기를 조절합니다.",
                            "접기: 제목 한 줄 → 색상 상자 · 상자를 두 번 클릭하면 복원",
                            "빈 곳을 우클릭하면 스티커 정렬 메뉴가 열립니다.",
                            "이미지와 표는 Ctrl+V로 본문에 붙여 넣습니다.",
                            "Esc를 두 번 누르면 창을 닫습니다. 메모는 유지됩니다.",
                        ] {
                            ui.label(format!("·  {line}"));
                        }
                        ui.add_space(8.);
                        crate::ui::hint(ui, format!("새 스티커 · {}", self.shortcut_label(2)));
                    },
                );
                w.help = help;
            });
            if self.active == i {
                w.occurrence = self.extra.edit_occurrence;
            }
            self.active = previous_active;
            self.extra.edit_occurrence = previous_occurrence;
            self.studio.windows.insert(i, w);
        }
        self.editor = self
            .studio
            .windows
            .values()
            .any(|w| w.open && w.mode == Mode::Editor);
        self.sticky = self.studio.windows.values().any(|w| w.open);
    }
    fn memo_shortcuts(&mut self, ctx: &egui::Context, i: usize, w: &mut Window) {
        if ctx.input_mut(|v| v.consume_key(egui::Modifiers::CTRL, egui::Key::T)) {
            self.data.notes[i].topmost = !self.data.notes[i].topmost;
        }
        if ctx.input_mut(|v| {
            v.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::H)
        }) {
            w.open = false;
            for other in self.studio.windows.values_mut() {
                other.open = false;
            }
        }
        if ctx.input_mut(|v| v.consume_key(egui::Modifiers::CTRL, egui::Key::P)) {
            self.desktop.project_popup = true;
            self.desktop.focus_dialog = true;
        }
        if ctx.input_mut(|v| v.consume_key(egui::Modifiers::CTRL, egui::Key::F)) {
            self.catalog.focus_find();
            self.catalog.tab = 1;
            self.page = 4;
            self.main_open = true;
            ctx.send_viewport_cmd_to(main_viewport(), egui::ViewportCommand::Focus);
        }
        if ctx.input_mut(|v| v.consume_key(egui::Modifiers::CTRL, egui::Key::H)) {
            self.catalog.replace_open = true;
            self.catalog.selected = Some(i);
            self.page = 4;
            self.main_open = true;
            ctx.send_viewport_cmd_to(main_viewport(), egui::ViewportCommand::Focus);
        }
        if ctx.input_mut(|v| v.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            if ctx.memory(|m| m.any_popup_open()) {
                ctx.memory_mut(|m| m.close_popup());
                return;
            }
            let now = ctx.input(|v| v.time);
            let old = self.desktop.last_escape.insert(ctx.viewport_id(), now);
            if old.is_some_and(|t| now - t < 0.8) {
                w.open = false;
            }
            w.help = false;
            w.menu = None;
            self.extra.picker = None;
        }
    }
    fn memo_header(
        &mut self,
        ui: &mut egui::Ui,
        i: usize,
        w: &mut Window,
        mode: Mode,
        engaged: bool,
        maximized: bool,
    ) {
        let context = ui.ctx().clone();
        let ctx = &context;
        ui.spacing_mut().item_spacing.x = 4.;
        ui.spacing_mut().button_padding = Vec2::new(6., 4.);
        ui.horizontal(|ui| {
            if mode == Mode::Sticker {
                if icon_button(ui, Icon::Plus, "새 스티커 메모").clicked() {
                    self.new_sticker();
                }
                if engaged {
                    crate::ui::icon_menu(ui, Icon::Palette, "메모 색상", |ui| {
                        crate::desktop::color_choices(ui, &mut self.data.notes[i].color)
                    });
                }
            } else {
                if icon_button(ui, Icon::Fold, "메모 접기").clicked() {
                    w.fold();
                }
                if icon_button(ui, Icon::Sticky, "스티커 노트로 전환").clicked() {
                    w.sticker();
                }
            }
            let controls = if mode == Mode::Editor {
                7.
            } else if engaged {
                8.
            } else {
                3.
            };
            let gap = (ui.available_width() - controls * 32.).max(0.);
            let response = ui.add_sized(
                [gap, 28.],
                egui::Label::new(
                    RichText::new(if mode == Mode::Editor {
                        "메모 편집"
                    } else {
                        ""
                    })
                    .size(11.)
                    .color(MUTED),
                )
                .sense(Sense::click_and_drag()),
            );
            if response.drag_started() || response.drag_stopped() {
                w.native.start_drag(ctx);
            }
            if response.secondary_clicked() {
                open_menu(ctx, w);
            }
            // The header's gap is blank only in Sticker mode (Editor shows
            // "메모 편집" there), so this is the sticker's one unambiguous
            // empty area: double-clicking it toggles fullscreen, mirroring
            // the native title-bar gesture.
            if mode == Mode::Sticker && response.double_clicked() {
                w.native.cancel();
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            }
            if engaged {
                let r = icon_button(ui, Icon::More, "더보기");
                if r.clicked() {
                    toggle_menu(ctx, w, MenuKind::More, r.rect);
                }
            }
            if mode == Mode::Editor {
                crate::ui::icon_menu(ui, Icon::Opacity, "메모 투명도", |ui| {
                    crate::ui::caption(ui, "투명도");
                    let mut opacity = self.data.notes[i].document.opacity * 100.;
                    if ui
                        .add(
                            egui::Slider::new(&mut opacity, 20.0..=100.)
                                .integer()
                                .suffix("%"),
                        )
                        .changed()
                    {
                        self.data.notes[i].document.opacity = opacity / 100.;
                    }
                });
            }
            if engaged && icon_button(ui, Icon::Alarm, "알람 날짜와 시간").clicked() {
                self.open_reminder(i);
            }
            if engaged {
                let n = &self.data.notes[i];
                let tip = match n.date {
                    Some(d) if n.repeat.is_none() => format!("일정 · {}", d.format("%m.%d")),
                    Some(d) => format!("일정 · {} · {}", d.format("%m.%d"), n.repeat.describe(d)),
                    None => "일정 설정".into(),
                };
                if icon_button(ui, Icon::Calendar, &tip).clicked() {
                    self.open_schedule(i);
                }
                if mode == Mode::Sticker
                    && icon_button(ui, Icon::Edit, "메모 편집 창 열기").clicked()
                {
                    w.edit();
                }
            }
            pin_button(ui, &mut self.data.notes[i].topmost);
            if mode == Mode::Sticker {
                let mut done = self.data.notes[i]
                    .done_on(self.extra.edit_occurrence.or(self.data.notes[i].date));
                let changed = ui
                    .add_sized([28., 28.], |ui: &mut egui::Ui| {
                        crate::ui::round_check(ui, &mut done, Some(INK.gamma_multiply(0.45)))
                    })
                    .on_hover_text("완료")
                    .changed();
                if changed {
                    self.done_at(
                        i,
                        self.extra.edit_occurrence.or(self.data.notes[i].date),
                        done,
                    );
                    self.extra.scope_owner = ctx.viewport_id();
                }
                if icon_button(ui, Icon::Fold, "제목 한 줄로 접기").clicked() {
                    w.fold();
                }
            }
            if engaged
                && mode == Mode::Editor
                && icon_button(
                    ui,
                    if maximized {
                        Icon::Restore
                    } else {
                        Icon::Maximize
                    },
                    if maximized {
                        "원래 크기로 복원"
                    } else {
                        "창 최대화"
                    },
                )
                .clicked()
            {
                w.native.cancel();
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            }
            if engaged && icon_button(ui, Icon::Close, "창 닫기 · 메모는 유지").clicked() {
                w.open = false;
            }
        });
    }
}
fn paper_style(ui: &mut egui::Ui, paper: Color32) {
    let v = ui.visuals_mut();
    v.dark_mode = false;
    v.override_text_color = Some(INK);
    v.window_fill = Color32::from_rgb(255, 254, 250);
    v.panel_fill = paper;
    v.extreme_bg_color = paper;
    v.selection.bg_fill = Color32::from_rgb(250, 225, 180);
    v.widgets.inactive.bg_fill = paper;
    v.widgets.inactive.weak_bg_fill = Color32::from_white_alpha(65);
    v.widgets.hovered.weak_bg_fill = Color32::from_white_alpha(120);
    v.window_stroke = egui::Stroke::new(1.0_f32, Color32::from_gray(215));
}
fn pin_button(ui: &mut egui::Ui, value: &mut bool) {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(28.), Sense::click());
    if *value || response.hovered() {
        ui.painter().rect_filled(
            rect,
            6.,
            Color32::from_black_alpha(if *value { 20 } else { 8 }),
        );
    }
    crate::icons::paint_icon(
        ui.painter(),
        rect.shrink(5.),
        Icon::Pin,
        if *value {
            INK
        } else {
            INK.gamma_multiply(0.35)
        },
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, *value, "항상 위에")
    });
    if response.on_hover_text("항상 위에 · Ctrl+T").clicked() {
        *value = !*value;
    }
}
fn translucent(c: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (alpha * 255.).round() as u8)
}
/// Resize the color box from its right and bottom edges. The window origin
/// stays fixed, so pointer positions stay stable while the size follows.
fn swatch_resize(ctx: &egui::Context, ui: &egui::Ui, w: &mut Window) {
    let Some(inner) = ctx.input(|v| v.viewport().inner_rect) else {
        return;
    };
    let size = inner.size();
    let (pointer, down, pressed, shift) = ctx.input(|v| {
        (
            v.pointer.latest_pos(),
            v.pointer.primary_down(),
            v.pointer.primary_pressed(),
            v.modifiers.shift,
        )
    });
    let band = (size.min_elem() / 3.).clamp(4., 10.);
    let edge = |p: Pos2| (p.x >= size.x - band, p.y >= size.y - band);
    if let Some(p) = pointer {
        let (right, bottom) = edge(p);
        if w.swatch_drag.is_some() || right || bottom {
            ctx.set_cursor_icon(match (right, bottom) {
                (true, true) => egui::CursorIcon::ResizeNwSe,
                (true, false) => egui::CursorIcon::ResizeHorizontal,
                _ => egui::CursorIcon::ResizeVertical,
            });
        }
        if pressed && (right || bottom) {
            let axes = Vec2::new(f32::from(u8::from(right)), f32::from(u8::from(bottom)));
            w.swatch_drag = Some((size, p, axes));
        }
    }
    let Some((start, origin, axes)) = w.swatch_drag else {
        return;
    };
    if let Some(p) = pointer {
        let delta = (p - origin) * axes;
        let mut next = (start + delta).clamp(Vec2::splat(16.), Vec2::splat(640.));
        if shift {
            next = Vec2::splat(next.x.max(next.y));
        }
        if (next - size).length() > 0.5 {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(next));
        }
        w.swatch_size = next;
    }
    if !down {
        w.swatch_drag = None;
    }
    ui.ctx().request_repaint();
}
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    Color32::from_rgb(
        (a.r() as f32 * (1. - t) + b.r() as f32 * t) as u8,
        (a.g() as f32 * (1. - t) + b.g() as f32 * t) as u8,
        (a.b() as f32 * (1. - t) + b.b() as f32 * t) as u8,
    )
}
pub fn nav_button(
    ui: &mut egui::Ui,
    text: &str,
    selected: bool,
    icon: Icon,
    badge: Option<String>,
    accent: bool,
) -> egui::Response {
    let tone = crate::ui::t(ui);
    let (rect, r) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.), Sense::click());
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, text));
    if selected || r.hovered() {
        ui.painter().rect_filled(
            rect,
            6.,
            if selected {
                tone.accent_soft
            } else {
                tone.hover
            },
        );
    }
    let color = if selected || accent { ACCENT } else { tone.ink };
    crate::icons::paint_icon(
        ui.painter(),
        Rect::from_center_size(
            egui::pos2(rect.left() + 17., rect.center().y),
            Vec2::splat(17.),
        ),
        icon,
        if selected || accent {
            ACCENT
        } else {
            tone.muted
        },
    );
    ui.painter().text(
        egui::pos2(rect.left() + 34., rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(13.5),
        color,
    );
    if let Some(b) = badge {
        ui.painter().text(
            egui::pos2(rect.right() - 10., rect.center().y),
            egui::Align2::RIGHT_CENTER,
            b,
            egui::FontId::proportional(12.),
            tone.faint,
        );
    }
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
}
/// Sidebar project row with a hash mark.
pub fn project_button(ui: &mut egui::Ui, name: &str, selected: bool) -> egui::Response {
    let tone = crate::ui::t(ui);
    let (rect, r) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.), Sense::click());
    if selected || r.hovered() {
        ui.painter().rect_filled(
            rect,
            6.,
            if selected {
                tone.accent_soft
            } else {
                tone.hover
            },
        );
    }
    ui.painter().text(
        egui::pos2(rect.left() + 17., rect.center().y),
        egui::Align2::CENTER_CENTER,
        "#",
        egui::FontId::proportional(14.),
        if selected { ACCENT } else { tone.faint },
    );
    let galley = ui.painter().layout(
        name.into(),
        egui::FontId::proportional(13.5),
        if selected { ACCENT } else { tone.ink },
        rect.width() - 44.,
    );
    ui.painter().galley(
        egui::pos2(rect.left() + 34., rect.center().y - galley.size().y / 2.),
        galley,
        tone.ink,
    );
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normal_fold_and_title_to_sticker_are_distinct() {
        let mut w = Window::new(Mode::Editor);
        w.fold();
        assert_eq!(w.mode, Mode::Title);
        w.fold();
        assert_eq!(w.mode, Mode::Swatch);
        w.fold();
        assert_eq!(w.mode, Mode::Editor);
        w.fold();
        w.sticker();
        assert_eq!(w.mode, Mode::Sticker);
        w.fold();
        assert_eq!(w.mode, Mode::Title);
        w.fold();
        assert_eq!(w.mode, Mode::Swatch);
        w.fold();
        assert_eq!(w.mode, Mode::Sticker);
    }
    #[test]
    fn explicit_editor_retains_sticker_restore_intent() {
        let mut w = Window::new(Mode::Sticker);
        w.skip_title = true;
        w.edit();
        assert_eq!(w.mode, Mode::Editor);
        w.fold();
        assert_eq!(w.mode, Mode::Swatch);
        w.fold();
        assert_eq!(w.mode, Mode::Sticker);
    }
    #[test]
    fn adding_stickers_keeps_existing_windows_and_notes_independent() {
        let mut a = App::sample(Local::now().date_naive());
        let old = a.data.notes.len();
        let first = a.new_sticker();
        let second = a.new_sticker();
        assert_ne!(viewport(first), viewport(second));
        a.data.notes[first].body = "첫 번째".into();
        a.data.notes[second].body = "두 번째".into();
        a.studio.windows.get_mut(&first).unwrap().fold();
        assert_eq!(a.studio.windows[&second].mode, Mode::Sticker);
        assert_eq!(a.data.notes.len(), old + 2);
        a.studio.windows.get_mut(&first).unwrap().open = false;
        assert_eq!(a.data.notes[first].body, "첫 번째");
        assert!(a.studio.windows[&second].open);
    }
    #[test]
    fn reopening_a_closed_viewport_reapplies_saved_size_without_resizing_live_windows() {
        let mut studio = Studio::default();
        studio.open(0, None);
        let w = studio.windows.get_mut(&0).unwrap();
        w.sticker();
        w.sticker_size = Vec2::new(410., 370.);
        w.applied = Some((Mode::Sticker, false));
        w.open = false;
        studio.open(0, None);
        let w = studio.windows.get_mut(&0).unwrap();
        w.sticker();
        assert!(w.applied.is_none());
        assert_eq!(w.size(), Vec2::new(410., 370.));
        w.applied = Some((Mode::Editor, false));
        studio.open(0, None);
        assert_eq!(studio.windows[&0].applied, Some((Mode::Editor, false)));
    }
    #[test]
    fn all_three_shortcuts_reject_duplicate_combinations() {
        let mut a = App::sample(Local::now().date_naive());
        a.shortcuts[2] = a.shortcuts[0].clone();
        a.apply_shortcuts();
        assert!(a.hotkey_status.contains("서로 다른"));
        a.shortcuts[2] = a.shortcuts[1].clone();
        a.apply_shortcuts();
        assert!(a.hotkey_status.contains("서로 다른"));
    }
    #[test]
    fn unchecked_single_note_cannot_expose_stale_completion_time() {
        let mut a = App::sample(Local::now().date_naive());
        a.data.notes[0].completed = Some(Local::now().naive_local());
        a.data.notes[0].done = false;
        assert!(a.editor_completion().is_none());
        a.data.notes[0].done = true;
        assert!(a.editor_completion().is_some());
    }
}
#[cfg(test)]
mod occurrence_window_tests {
    use super::*;
    #[test]
    fn open_editor_follows_rescheduled_period_occurrence() {
        let start = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
        let mut n = Note::new("기간", Some(start), 0);
        n.duration = 3;
        let selected = start + chrono::Duration::days(1);
        let mut w = Window::new(Mode::Editor);
        w.schedule_start = Some(start);
        w.occurrence = Some(selected);
        n.move_start(start + chrono::Duration::days(5));
        w.reconcile(&n);
        assert_eq!(w.occurrence, Some(selected + chrono::Duration::days(5)));
        n.duration = 1;
        w.reconcile(&n);
        assert_eq!(w.schedule_start, n.date);
    }
}
#[cfg(test)]
mod arrangement_tests {
    use super::*;
    #[test]
    fn daily_memos_roll_over_at_five_and_pick_open_past_and_today() {
        let at = |d: u32, h: u32, m: u32| {
            NaiveDate::from_ymd_opt(2026, 11, d)
                .unwrap()
                .and_hms_opt(h, m, 0)
                .unwrap()
        };
        let d = |day: u32| NaiveDate::from_ymd_opt(2026, 11, day).unwrap();
        assert_eq!(daily_day(at(12, 4, 59)), d(11));
        assert_eq!(daily_day(at(12, 5, 0)), d(12));
        let mut a = App::sample(d(11));
        let all = a.daily_due(d(12));
        // Sample memos of the 11th are still open; the 12th's memo is due.
        assert!(all.contains(&0) && all.contains(&2));
        a.done(0, true);
        assert!(!a.daily_due(d(12)).contains(&0));
        // Memos for later days are not shown early.
        assert!(!a.daily_due(d(11)).contains(&2));
    }
    #[test]
    fn submenus_open_outside_their_menu_right_first_then_left() {
        let area = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 800.));
        let size = Vec2::new(200., 150.);
        let parent = Rect::from_min_size(Pos2::new(100., 100.), Vec2::new(220., 300.));
        let p = submenu_pos(parent, size, 140., area);
        assert_eq!(p, Pos2::new(322., 140.));
        // Near the right edge it flips to the left, never overlapping.
        let parent = Rect::from_min_size(Pos2::new(700., 100.), Vec2::new(220., 300.));
        let p = submenu_pos(parent, size, 140., area);
        assert!(p.x + size.x <= parent.left());
        // A menu near the bottom is pulled up to stay visible.
        let p = submenu_pos(parent, size, 760., area);
        assert_eq!(p.y, 650.);
    }
    #[test]
    fn right_layout_stacks_from_top_right_and_wraps_into_a_new_column() {
        let area = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 300.));
        let sizes = [
            Vec2::new(200., 120.),
            Vec2::new(150., 120.),
            Vec2::new(180., 120.),
        ];
        let p = plan(Layout::Right, area, &sizes);
        assert_eq!(p[0], Pos2::new(1000. - 14. - 200., 14.));
        assert_eq!(p[1], Pos2::new(1000. - 14. - 150., 14. + 120. + 8.));
        // The third does not fit vertically and starts a column to the left.
        assert_eq!(p[2].y, 14.);
        assert!(p[2].x + 180. <= p[0].x);
    }
    #[test]
    fn left_and_cascade_layouts_stay_inside_the_work_area() {
        let area = Rect::from_min_size(Pos2::new(-1280., 0.), Vec2::new(1280., 700.));
        let sizes = vec![Vec2::splat(48.); 20];
        for layout in [Layout::Left, Layout::Cascade] {
            for (pos, size) in plan(layout, area, &sizes).into_iter().zip(&sizes) {
                assert!(
                    area.contains_rect(Rect::from_min_size(pos, *size)),
                    "{layout:?} {pos:?}"
                );
            }
        }
    }
    #[test]
    fn folding_arrangement_changes_every_open_sticker_and_places_it() {
        let mut a = App::sample(Local::now().date_naive());
        let first = a.new_sticker();
        let second = a.new_sticker();
        a.arrange = Some(Arrange {
            layout: Layout::Right,
            fold: Some(Mode::Swatch),
            area: Rect::from_min_size(Pos2::ZERO, Vec2::new(1600., 900.)),
        });
        a.apply_arrangement(&egui::Context::default());
        for i in [first, second] {
            let w = &a.studio.windows[&i];
            assert_eq!(w.mode, Mode::Swatch);
            assert!(w.place.is_some());
        }
        assert!(a.arrange.is_none());
    }
    fn menu_has_text(output: &egui::FullOutput, text: &str) -> bool {
        output
            .shapes
            .iter()
            .any(|shape| matches!(&shape.shape, egui::Shape::Text(t) if t.galley.job.text == text))
    }
    #[test]
    fn arrange_submenu_omits_the_close_actions_the_context_menu_still_has() {
        let mut a = App::sample(Local::now().date_naive());
        let mut w = Window::new(Mode::Editor);
        let ctx = egui::Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(800., 600.));
        let input = || egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        // The sticker's own right-click menu (`own = true`) still offers to
        // close it.
        let full = ctx.run(input(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                a.sticker_menu(ui, 0, &mut w, true);
            });
        });
        assert!(menu_has_text(&full, "이 스티커 닫기"));
        assert!(menu_has_text(&full, "스티커 정렬"));
        // "더보기 › 스티커 정렬" (`own = false`) is specifically about
        // arranging stickers; it must not repeat the close actions the
        // context menu already has.
        let sub = ctx.run(input(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                a.sticker_menu(ui, 0, &mut w, false);
            });
        });
        assert!(!menu_has_text(&sub, "이 스티커 닫기"));
        assert!(!menu_has_text(&sub, "모든 스티커 닫기  Ctrl+Shift+H"));
        assert!(menu_has_text(&sub, "스티커 정렬"));
        assert!(menu_has_text(&sub, "화면 오른쪽에 정렬"));
    }
}
