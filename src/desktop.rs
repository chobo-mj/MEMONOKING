use crate::*;
use chrono::{Duration, NaiveDateTime};
use std::collections::BTreeMap;

pub struct Desktop {
    pub settings_tab: usize,
    pub hide_completed: bool,
    pub hide_locked: bool,
    pub show_preview: bool,
    pub auto_toolbar: bool,
    pub skip_title: bool,
    pub unpin_minimized: bool,
    pub default_color: Color32,
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
    pub deleted_at: BTreeMap<usize, NaiveDateTime>,
    pub passwords: BTreeMap<usize, String>,
    /// When each password memo was last unlocked (for automatic re-locking).
    pub unlocked_at: BTreeMap<usize, std::time::Instant>,
    pub lock_target: Option<(usize, bool, bool)>,
    pub lock_input: String,
    pub lock_confirm: String,
    pub lock_error: String,
    pub reminder_target: Option<usize>,
    pub reminder_date: NaiveDate,
    pub reminder_hour: u32,
    pub reminder_minute: u32,
    pub reminder_month: NaiveDate,
    pub purge_target: Option<usize>,
    pub project_popup: bool,
    pub last_escape: std::collections::HashMap<egui::ViewportId, f64>,
    pub last_action: String,
    pub focus_dialog: bool,
    pub schedule: Option<crate::schedule::ScheduleEdit>,
    /// Open memos as stickers; the editor is entered from the sticker's edit button.
    pub always_sticker: bool,
    /// Size of newly opened stickers.
    pub sticker_size: Vec2,
    /// Every day (from 05:00) open unfinished past and today's memos as
    /// titles arranged on the right.
    pub daily_today: bool,
    pub daily_shown: Option<NaiveDate>,
    /// Monitor (index into `native_window::monitors`) used for arranging.
    pub arrange_monitor: Option<usize>,
    pub monitor_pick: bool,
}
impl Default for Desktop {
    fn default() -> Self {
        let today = Local::now().date_naive();
        Self {
            settings_tab: 0,
            hide_completed: true,
            hide_locked: false,
            show_preview: false,
            auto_toolbar: true,
            skip_title: false,
            unpin_minimized: true,
            default_color: Color32::from_rgb(255, 240, 166),
            font: "맑은 고딕".into(),
            note_font: "맑은 고딕".into(),
            font_size: 10.,
            note_font_size: 10.,
            letter_spacing: 0.,
            row_spacing: 1,
            opacity: 100.,
            image_scale: 3,
            autostart: false,
            taskbar: true,
            retention: 1,
            auto_lock: 1,
            sound: true,
            popup: true,
            mute: false,
            calendar_basis: 1,
            deleted_at: BTreeMap::new(),
            passwords: BTreeMap::new(),
            unlocked_at: BTreeMap::new(),
            lock_target: None,
            lock_input: String::new(),
            lock_confirm: String::new(),
            lock_error: String::new(),
            reminder_target: None,
            reminder_date: today,
            reminder_hour: 9,
            reminder_minute: 0,
            reminder_month: today.with_day(1).unwrap(),
            purge_target: None,
            project_popup: false,
            last_escape: Default::default(),
            last_action: String::new(),
            focus_dialog: false,
            schedule: None,
            always_sticker: true,
            sticker_size: Vec2::splat(360.),
            daily_today: false,
            daily_shown: None,
            arrange_monitor: None,
            monitor_pick: false,
        }
    }
}
pub fn family(ui: &egui::Ui, name: &str) -> egui::FontFamily {
    let key = egui::FontFamily::Name(name.into());
    if ui.fonts(|f| f.families().contains(&key)) {
        key
    } else {
        egui::FontFamily::Proportional
    }
}
pub fn infer_title(title: &mut String, body: &mut String) {
    if !title.trim().is_empty() {
        return;
    }
    // The first line with real text becomes the title. Inline pictures on
    // that line stay in the body; a picture-only line never becomes a title.
    if let Some((offset, line, text)) = body
        .split_inclusive('\n')
        .scan(0usize, |pos, line| {
            let old = *pos;
            *pos += line.len();
            Some((old, line))
        })
        .map(|(offset, line)| (offset, line, crate::composer::strip_pictures(line)))
        .find(|(_, _, text)| !text.trim().is_empty() && !text.trim_start().starts_with('#'))
    {
        *title = text.trim().into();
        let pictures: String = line
            .chars()
            .filter(|c| crate::composer::is_picture(*c))
            .collect();
        let keep = if pictures.is_empty() {
            String::new()
        } else if line.ends_with('\n') {
            format!("{pictures}\n")
        } else {
            pictures
        };
        body.replace_range(offset..offset + line.len(), &keep);
    }
}
impl App {
    pub fn open_sticker(&mut self, i: usize, day: Option<NaiveDate>) {
        if self.data.notes[i].locked {
            self.ask_unlock(i, true);
            return;
        }
        self.open_note_on(i, day);
        if let Some(w) = self.studio.windows.get_mut(&i) {
            w.sticker();
        }
    }
    pub fn toggle_sticker(&mut self, i: usize) {
        if let Some(w) = self.studio.windows.get_mut(&i) {
            if w.open {
                w.open = false;
                return;
            }
        }
        self.open_sticker(i, self.data.notes[i].date);
    }
    pub fn inline_note(&mut self, ui: &mut egui::Ui, i: usize) {
        if self.data.notes[i].locked {
            ui.vertical_centered(|ui| {
                ui.add_space(36.);
                ui.heading("잠긴 메모");
                ui.label("제목만 표시합니다.");
                if ui.button("잠금 해제").clicked() {
                    self.ask_unlock(i, false);
                }
            });
            return;
        }
        let day = if self.catalog.selected == Some(i) {
            self.catalog.selected_day
        } else {
            None
        };
        let mut done = self.data.notes[i].done_on(day);
        let mut done_changed = false;
        let mut action = None;
        let fill = self.data.notes[i].color;
        egui::Frame::NONE
            .fill(fill)
            .corner_radius(18.)
            .inner_margin(20.)
            .show(ui, |ui| {
                ui.visuals_mut().override_text_color = Some(INK);
                let n = &mut self.data.notes[i];
                ui.horizontal_top(|ui| {
                    if n.starred {
                        ui.label(RichText::new("★").color(crate::studio::STAR).size(20.));
                    }
                    ui.add(
                        egui::TextEdit::multiline(&mut n.title)
                            .font(egui::FontId::proportional(22.))
                            .desired_rows(1)
                            .desired_width((ui.available_width() - 34.).max(60.))
                            .frame(false)
                            .hint_text("제목"),
                    );
                    ui.add_space((ui.available_width() - 24.).max(0.));
                    done_changed = ui::round_check(ui, &mut done, None)
                        .on_hover_text("완료")
                        .changed();
                });
                ui.add_space(8.);
                ui.separator();
                ui.add_space(8.);
                if self
                    .catalog
                    .find_selection
                    .as_ref()
                    .is_some_and(|(id, _, _)| *id == i)
                {
                    if let Some((_, tab, range)) = self.catalog.find_selection.take() {
                        n.document.requested_tab = Some(tab);
                        n.document.requested_selection = Some(range);
                    }
                }
                n.document.toolbar_visible = true;
                let tools = 4. * 30.;
                crate::composer::show_with(ui, &mut n.body, &mut n.document, ("inline", i), tools);
                // Memo actions sit at the right end of the tool row.
                let area = ui.min_rect();
                let rect = egui::Rect::from_min_max(
                    egui::pos2(area.right() - tools, area.bottom() - 28.),
                    area.right_bottom(),
                );
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.;
                        if icons::icon_button(ui, icons::Icon::Alarm, "알람").clicked() {
                            action = Some(0);
                        }
                        if icons::icon_button(ui, icons::Icon::Calendar, "일정").clicked() {
                            action = Some(1);
                        }
                        let project =
                            icons::icon_button(ui, icons::Icon::ChevronDown, "프로젝트 선택");
                        let popup = ui.make_persistent_id(("inline_project", i));
                        if project.clicked() {
                            ui.memory_mut(|m| m.toggle_popup(popup));
                        }
                        egui::popup_above_or_below_widget(
                            ui,
                            popup,
                            &project,
                            egui::AboveOrBelow::Above,
                            egui::PopupCloseBehavior::CloseOnClickOutside,
                            |ui| {
                                ui.set_min_width(200.);
                                if self.project_items(ui, i) {
                                    ui.memory_mut(|m| m.close_popup());
                                }
                            },
                        );
                        let linked =
                            icons::icon_button(ui, icons::Icon::ChevronRight, "연결된 메모");
                        let popup = ui.make_persistent_id(("inline_linked", i));
                        if linked.clicked() {
                            ui.memory_mut(|m| m.toggle_popup(popup));
                        }
                        egui::popup_above_or_below_widget(
                            ui,
                            popup,
                            &linked,
                            egui::AboveOrBelow::Above,
                            egui::PopupCloseBehavior::CloseOnClickOutside,
                            |ui| {
                                ui.set_min_width(240.);
                                if self.linked_items(ui, i) {
                                    ui.memory_mut(|m| m.close_popup());
                                }
                            },
                        );
                    });
                });
            });
        if done_changed {
            self.done_at(i, day, done);
        }
        self.consume_body_project(i);
        match action {
            Some(0) => self.open_reminder(i),
            Some(1) => self.open_schedule(i),
            _ => {}
        }
    }
    pub fn consume_body_project(&mut self, i: usize) {
        let first = self.data.notes[i].body.clone();
        let Some(rest) = first.strip_prefix('#') else {
            return;
        };
        let Some(end) = rest.find(char::is_whitespace) else {
            return;
        };
        let name = rest[..end].trim().to_owned();
        if name.is_empty() {
            return;
        }
        let p = self
            .data
            .projects
            .iter()
            .enumerate()
            .find(|(p, title)| {
                !self.extra.deleted_projects.contains(p)
                    && !self.extra.hidden_projects.contains(p)
                    && *title == &name
            })
            .map(|(p, _)| p)
            .unwrap_or_else(|| {
                self.data.projects.push(name.clone());
                self.data.projects.len() - 1
            });
        self.data.notes[i].project = Some(p);
        let remove = 1 + end + rest[end..].chars().next().unwrap().len_utf8();
        self.data.notes[i].body.replace_range(..remove, "");
        self.toast = format!("#{name} 프로젝트에 추가했습니다");
    }
    /// Pick a backed-up `data.json` and load it, after backing up the
    /// current data.
    pub fn restore_from_backup(&mut self, dir: &std::path::Path) {
        let Some(file) = rfd::FileDialog::new()
            .add_filter("OmniMemoJang 데이터", &["db", "json"])
            .set_directory(dir.join("backups"))
            .pick_file()
        else {
            return;
        };
        let Some(from) = file.parent() else {
            return;
        };
        self.autosave(0., true);
        let _ = crate::storage::backup(dir);
        match crate::storage::read_file(&file) {
            Ok(saved) => {
                // Backups from older versions carry their own pictures.
                crate::storage::copy_missing(&from.join("images"), &dir.join("images"));
                self.restore(&saved);
                self.saving.rewrite();
                self.autosave(0., true);
                self.desktop.last_action = format!("복원했습니다: {}", file.display());
            }
            Err(e) => self.desktop.last_action = format!("복원하지 못했습니다: {e}"),
        }
    }
    /// Lock password memos again 1, 5 or 15 minutes after they were
    /// unlocked (the last choice keeps them open until the app restarts).
    pub fn auto_relock(&mut self) {
        let Some(minutes) = [1u64, 5, 15].get(self.desktop.auto_lock).copied() else {
            return;
        };
        let limit = std::time::Duration::from_secs(minutes * 60);
        let expired: Vec<usize> = self
            .desktop
            .unlocked_at
            .iter()
            .filter(|(_, at)| at.elapsed() >= limit)
            .map(|(i, _)| *i)
            .collect();
        for i in expired {
            self.desktop.unlocked_at.remove(&i);
            if self.desktop.passwords.contains_key(&i) {
                if let Some(n) = self.data.notes.get_mut(i) {
                    n.locked = true;
                }
            }
        }
    }
    /// Erase a memo's content for good (it stays as an empty tombstone so
    /// memo indexes keep their meaning).
    pub fn purge_note(&mut self, i: usize) {
        let n = &mut self.data.notes[i];
        n.body.clear();
        n.title = "삭제된 메모".into();
        n.document = Default::default();
        n.wiki_comments.clear();
        n.journal_comments.clear();
        n.occurrence_comments.clear();
        n.reminder = None;
        n.purged = true;
        n.deleted = true;
        self.desktop.passwords.remove(&i);
        self.desktop.deleted_at.remove(&i);
    }
    /// Erase memos that sat in the trash longer than the retention setting
    /// (7, 30, 90 or 365 days; the last choice keeps them).
    pub fn purge_expired_trash(&mut self, now: NaiveDateTime) {
        let Some(days) = [7, 30, 90, 365].get(self.desktop.retention).copied() else {
            return;
        };
        let expired: Vec<usize> = self
            .desktop
            .deleted_at
            .iter()
            .filter(|(i, at)| {
                now - **at > Duration::days(days)
                    && self
                        .data
                        .notes
                        .get(**i)
                        .is_some_and(|n| n.deleted && !n.purged)
            })
            .map(|(i, _)| *i)
            .collect();
        for i in expired {
            self.purge_note(i);
        }
    }
    pub fn trash_note(&mut self, i: usize) {
        self.data.notes[i].deleted = true;
        self.desktop
            .deleted_at
            .insert(i, Local::now().naive_local());
        if let Some(w) = self.studio.windows.get_mut(&i) {
            w.open = false;
        }
        self.toast = "휴지통으로 옮겼습니다".into();
    }
    pub fn restore_note(&mut self, i: usize) {
        if !self.data.notes[i].purged {
            self.data.notes[i].deleted = false;
            self.desktop.deleted_at.remove(&i);
        }
    }
    pub fn request_purge(&mut self, i: usize) {
        self.desktop.purge_target = Some(i);
        self.desktop.focus_dialog = true;
    }
    pub fn duplicate_note(&mut self, i: usize) {
        if self.data.notes[i].locked {
            self.ask_unlock(i, false);
            return;
        }
        let mut n = self.data.notes[i].clone();
        n.title = format!("{} 사본", n.title);
        n.created = Local::now().naive_local();
        n.done = false;
        n.completed = None;
        n.occurrence_completed.clear();
        n.occurrence_comments.clear();
        n.journal_comments.clear();
        n.locked = false;
        n.deleted = false;
        n.purged = false;
        let id = self.data.notes.len();
        self.data.notes.push(n);
        self.open_sticker(id, self.data.notes[id].date);
    }
    pub fn ask_unlock(&mut self, i: usize, sticker: bool) {
        self.desktop.focus_dialog = true;
        self.desktop.lock_target = Some((i, false, sticker));
        self.desktop.lock_input.clear();
        self.desktop.lock_confirm.clear();
        self.desktop.lock_error.clear();
    }
    pub fn ask_lock(&mut self, i: usize) {
        self.desktop.focus_dialog = true;
        self.desktop.lock_target = Some((i, true, false));
        self.desktop.lock_input.clear();
        self.desktop.lock_confirm.clear();
        self.desktop.lock_error.clear();
    }
    pub fn open_reminder(&mut self, i: usize) {
        self.desktop.focus_dialog = true;
        use chrono::Timelike;
        self.desktop.reminder_target = Some(i);
        let t = self.data.notes[i]
            .reminder
            .unwrap_or_else(|| Local::now().naive_local() + Duration::hours(1));
        self.desktop.reminder_date = t.date();
        self.desktop.reminder_month = t.date().with_day(1).unwrap();
        self.desktop.reminder_hour = t.hour();
        self.desktop.reminder_minute = t.minute();
    }
    pub fn desktop_dialogs(&mut self, ctx: &egui::Context) {
        if std::mem::take(&mut self.desktop.focus_dialog) {
            for id in [
                "project_popup",
                "lock_dialog",
                "reminder_dialog",
                "purge_dialog",
                "schedule_dialog",
            ] {
                crate::ui::raise(ctx, id);
            }
        }
        if self.desktop.project_popup {
            let mut open = true;
            crate::ui::dialog(
                ctx,
                "project_popup",
                "프로젝트 선택",
                [300., 380.],
                true,
                &mut open,
                |ui| {
                    crate::ui::caption(ui, "프로젝트로 필터");
                    ui.add_space(4.);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let mut pick = None;
                        if crate::studio::project_button(
                            ui,
                            "모든 프로젝트",
                            self.project_filter.is_none(),
                        )
                        .clicked()
                        {
                            pick = Some(None);
                        }
                        if crate::studio::project_button(
                            ui,
                            "프로젝트 없음",
                            self.project_filter == Some(NO_PROJECT),
                        )
                        .clicked()
                        {
                            pick = Some(Some(NO_PROJECT));
                        }
                        for (i, name) in self.data.projects.iter().enumerate() {
                            if self.extra.hidden_projects.contains(&i)
                                || self.extra.deleted_projects.contains(&i)
                            {
                                continue;
                            }
                            if crate::studio::project_button(
                                ui,
                                name,
                                self.project_filter == Some(i),
                            )
                            .clicked()
                            {
                                pick = Some(Some(i));
                            }
                        }
                        if let Some(p) = pick {
                            self.project_filter = p;
                            self.page = 0;
                            self.desktop.project_popup = false;
                        }
                    });
                },
            );
            self.desktop.project_popup &= open;
        }
        if let Some((i, set, sticker)) = self.desktop.lock_target {
            let mut open = true;
            let mut apply = false;
            crate::ui::dialog(
                ctx,
                "lock_dialog",
                if set {
                    "메모 잠금"
                } else {
                    "잠금 해제"
                },
                [360., 280.],
                true,
                &mut open,
                |ui| {
                    ui.horizontal(|ui| {
                        crate::icons::paint_icon(
                            ui.painter(),
                            egui::Rect::from_min_size(ui.cursor().min, Vec2::splat(20.)),
                            crate::icons::Icon::Lock,
                            ACCENT,
                        );
                        ui.add_space(26.);
                        ui.label(RichText::new(&self.data.notes[i].title).strong().size(15.));
                    });
                    crate::ui::hint(
                        ui,
                        "비밀번호는 해시로만 저장합니다 · 본문은 암호화하지 않습니다",
                    );
                    ui.add_space(10.);
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut self.desktop.lock_input)
                            .password(true)
                            .desired_width(f32::INFINITY)
                            .margin(egui::Margin::symmetric(10, 7))
                            .hint_text("비밀번호"),
                    );
                    if self.desktop.lock_input.is_empty() && !r.has_focus() {
                        r.request_focus();
                    }
                    if set {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.desktop.lock_confirm)
                                .password(true)
                                .desired_width(f32::INFINITY)
                                .margin(egui::Margin::symmetric(10, 7))
                                .hint_text("비밀번호 확인"),
                        );
                    }
                    if !self.desktop.lock_error.is_empty() {
                        ui.colored_label(ACCENT, &self.desktop.lock_error);
                    }
                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let (ok, cancel) =
                        crate::ui::dialog_buttons(ui, if set { "잠그기" } else { "열기" }, "취소");
                    apply = ok || enter;
                    if cancel {
                        self.desktop.lock_target = None;
                    }
                },
            );
            if apply {
                let valid = if set {
                    !self.desktop.lock_input.is_empty()
                        && self.desktop.lock_input == self.desktop.lock_confirm
                } else {
                    self.desktop.passwords.get(&i).is_some_and(|h| {
                        crate::storage::check_password(h, &self.desktop.lock_input)
                    })
                };
                if valid {
                    if set {
                        self.desktop
                            .passwords
                            .insert(i, crate::storage::hash_password(&self.desktop.lock_input));
                        self.data.notes[i].locked = true;
                    } else {
                        self.data.notes[i].locked = false;
                        self.desktop
                            .unlocked_at
                            .insert(i, std::time::Instant::now());
                        if sticker {
                            self.open_sticker(i, self.data.notes[i].date);
                        } else {
                            self.open_note(i);
                        }
                    }
                    open = false;
                } else {
                    self.desktop.lock_error = if set {
                        "같은 비밀번호를 두 번 입력하세요"
                    } else {
                        "비밀번호를 확인하세요"
                    }
                    .into();
                }
            }
            if !open || self.desktop.lock_target.is_none() {
                self.desktop.lock_target = None;
                self.desktop.lock_input.clear();
                self.desktop.lock_confirm.clear();
            }
        }
        if let Some(i) = self.desktop.reminder_target {
            let mut open = true;
            let mut applied = false;
            let mut cleared = false;
            crate::ui::dialog(
                ctx,
                "reminder_dialog",
                "알람 설정",
                [340., 470.],
                true,
                &mut open,
                |ui| {
                    ui.horizontal(|ui| {
                        crate::icons::paint_icon(
                            ui.painter(),
                            egui::Rect::from_min_size(ui.cursor().min, Vec2::splat(20.)),
                            crate::icons::Icon::Alarm,
                            ACCENT,
                        );
                        ui.add_space(26.);
                        ui.label(
                            RichText::new(if self.data.notes[i].title.is_empty() {
                                "새 메모"
                            } else {
                                &self.data.notes[i].title
                            })
                            .strong()
                            .size(15.),
                        );
                    });
                    ui.add_space(8.);
                    ui.horizontal_wrapped(|ui| {
                        let now = Local::now().naive_local();
                        for (label, t) in [
                            ("+10분", now + Duration::minutes(10)),
                            ("+1시간", now + Duration::hours(1)),
                            ("오늘 18:00", now.date().and_hms_opt(18, 0, 0).unwrap()),
                            (
                                "내일 09:00",
                                (now.date() + Duration::days(1))
                                    .and_hms_opt(9, 0, 0)
                                    .unwrap(),
                            ),
                        ] {
                            if crate::ui::pill(ui, false, label).clicked() {
                                use chrono::Timelike;
                                self.desktop.reminder_date = t.date();
                                self.desktop.reminder_month = t.date().with_day(1).unwrap();
                                self.desktop.reminder_hour = t.hour();
                                self.desktop.reminder_minute = t.minute();
                            }
                        }
                    });
                    ui.add_space(10.);
                    crate::ui::month_grid(
                        ui,
                        &mut self.desktop.reminder_month,
                        &mut self.desktop.reminder_date,
                    );
                    ui.add_space(6.);
                    crate::ui::time_row(
                        ui,
                        &mut self.desktop.reminder_hour,
                        &mut self.desktop.reminder_minute,
                    );
                    crate::ui::hint(
                        ui,
                        "시각이 되면 소리와 함께 화면 오른쪽 아래에 알림 창이 뜹니다",
                    );
                    ui.add_space(4.);
                    let mut out = (false, false);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        out.0 = crate::ui::primary(ui, "설정").clicked();
                        if self.data.notes[i].reminder.is_some() {
                            out.1 = ui.button("알람 해제").clicked();
                        }
                    });
                    (applied, cleared) = out;
                },
            );
            if applied {
                self.data.notes[i].reminder = self.desktop.reminder_date.and_hms_opt(
                    self.desktop.reminder_hour,
                    self.desktop.reminder_minute,
                    0,
                );
                open = false;
            }
            if cleared {
                self.data.notes[i].reminder = None;
                open = false;
            }
            if !open {
                self.desktop.reminder_target = None;
            }
        }
        self.schedule_dialog(ctx);
        if let Some(i) = self.desktop.purge_target {
            let mut open = true;
            let mut purge = false;
            crate::ui::dialog(
                ctx,
                "purge_dialog",
                "메모 영구삭제",
                [380., 200.],
                true,
                &mut open,
                |ui| {
                    ui.label(
                        RichText::new(format!(
                            "‘{}’을(를) 영구삭제할까요?",
                            self.data.notes[i].title
                        ))
                        .strong()
                        .size(15.),
                    );
                    ui.add_space(4.);
                    crate::ui::hint(ui, "본문과 댓글이 모두 삭제되며 되돌릴 수 없습니다.");
                    let (ok, cancel) = crate::ui::dialog_buttons(ui, "영구삭제", "취소");
                    purge = ok;
                    if cancel {
                        self.desktop.purge_target = None;
                    }
                },
            );
            if purge {
                self.purge_note(i);
                self.desktop.purge_target = None;
            }
            if !open {
                self.desktop.purge_target = None;
            }
        }
    }
    pub fn ppt_settings(&mut self, ui: &mut egui::Ui) {
        let tone = crate::ui::t(ui);
        let tabs = [
            "일반",
            "메인창",
            "메모창",
            "캘린더",
            "알람",
            "단축키",
            "백업 / 복원",
            "정보",
        ];
        let icons = [
            crate::icons::Icon::Settings,
            crate::icons::Icon::Home,
            crate::icons::Icon::Sticky,
            crate::icons::Icon::Calendar,
            crate::icons::Icon::Alarm,
            crate::icons::Icon::Grid,
            crate::icons::Icon::Folder,
            crate::icons::Icon::Help,
        ];
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(150.);
                ui.label(RichText::new("설정").size(18.).strong());
                ui.add_space(12.);
                for (i, text) in tabs.iter().enumerate() {
                    if crate::studio::nav_button(
                        ui,
                        text,
                        self.desktop.settings_tab == i,
                        icons[i],
                        None,
                        false,
                    )
                    .clicked()
                    {
                        self.desktop.settings_tab = i;
                        self.desktop.last_action.clear();
                    }
                }
            });
            ui.add_space(14.);
            let (line, _) =
                ui.allocate_exact_size(Vec2::new(1., ui.available_height()), egui::Sense::hover());
            ui.painter().line_segment(
                [line.center_top(), line.center_bottom()],
                egui::Stroke::new(1.0_f32, tone.line),
            );
            ui.add_space(18.);
            egui::ScrollArea::vertical()
                .id_salt("settings_content")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 12.).clamp(320., 600.));
                        ui.label(
                            RichText::new(tabs[self.desktop.settings_tab.min(tabs.len() - 1)])
                                .size(17.)
                                .strong(),
                        );
                        ui.add_space(14.);
                        self.settings_page(ui);
                        if !self.desktop.last_action.is_empty() {
                            ui.add_space(12.);
                            ui.colored_label(ACCENT, &self.desktop.last_action);
                        }
                    });
                });
        });
    }
    fn settings_page(&mut self, ui: &mut egui::Ui) {
        use crate::ui::{caption, hint, segmented, toggle_row};
        let section = |ui: &mut egui::Ui, title: &str| {
            ui.add_space(10.);
            caption(ui, title);
            ui.add_space(4.);
        };
        match self.desktop.settings_tab {
            0 => {
                toggle_row(
                    ui,
                    &mut self.desktop.autostart,
                    "Windows 시작 시 자동 실행",
                    "트레이에서 시작하고, 열어 둔 스티커 메모가 제자리에 다시 뜹니다",
                );
                toggle_row(
                    ui,
                    &mut self.desktop.taskbar,
                    "메모창을 작업표시줄에 표시",
                    "끄면 새로 여는 스티커 메모 창이 작업표시줄에 나타나지 않습니다",
                );
                section(ui, "새 스티커 메모 크기");
                let mut preset = match self.desktop.sticker_size {
                    s if s == Vec2::new(340., 240.) => 0,
                    s if s == Vec2::splat(360.) => 1,
                    s if s == Vec2::new(460., 440.) => 2,
                    _ => 3,
                };
                if segmented(ui, &mut preset, &["작게", "보통", "크게", "직접 설정"]) {
                    match preset {
                        0 => self.desktop.sticker_size = Vec2::new(340., 240.),
                        1 => self.desktop.sticker_size = Vec2::splat(360.),
                        2 => self.desktop.sticker_size = Vec2::new(460., 440.),
                        _ => {}
                    }
                }
                ui.horizontal(|ui| {
                    ui.label("너비");
                    ui.add(
                        egui::Slider::new(&mut self.desktop.sticker_size.x, 340.0..=900.0)
                            .step_by(10.)
                            .suffix(" px"),
                    );
                    ui.add_space(12.);
                    ui.label("높이");
                    ui.add(
                        egui::Slider::new(&mut self.desktop.sticker_size.y, 220.0..=900.0)
                            .step_by(10.)
                            .suffix(" px"),
                    );
                });
                hint(ui, "새로 여는 스티커 메모에 적용됩니다");
                ui.add_space(6.);
                if toggle_row(
                    ui,
                    &mut self.desktop.daily_today,
                    "매일 오늘 메모 자동으로 띄우기",
                    "매일 새벽 5시 기준으로 완료하지 않은 지난 메모와 오늘 메모를 제목만 보기로 화면 오른쪽에 정렬합니다",
                )
                .changed()
                    && self.desktop.daily_today
                {
                    self.desktop.daily_shown = None;
                }
                let monitors = crate::native_window::monitors(ui.ctx()).len();
                let mut pick = self.desktop.arrange_monitor.is_some() || self.desktop.monitor_pick;
                ui.add_enabled_ui(monitors >= 2, |ui| {
                    if toggle_row(
                        ui,
                        &mut pick,
                        "메모 정렬 모니터 정하기",
                        &match self.desktop.arrange_monitor {
                            Some(m) => format!("스티커 자동 정렬을 모니터 {}에 합니다", m + 1),
                            None if monitors >= 2 => {
                                "켜면 각 모니터에 번호가 뜹니다 · 정렬에 쓸 모니터의 번호를 누르세요".into()
                            }
                            None => "모니터가 두 대 이상일 때 사용할 수 있습니다".into(),
                        },
                    )
                    .changed()
                    {
                        self.desktop.monitor_pick = pick;
                        if !pick {
                            self.desktop.arrange_monitor = None;
                        }
                    }
                });
                section(ui, "첫 화면");
                ui.horizontal_wrapped(|ui| {
                    for (page, name, _) in crate::PAGES {
                        if self.prefs.hidden_pages.contains(&page) {
                            continue;
                        }
                        if crate::ui::pill(ui, self.prefs.start_page == page, name).clicked() {
                            self.prefs.start_page = page;
                        }
                    }
                });
                hint(ui, "프로그램을 켰을 때 처음 보여 줄 메뉴입니다");
                section(ui, "메뉴 숨기기");
                ui.horizontal_wrapped(|ui| {
                    for (page, name, _) in crate::PAGES.into_iter().filter(|p| p.0 != 0) {
                        let mut hidden = self.prefs.hidden_pages.contains(&page);
                        if ui.checkbox(&mut hidden, name).changed() {
                            if hidden {
                                self.prefs.hidden_pages.insert(page);
                                if self.prefs.start_page == page {
                                    self.prefs.start_page = 0;
                                }
                            } else {
                                self.prefs.hidden_pages.remove(&page);
                            }
                        }
                    }
                });
                hint(
                    ui,
                    "체크한 메뉴를 왼쪽 메뉴창에서 숨깁니다 · 내 작업실은 숨길 수 없습니다",
                );
                section(ui, "휴지통 보관 기간");
                segmented(
                    ui,
                    &mut self.desktop.retention,
                    &["7일", "30일", "90일", "365일", "자동삭제 안 함"],
                );
                section(ui, "잠긴 메모 자동 잠금");
                segmented(
                    ui,
                    &mut self.desktop.auto_lock,
                    &["1분", "5분", "15분", "Windows 잠금 때만"],
                );
                hint(
                    ui,
                    "휴지통은 프로그램을 켤 때 정리합니다 · 잠긴 메모는 연 뒤 정한 시간이 지나면 다시 잠깁니다",
                );
            }
            1 => {
                section(ui, "테마");
                let mut theme = usize::from(self.dark);
                if segmented(ui, &mut theme, &["라이트 모드", "다크 모드"]) {
                    self.dark = theme == 1;
                }
                section(ui, "글꼴");
                font_choice(ui, "main_font", &mut self.desktop.font);
                section(ui, "글꼴 크기");
                ui.add(
                    egui::Slider::new(&mut self.desktop.font_size, 8.0..=16.0)
                        .integer()
                        .suffix(" pt"),
                );
                section(ui, "자간");
                let steps = [-0.5, 0., 0.5, 1.0];
                let mut at = steps
                    .iter()
                    .position(|v| (*v - self.desktop.letter_spacing).abs() < 0.01)
                    .unwrap_or(1);
                if segmented(ui, &mut at, &["-0.5", "기본", "+0.5", "+1.0"]) {
                    self.desktop.letter_spacing = steps[at];
                }
                section(ui, "메모 목록 행 간격");
                segmented(ui, &mut self.desktop.row_spacing, &["좁게", "기본", "넓게"]);
                section(ui, "내 작업실");
                toggle_row(
                    ui,
                    &mut self.desktop.show_preview,
                    "캘린더와 함께 볼 때 본문 한 줄 표시",
                    "",
                );
                toggle_row(ui, &mut self.desktop.hide_locked, "잠긴 메모 숨기기", "");
                toggle_row(ui, &mut self.desktop.hide_completed, "완료 메모 숨기기", "");
            }
            2 => {
                toggle_row(
                    ui,
                    &mut self.desktop.always_sticker,
                    "메모를 항상 스티커 메모로 열기",
                    "메모 편집 창은 스티커 제목줄의 편집 버튼으로 엽니다",
                );
                section(ui, "새 메모 기본 색");
                color_choices(ui, &mut self.desktop.default_color);
                ui.add_space(6.);
                toggle_row(
                    ui,
                    &mut self.desktop.auto_toolbar,
                    "스티커 툴바 자동 숨기기",
                    "끄면 편집 중이 아니어도 도구가 항상 보입니다",
                );
                toggle_row(
                    ui,
                    &mut self.desktop.skip_title,
                    "접기 1단계 건너뛰기",
                    "스티커에서 바로 색상 상자로 접습니다",
                );
                toggle_row(
                    ui,
                    &mut self.desktop.unpin_minimized,
                    "상자로 접으면 ‘항상 위에’ 자동 해제",
                    "",
                );
                section(ui, "기본 글꼴");
                font_choice(ui, "note_font", &mut self.desktop.note_font);
                section(ui, "기본 글자 크기");
                ui.add(
                    egui::Slider::new(&mut self.desktop.note_font_size, 8.0..=30.0)
                        .integer()
                        .suffix(" pt"),
                );
                section(ui, "기본 투명도");
                ui.add(
                    egui::Slider::new(&mut self.desktop.opacity, 20.0..=100.0)
                        .integer()
                        .suffix("%"),
                );
                hint(ui, "투명도를 낮추면 스티커 뒤의 바탕화면이 비쳐 보입니다.");
                section(ui, "이미지 붙여넣기 기본 크기");
                segmented(
                    ui,
                    &mut self.desktop.image_scale,
                    &["25%", "50%", "75%", "100%", "너비 맞춤"],
                );
            }
            3 => {
                section(ui, "테마 (분리 캘린더 · 스탠딩 캘린더)");
                ui.horizontal_wrapped(|ui| {
                    let names = ["기본", "종이", "세이지", "나이트"];
                    for (k, name) in names.into_iter().enumerate() {
                        if crate::ui::pill(ui, self.cal.theme == k, name).clicked() {
                            self.cal.theme = k;
                        }
                    }
                });
                section(ui, "캘린더 안 메모 글자 크기");
                ui.add(
                    egui::Slider::new(&mut self.prefs.cal_font, 9.0..=18.0)
                        .step_by(0.5)
                        .suffix(" pt"),
                );
                section(ui, "완료 일정");
                toggle_row(
                    ui,
                    &mut self.prefs.cal_hide_done,
                    "완료 일정 숨기기",
                    "완료한 날의 일정을 캘린더 칸에서 감춥니다",
                );
                toggle_row(ui, &mut self.prefs.cal_strike, "완료 일정 취소선 긋기", "");
                section(ui, "보기 단위");
                segmented(ui, &mut self.prefs.cal_weeks, &["한 달", "1주", "2주"]);
                hint(ui, "1주·2주 보기는 선택한 날짜가 있는 주부터 보여 줍니다");
            }
            4 => {
                section(ui, "알림 소리");
                let mut sound = usize::from(!self.desktop.sound);
                if segmented(ui, &mut sound, &["Windows 기본", "소리 없음"]) {
                    self.desktop.sound = sound == 0;
                }
                ui.add_space(10.);
                toggle_row(
                    ui,
                    &mut self.desktop.popup,
                    "바탕화면 우측 하단에 팝업으로 알리기",
                    "",
                );
                toggle_row(
                    ui,
                    &mut self.desktop.mute,
                    "전체 알림 끄기",
                    "다시 켜면 알람이 설정된 메모의 알림이 재설정됩니다",
                );
                ui.add_space(10.);
                hint(
                    ui,
                    "알람 시각이 되면 화면 오른쪽 아래에 알림 창이 뜹니다 · 꺼져 있던 동안 지난 알람은 다음 실행 때 보여 줍니다",
                );
                ui.add_space(6.);
                if ui.button("소리 들어 보기").clicked() {
                    crate::system::alert_sound();
                }
            }
            5 => {
                section(ui, "전역 단축키");
                hint(ui, "다른 프로그램을 쓰는 중에도 동작합니다");
                ui.add_space(4.);
                for (i, name) in ["빠른 메모", "캘린더 분리창", "새 스티커 메모"]
                    .iter()
                    .enumerate()
                {
                    crate::ui::field(ui, name, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.shortcuts[i])
                                .desired_width(200.)
                                .margin(egui::Margin::symmetric(8, 5)),
                        );
                    });
                }
                ui.horizontal(|ui| {
                    if crate::ui::primary(ui, "단축키 적용").clicked() {
                        self.apply_shortcuts();
                    }
                    hint(ui, &self.hotkey_status);
                });
                hint(ui, "예: Ctrl+Alt+N · 충돌하면 기존 설정으로 복구됩니다");
                section(ui, "프로그램 사용 중");
                let tone = crate::ui::t(ui);
                for (key, label) in [
                    ("Ctrl+Shift+H", "모든 스티커 닫기"),
                    ("Ctrl+T", "항상 위에 켜기/끄기"),
                    ("Ctrl+B / I / U", "굵게 / 기울임 / 밑줄"),
                    ("~~ Space", "직전 단어 취소선"),
                    ("~~~ Space", "한 줄 전체 취소선"),
                    ("[] Space", "체크박스 넣기"),
                    ("Ctrl+;", "날짜 넣기"),
                    ("Ctrl+Shift+;", "날짜와 시간 넣기"),
                    ("Ctrl+F", "빠른 검색"),
                    ("Ctrl+H", "찾아 바꾸기"),
                    ("Ctrl+P", "프로젝트 선택"),
                    ("#이름 Space", "본문 첫 줄에서 프로젝트 지정"),
                    ("Esc Esc", "메모 닫기"),
                ] {
                    ui.horizontal(|ui| {
                        ui.add_sized([180., 22.], egui::Label::new(RichText::new(label)));
                        crate::ui::chip(ui, key, tone.field, tone.muted);
                    });
                }
            }
            6 => {
                section(ui, "데이터 위치");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.prefs.data_path)
                            .desired_width(ui.available_width() - 90.)
                            .margin(egui::Margin::symmetric(8, 5))
                            .hint_text("데이터 저장 폴더"),
                    );
                    if ui.button("폴더 선택").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.prefs.data_path = path.display().to_string();
                        }
                    }
                });
                let dir = crate::storage::data_dir(&self.prefs.data_path);
                hint(
                    ui,
                    format!(
                        "메모·위키·설정은 자동으로 저장됩니다 · 지금 위치: {}",
                        dir.display()
                    ),
                );
                if !self.saving.error.is_empty() {
                    ui.colored_label(ACCENT, &self.saving.error);
                }
                section(ui, "백업 / 복원");
                ui.horizontal(|ui| {
                    if crate::ui::primary(ui, "지금 백업").clicked() {
                        self.autosave(0., true);
                        self.desktop.last_action = match crate::storage::backup(&dir) {
                            Ok(p) => format!("백업했습니다: {}", p.display()),
                            Err(e) => format!("백업하지 못했습니다: {e}"),
                        };
                    }
                    if ui.button("백업에서 복원…").clicked() {
                        self.restore_from_backup(&dir);
                    }
                });
                hint(
                    ui,
                    "복원하기 전에 지금 데이터를 자동으로 한 번 더 백업합니다 · 백업 폴더의 data.db를 고르세요 (이전 버전 백업은 data.json)",
                );
                hint(
                    ui,
                    "이미지는 화면에 보일 때만 불러오므로 용량이 커져도 느려지지 않습니다 · 이미지는 지우지 않고 모든 백업이 함께 씁니다 · 다른 PC로 옮길 때는 데이터 폴더 전체를 복사하세요",
                );
            }
            _ => {
                crate::ui::banner(ui, Vec2::new(260., 64.));
                hint(
                    ui,
                    format!(
                        "개인 메모와 일정 작업 공간 · 버전 {}",
                        env!("CARGO_PKG_VERSION")
                    ),
                );
                ui.add_space(10.);
                ui.label("메모·위키·설정은 자동으로 저장됩니다.");
                ui.add_space(6.);
                if ui.button("사용법 보기").clicked() {
                    self.open_guide();
                    self.settings = false;
                }
            }
        }
    }
}
fn font_choice(ui: &mut egui::Ui, id: &str, value: &mut String) {
    ui.push_id(id, |ui| {
        ui.horizontal_wrapped(|ui| {
            for name in ["맑은 고딕", "굴림", "돋움", "바탕", "궁서체", "개구쟁이"]
            {
                if crate::ui::pill(ui, value == name, name).clicked() {
                    *value = name.into();
                }
            }
        });
    });
    crate::ui::hint(ui, "설치된 글꼴을 사용하며 없으면 기본 글꼴로 표시합니다.");
}
pub fn color_choices(ui: &mut egui::Ui, color: &mut Color32) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(6.);
        for c in PALETTE {
            let (rect, r) = ui.allocate_exact_size(Vec2::splat(24.), egui::Sense::click());
            let selected = *color == c;
            ui.painter().circle_filled(rect.center(), 11., c);
            ui.painter().circle_stroke(
                rect.center(),
                11.,
                egui::Stroke::new(1.0_f32, Color32::from_black_alpha(30)),
            );
            if selected {
                ui.painter()
                    .circle_stroke(rect.center(), 12.5, egui::Stroke::new(2.0_f32, ACCENT));
            }
            if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                *color = c;
            }
        }
        ui.color_edit_button_srgba(color)
            .on_hover_text("사용자 지정 색");
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_content_stacks_vertically_inside_its_navigation_row() {
        let mut app = App::sample(Local::now().date_naive());
        let ctx = egui::Context::default();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1000., 760.),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.ppt_settings(ui));
            },
        );
        let find = |label: &str| {
            output
                .shapes
                .iter()
                .find_map(|s| {
                    if let egui::Shape::Text(t) = &s.shape {
                        if t.galley.job.text == label {
                            Some(t.pos)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .unwrap()
        };
        let first = find("Windows 시작 시 자동 실행");
        let second = find("메모창을 작업표시줄에 표시");
        assert!(second.y > first.y + 5.);
        assert!(second.x < 850.);
    }
    #[test]
    fn first_real_line_becomes_title_without_consuming_project_directive() {
        let mut title = String::new();
        let mut body = "#프로젝트 \n\n첫 제목\n본문입니다".to_owned();
        infer_title(&mut title, &mut body);
        assert_eq!(title, "첫 제목");
        assert_eq!(body, "#프로젝트 \n\n본문입니다");
        infer_title(&mut title, &mut body);
        assert_eq!(title, "첫 제목");
    }
    #[test]
    fn trash_restore_keeps_original_identity_and_body() {
        let mut app = App::sample(Local::now().date_naive());
        let body = app.data.notes[0].body.clone();
        app.trash_note(0);
        assert!(app.data.notes[0].deleted);
        app.restore_note(0);
        assert!(!app.data.notes[0].deleted);
        assert_eq!(app.data.notes[0].body, body);
    }
}
