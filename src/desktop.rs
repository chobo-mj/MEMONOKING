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
    if let Some((offset, line)) = body
        .split_inclusive('\n')
        .scan(0usize, |pos, line| {
            let old = *pos;
            *pos += line.len();
            Some((old, line))
        })
        .find(|(_, line)| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
    {
        *title = line.trim().into();
        body.replace_range(offset..offset + line.len(), "");
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
        if self.desktop.focus_dialog {
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            self.desktop.focus_dialog = false;
        }
        if self.desktop.project_popup {
            let mut open = true;
            egui::Window::new("프로젝트 선택")
                .open(&mut open)
                .default_width(300.)
                .show(ctx, |ui| {
                    if ui
                        .selectable_label(self.project_filter.is_none(), "모든 프로젝트")
                        .clicked()
                    {
                        self.project_filter = None;
                        self.desktop.project_popup = false;
                    }
                    if ui
                        .selectable_label(self.project_filter == Some(NO_PROJECT), "프로젝트 없음")
                        .clicked()
                    {
                        self.project_filter = Some(NO_PROJECT);
                        self.desktop.project_popup = false;
                    }
                    for (i, name) in self.data.projects.iter().enumerate() {
                        if self.extra.hidden_projects.contains(&i)
                            || self.extra.deleted_projects.contains(&i)
                        {
                            continue;
                        }
                        if ui
                            .selectable_label(self.project_filter == Some(i), name)
                            .clicked()
                        {
                            self.project_filter = Some(i);
                            self.page = 0;
                            self.desktop.project_popup = false;
                        }
                    }
                });
            self.desktop.project_popup &= open;
        }
        if let Some((i, set, sticker)) = self.desktop.lock_target {
            let mut open = true;
            let mut apply = false;
            egui::Window::new(if set {
                "메모 잠금 설정"
            } else {
                "메모 잠금 해제"
            })
            .id(egui::Id::new("lock_dialog"))
            .open(&mut open)
            .collapsible(false)
            .default_width(340.)
            .show(ctx, |ui| {
                ui.strong(&self.data.notes[i].title);
                ui.label("미리보기 잠금입니다. 암호화 저장은 아직 연결하지 않았습니다.");
                ui.add(
                    egui::TextEdit::singleline(&mut self.desktop.lock_input)
                        .password(true)
                        .hint_text("비밀번호"),
                );
                if set {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.desktop.lock_confirm)
                            .password(true)
                            .hint_text("비밀번호 확인"),
                    );
                }
                if !self.desktop.lock_error.is_empty() {
                    ui.colored_label(ACCENT, &self.desktop.lock_error);
                }
                apply = ui.button(if set { "잠그기" } else { "열기" }).clicked();
            });
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
            if !open {
                self.desktop.lock_target = None;
                self.desktop.lock_input.clear();
                self.desktop.lock_confirm.clear();
            }
        }
        if let Some(i) = self.desktop.reminder_target {
            let mut open = true;
            let mut applied = false;
            let mut cleared = false;
            egui::Window::new("알람 설정")
                .id(egui::Id::new("reminder_dialog"))
                .open(&mut open)
                .collapsible(false)
                .default_width(320.)
                .show(ctx, |ui| {
                    ui.strong(&self.data.notes[i].title);
                    calendar_input(
                        ui,
                        &mut self.desktop.reminder_month,
                        &mut self.desktop.reminder_date,
                    );
                    ui.horizontal(|ui| {
                        ui.label("시간");
                        ui.add(
                            egui::DragValue::new(&mut self.desktop.reminder_hour)
                                .range(0..=23)
                                .suffix("시"),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.desktop.reminder_minute)
                                .range(0..=59)
                                .suffix("분"),
                        );
                    });
                    ui.label(
                        RichText::new("알람 표시 미리보기 · Windows 알림은 후속 연결")
                            .small()
                            .color(MUTED),
                    );
                    ui.horizontal(|ui| {
                        applied = ui.button("설정").clicked();
                        cleared = ui.button("알람 해제").clicked();
                    });
                });
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
        if let Some(i) = self.desktop.purge_target {
            let mut open = true;
            let mut purge = false;
            egui::Window::new("메모 영구삭제")
                .id(egui::Id::new("purge_dialog"))
                .open(&mut open)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "‘{}’의 내용과 댓글을 삭제할까요?",
                        self.data.notes[i].title
                    ));
                    ui.label("이 작업은 실행 중인 미리보기에서 되돌릴 수 없습니다.");
                    ui.horizontal(|ui| {
                        purge = ui.button(RichText::new("영구삭제").color(ACCENT)).clicked();
                        if ui.button("취소").clicked() {
                            self.desktop.purge_target = None;
                        }
                    });
                });
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
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(130.);
                for (i, text) in ["일반", "메인창", "메모창", "알람", "단축키", "백업 / 복원", "정보"].iter().enumerate() {
                    if ui.add_sized([126., 38.], egui::Button::new(*text).selected(self.desktop.settings_tab == i)).clicked() { self.desktop.settings_tab = i; }
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().id_salt("settings_content").max_height(520.).show(ui, |ui| {
                ui.vertical(|ui| {
                ui.set_width(560.);
                match self.desktop.settings_tab {
                    0 => {
                        ui.heading("일반"); ui.add_space(16.);
                        ui.checkbox(&mut self.desktop.autostart, "Windows 시작 시 자동 실행");
                        ui.checkbox(&mut self.desktop.taskbar, "메모창을 작업표시줄에 표시");
                        ui.label(RichText::new("운영체제 적용은 후속 단계입니다. 현재 선택값만 유지합니다.").small().color(MUTED));
                        ui.add_space(14.); ui.strong("휴지통 보관 기간");
                        choice(ui, "retention", &mut self.desktop.retention, &["7일", "30일", "90일", "365일", "자동삭제 안 함"]);
                        ui.add_space(14.); ui.strong("잠긴 메모 자동 잠금");
                        choice(ui, "auto_lock", &mut self.desktop.auto_lock, &["1분", "5분", "15분", "Windows 잠금 때만"]);
                        ui.label(RichText::new("보관기간 자동삭제와 자동 잠금 타이머는 후속 연결").small().color(MUTED));
                    }
                    1 => {
                        ui.heading("메인창"); ui.add_space(14.);
                        ui.horizontal(|ui| { ui.selectable_value(&mut self.dark, false, "라이트 모드"); ui.selectable_value(&mut self.dark, true, "다크 모드"); });
                        font_choice(ui, "main_font", &mut self.desktop.font);
                        ui.add(egui::Slider::new(&mut self.desktop.font_size, 8.0..=22.0).text("글꼴 크기 (pt)"));
                        ui.add(egui::Slider::new(&mut self.desktop.letter_spacing, -0.5..=1.0).step_by(0.5).text("자간"));
                        choice(ui, "row_spacing", &mut self.desktop.row_spacing, &["좁게", "기본", "넓게"]);
                        ui.separator(); ui.strong("내 작업실");
                        ui.checkbox(&mut self.desktop.show_preview, "캘린더와 함께 볼 때 본문 한 줄 표시");
                        ui.checkbox(&mut self.desktop.hide_locked, "잠긴 메모 숨기기");
                        ui.checkbox(&mut self.desktop.hide_completed, "완료 메모 숨기기");
                    }
                    2 => {
                        ui.heading("메모창"); ui.add_space(14.); ui.strong("새 메모 기본 색");
                        color_choices(ui, &mut self.desktop.default_color);
                        ui.checkbox(&mut self.desktop.auto_toolbar, "스티커 툴바 자동 숨기기");
                        ui.checkbox(&mut self.desktop.skip_title, "접기 1단계 건너뛰기 · 바로 색상 상자로");
                        ui.checkbox(&mut self.desktop.unpin_minimized, "최소화하면 항상 위에 자동 해제");
                        ui.separator(); font_choice(ui, "note_font", &mut self.desktop.note_font);
                        ui.add(egui::Slider::new(&mut self.desktop.note_font_size, 8.0..=30.0).text("기본 글자 크기 (pt)"));
                        ui.add(egui::Slider::new(&mut self.desktop.opacity, 25.0..=100.0).text("기본 투명도 (%)"));
                        ui.label(RichText::new("투명도는 메모 바탕색으로 미리 봅니다.").small().color(MUTED));
                        ui.strong("이미지 붙여넣기 기본 크기"); choice(ui, "image_scale", &mut self.desktop.image_scale, &["25%", "50%", "75%", "100%", "너비 맞춤"]);
                    }
                    3 => {
                        ui.heading("알람"); ui.add_space(14.);
                        ui.horizontal(|ui| { ui.selectable_value(&mut self.desktop.sound, true, "Windows 기본 소리"); ui.selectable_value(&mut self.desktop.sound, false, "소리 없음"); });
                        ui.checkbox(&mut self.desktop.popup, "바탕화면 우측 하단 팝업");
                        ui.checkbox(&mut self.desktop.mute, "전체 알림 끄기");
                        ui.add_space(16.); ui.label("메모별 알람 아이콘으로 날짜와 시간을 설정할 수 있습니다.");
                        ui.label("알림센터 연동·메모 전면 표시·테두리 강조·트레이 상주는 후속 구현입니다.");
                        if ui.button("알림 모양 미리보기").clicked() { self.desktop.last_action = "알림 미리보기 · 메모를 확인할 시간입니다".into(); }
                    }
                    4 => {
                        self.settings_ui(ui);
                        ui.separator(); ui.strong("프로그램 사용 중");
                        for (key, label) in [("Ctrl+Shift+H", "모든 스티커 닫기"), ("Ctrl+T", "항상 위에 켜기/끄기"), ("Ctrl+B / I / U", "굵게 / 기울임 / 밑줄"), ("Ctrl+;", "날짜 넣기"), ("Ctrl+Shift+;", "날짜와 시간 넣기"), ("Ctrl+F", "빠른 검색"), ("Ctrl+H", "찾아 바꾸기"), ("Ctrl+P", "프로젝트 선택"), ("Esc 두 번", "현재 메모 닫기")] { ui.horizontal(|ui| { ui.monospace(key); ui.label(label); }); }
                    }
                    5 => {
                        ui.heading("백업 / 복원"); ui.add_space(14.);
                        ui.label("데이터 폴더"); ui.text_edit_singleline(&mut self.desktop.data_path);
                        ui.horizontal(|ui| { if ui.button("폴더 선택").clicked() { if let Some(path) = rfd::FileDialog::new().pick_folder() { self.desktop.data_path = path.display().to_string(); } }
 if ui.button("데이터 가져오기").clicked() { self.desktop.last_action = "가져오기 화면 미리보기입니다. 실제 데이터는 변경하지 않았습니다.".into(); } });
                        ui.separator(); ui.add(egui::Slider::new(&mut self.desktop.image_limit, 0.5..=10.).step_by(0.5).text("이미지 최대 저장 용량 (GB)"));
                        ui.strong("이미지 정리 방법");
                        choice(ui, "image_age", &mut self.desktop.image_age, &["용량 기준", "1년 이상", "2년 이상", "3년 이상"]);
                        ui.label("백업 후 100 kB 이하 썸네일을 남기는 정책입니다.");
                        if ui.button("썸네일 처리된 원본 일괄 정리").clicked() { self.desktop.last_action = "이미지 정리 정책을 선택했습니다. 이번 미리보기에서는 파일을 삭제하지 않습니다.".into(); }
                        ui.label(RichText::new("설정 화면 미리보기 · 저장 및 백업 엔진은 연결 전").color(MUTED));
                    }
                    _ => { ui.heading("MEMONOKING"); ui.label("개인 메모와 일정 작업 공간"); ui.label("UI 미리보기 0.2.0"); ui.separator(); ui.label("화면·조작 미리보기입니다. 앱을 종료하면 메모와 설정이 초기화됩니다."); }
                }
                if !self.desktop.last_action.is_empty() { ui.add_space(12.); ui.colored_label(ACCENT, &self.desktop.last_action); }
                });
            });
        });
    }
}
fn font_choice(ui: &mut egui::Ui, id: &str, value: &mut String) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(value.as_str())
        .show_ui(ui, |ui| {
            for name in ["맑은 고딕", "굴림", "돋움", "바탕", "궁서체", "개구쟁이"]
            {
                ui.selectable_value(value, name.into(), name);
            }
        });
    ui.label(
        RichText::new("설치된 글꼴을 사용하며 없으면 기본 글꼴로 표시합니다.")
            .small()
            .color(MUTED),
    );
}
fn choice(ui: &mut egui::Ui, id: &str, value: &mut usize, choices: &[&str]) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(choices[(*value).min(choices.len() - 1)])
        .show_ui(ui, |ui| {
            for (i, text) in choices.iter().enumerate() {
                ui.selectable_value(value, i, *text);
            }
        });
}
pub fn color_choices(ui: &mut egui::Ui, color: &mut Color32) {
    ui.horizontal_wrapped(|ui| {
        for c in PALETTE {
            let r = ui.add_sized(
                [28., 28.],
                egui::Button::new(if *color == c { "✓" } else { "" }).fill(c),
            );
            if r.clicked() {
                *color = c;
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("사용자 지정");
        ui.color_edit_button_srgba(color);
    });
}
pub fn calendar_input(ui: &mut egui::Ui, month: &mut NaiveDate, day: &mut NaiveDate) {
    ui.horizontal(|ui| {
        if ui.button("‹").clicked() {
            *month = (*month - Duration::days(1)).with_day(1).unwrap();
        }
        ui.strong(month.format("%Y년 %m월").to_string());
        if ui.button("›").clicked() {
            *month = (*month + Duration::days(32)).with_day(1).unwrap();
        }
    });
    egui::Grid::new("reminder_calendar").show(ui, |ui| {
        for s in ["일", "월", "화", "수", "목", "금", "토"] {
            ui.label(s);
        }
        ui.end_row();
        let offset = month.weekday().num_days_from_sunday() as i64;
        for slot in 0..42 {
            let d = *month + Duration::days(slot - offset);
            if d.month() == month.month() {
                if ui
                    .add_sized(
                        [30., 26.],
                        egui::Button::new(d.day().to_string()).selected(*day == d),
                    )
                    .clicked()
                {
                    *day = d;
                }
            } else {
                ui.label("");
            }
            if slot % 7 == 6 {
                ui.end_row();
            }
        }
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
