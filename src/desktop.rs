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
    pub data_path: String,
    pub image_limit: f32,
    pub image_age: usize,
    pub calendar_basis: usize,
    pub deleted_at: BTreeMap<usize, NaiveDateTime>,
    pub passwords: BTreeMap<usize, String>,
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
            data_path: String::new(),
            image_limit: 5.,
            image_age: 0,
            calendar_basis: 1,
            deleted_at: BTreeMap::new(),
            passwords: BTreeMap::new(),
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
        let n = &mut self.data.notes[i];
        let fill = n.color;
        egui::Frame::NONE
            .fill(fill)
            .corner_radius(18.)
            .inner_margin(20.)
            .show(ui, |ui| {
                ui.visuals_mut().override_text_color = Some(INK);
                ui.add(
                    egui::TextEdit::multiline(&mut n.title)
                        .font(egui::FontId::proportional(22.))
                        .desired_rows(1)
                        .desired_width(f32::INFINITY)
                        .frame(false)
                        .hint_text("제목"),
                );
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
                crate::composer::show(ui, &mut n.body, &mut n.document, ("inline", i));
            });
        self.consume_body_project(i);
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
                        "화면 잠금 미리보기입니다 · 암호화 저장은 아직 연결 전입니다",
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
                    self.desktop.passwords.get(&i) == Some(&self.desktop.lock_input)
                };
                if valid {
                    if set {
                        self.desktop
                            .passwords
                            .insert(i, self.desktop.lock_input.clone());
                        self.data.notes[i].locked = true;
                    } else {
                        self.data.notes[i].locked = false;
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
                    crate::ui::hint(ui, "Windows 알림 연결은 후속 단계입니다");
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
                let n = &mut self.data.notes[i];
                n.body.clear();
                n.title = "삭제된 메모".into();
                n.document = Default::default();
                n.wiki_comments.clear();
                n.journal_comments.clear();
                n.occurrence_comments.clear();
                n.purged = true;
                n.deleted = true;
                self.desktop.passwords.remove(&i);
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
            "알람",
            "단축키",
            "백업 / 복원",
            "정보",
        ];
        let icons = [
            crate::icons::Icon::Settings,
            crate::icons::Icon::Home,
            crate::icons::Icon::Sticky,
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
                            RichText::new(tabs[self.desktop.settings_tab.min(6)])
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
                    "",
                );
                toggle_row(
                    ui,
                    &mut self.desktop.taskbar,
                    "메모창을 작업표시줄에 표시",
                    "끄면 트레이 아이콘으로만 표시합니다",
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
                ui.add_space(14.);
                hint(
                    ui,
                    "운영체제 연동과 자동 정리 타이머는 후속 단계에서 연결됩니다.",
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
                    "Windows 알림센터 연동 · 알람 메모 강조 · 트레이 상주는 후속 단계입니다.",
                );
                ui.add_space(6.);
                if ui.button("알림 모양 미리보기").clicked() {
                    self.desktop.last_action = "알림 미리보기 · 메모를 확인할 시간입니다".into();
                }
            }
            4 => {
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
            5 => {
                section(ui, "데이터 위치");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.desktop.data_path)
                            .desired_width(ui.available_width() - 90.)
                            .margin(egui::Margin::symmetric(8, 5))
                            .hint_text("데이터 저장 폴더"),
                    );
                    if ui.button("폴더 선택").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.desktop.data_path = path.display().to_string();
                        }
                    }
                });
                ui.add_space(4.);
                if ui.button("데이터 가져오기…").clicked() {
                    self.desktop.last_action =
                        "가져오기 화면 미리보기입니다. 실제 데이터는 변경하지 않았습니다.".into();
                }
                section(ui, "이미지 최대 저장 용량");
                ui.add(
                    egui::Slider::new(&mut self.desktop.image_limit, 0.5..=10.)
                        .step_by(0.5)
                        .suffix(" GB"),
                );
                section(ui, "오래된 이미지 정리");
                segmented(
                    ui,
                    &mut self.desktop.image_age,
                    &["용량 기준", "1년 이상", "2년 이상", "3년 이상"],
                );
                hint(ui, "백업 후 100 kB 이하 썸네일만 남깁니다.");
                ui.add_space(6.);
                if ui.button("썸네일 처리된 원본 일괄 정리").clicked() {
                    self.desktop.last_action =
                        "정리 정책을 선택했습니다. 이번 미리보기에서는 파일을 삭제하지 않습니다."
                            .into();
                }
                ui.add_space(10.);
                hint(ui, "저장·백업 엔진은 아직 연결 전입니다.");
            }
            _ => {
                ui.label(RichText::new("MEMONOKING").size(16.).strong());
                hint(ui, "개인 메모와 일정 작업 공간 · UI 미리보기 0.2.0");
                ui.add_space(10.);
                ui.label("화면·조작 미리보기입니다. 앱을 종료하면 메모와 설정이 초기화됩니다.");
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
