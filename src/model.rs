use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime};
use eframe::egui::Vec2;
pub struct Note {
    pub title: String,
    pub body: String,
    pub date: Option<NaiveDate>,
    pub time: NaiveTime,
    pub done: bool,
    pub completed: Option<NaiveDateTime>,
    pub starred: bool,
    pub recurring: bool,
    pub label: usize,
    pub project: Option<usize>,
    pub parent: Option<usize>,
    pub pos: Vec2,
}
impl Note {
    pub fn new(title: &str, date: Option<NaiveDate>, i: usize) -> Self {
        Self {
            title: title.into(),
            body: "아이디어와 준비할 내용을 여기에 정리하세요.".into(),
            date,
            time: NaiveTime::from_hms_opt(9 + (i % 8) as u32, 0, 0).unwrap(),
            done: false,
            completed: None,
            starred: false,
            recurring: false,
            label: i % 3,
            project: None,
            parent: None,
            pos: Vec2::new(30. + (i % 3) as f32 * 300., 30. + (i / 3) as f32 * 240.),
        }
    }
}
pub struct Workspace {
    pub notes: Vec<Note>,
    pub projects: Vec<String>,
    pub links: Vec<(usize, usize)>,
}
impl Workspace {
    pub fn sample(today: NaiveDate) -> Self {
        let mut notes = vec![
            Note::new("디자인 방향 정리", Some(today), 0),
            Note::new("화면 흐름 검토", Some(today), 1),
            Note::new("주간 회의 준비", Some(today + Duration::days(1)), 2),
            Note::new("프로토타입 확인", Some(today + Duration::days(3)), 3),
            Note::new("떠오른 아이디어 모으기", None, 4),
            Note::new("하루 마무리 정리", Some(today), 5),
        ];
        notes[0].body = "캘린더 가독성을 높이고 메모와 일정을 자연스럽게 연결합니다.".into();
        notes[0].starred = true;
        notes[0].project = Some(0);
        notes[1].project = Some(0);
        notes[5].recurring = true;
        Self {
            notes,
            projects: vec!["MEMONOKING".into(), "개인 업무".into()],
            links: vec![(0, 1)],
        }
    }
    pub fn complete(&mut self, i: usize, done: bool, now: DateTime<Local>) {
        let n = &mut self.notes[i];
        if n.done != done {
            n.done = done;
            n.completed = if done { Some(now.naive_local()) } else { None };
        }
    }
    pub fn connect(&mut self, a: usize, b: usize) {
        let e = (a.min(b), a.max(b));
        if a != b && !self.links.contains(&e) {
            self.links.push(e);
        }
    }
    pub fn reparent(&mut self, c: usize, p: usize) -> bool {
        if c >= self.notes.len() || p >= self.notes.len() {
            return false;
        }
        let mut cursor = Some(p);
        let mut depth = 0;
        while let Some(i) = cursor {
            if i == c || depth > self.notes.len() {
                return false;
            }
            cursor = self.notes[i].parent;
            depth += 1;
        }
        self.notes[c].parent = Some(p);
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nesting_rejects_cycles() {
        let mut w = Workspace::sample(Local::now().date_naive());
        assert!(w.reparent(1, 0));
        assert!(w.reparent(2, 1));
        assert!(!w.reparent(0, 2));
        assert!(!w.reparent(1, 1));
        assert_eq!(w.notes[0].parent, None);
    }
    #[test]
    fn completion_stamps_once_and_reopening_removes_journal_entry() {
        let mut w = Workspace::sample(Local::now().date_naive());
        let now = Local::now();
        w.complete(0, true, now);
        let first = w.notes[0].completed;
        w.complete(0, true, now + Duration::hours(1));
        assert_eq!(w.notes[0].completed, first);
        w.complete(0, false, now);
        assert!(w.notes[0].completed.is_none());
    }
    #[test]
    fn relationships_are_unique() {
        let mut w = Workspace::sample(Local::now().date_naive());
        w.connect(1, 0);
        w.connect(0, 0);
        assert_eq!(w.links, vec![(0, 1)]);
    }
}
