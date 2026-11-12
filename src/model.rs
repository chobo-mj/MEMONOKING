use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime};
use eframe::egui::Vec2;
#[derive(Clone)]
pub struct Note {
    pub title: String,
    pub body: String,
    pub document: crate::composer::Document,
    pub reminder: Option<NaiveDateTime>,
    pub locked: bool,
    pub purged: bool,
    pub date: Option<NaiveDate>,
    pub duration: u16,
    pub end_time: Option<NaiveTime>,
    pub occurrence_completed: std::collections::BTreeMap<NaiveDate, NaiveDateTime>,
    pub occurrence_comments: std::collections::BTreeMap<NaiveDate, Vec<String>>,
    pub time: NaiveTime,
    pub done: bool,
    pub completed: Option<NaiveDateTime>,
    pub starred: bool,
    pub recurring: bool,
    pub repeat: crate::repeat::Repeat,
    pub project: Option<usize>,
    pub parents: Vec<usize>,
    pub created: NaiveDateTime,
    pub wiki: bool,
    pub folder: Option<usize>,
    /// Wiki tab the memo is filed in (at the tab's root unless `folder`).
    /// `None` with no folder: an unfiled wiki memo.
    pub wiki_tab: Option<usize>,
    pub wiki_comments: Vec<String>,
    pub journal_comments: Vec<String>,
    pub color: eframe::egui::Color32,
    pub topmost: bool,
    pub fold: u8,
    pub deleted: bool,
    pub has_image: bool,
    pub has_table: bool,
    pub has_link: bool,
    pub pos: Vec2,
}
impl Note {
    pub fn new(title: &str, date: Option<NaiveDate>, i: usize) -> Self {
        Self {
            title: title.into(),
            body: "아이디어와 준비할 내용을 여기에 정리하세요.".into(),
            document: Default::default(),
            reminder: None,
            locked: false,
            purged: false,
            date,
            duration: 1,
            end_time: None,
            occurrence_completed: Default::default(),
            occurrence_comments: Default::default(),
            time: NaiveTime::from_hms_opt(9 + (i % 8) as u32, 0, 0).unwrap(),
            done: false,
            completed: None,
            starred: false,
            recurring: false,
            repeat: Default::default(),
            project: None,
            parents: vec![],
            created: Local::now().naive_local(),
            wiki: false,
            folder: None,
            wiki_tab: None,
            wiki_comments: vec![],
            journal_comments: vec![],
            color: crate::PALETTE[i % 10],
            topmost: false,
            fold: 0,
            deleted: false,
            has_image: false,
            has_table: false,
            has_link: false,
            pos: Vec2::new(30. + (i % 3) as f32 * 300., 30. + (i / 3) as f32 * 240.),
        }
    }
}
impl Note {
    /// Update an explicit schedule as one operation. Completion timestamps record
    /// when work finished; moving a schedule only moves its occurrence date keys.
    pub fn set_span(&mut self, first: NaiveDateTime, last: NaiveDateTime) {
        let (start, end) = if first <= last {
            (first, last)
        } else {
            (last, first)
        };
        let was_single = self.duration <= 1;
        let single_completed = if was_single && self.done {
            self.completed
        } else {
            None
        };
        if was_single {
            self.occurrence_completed.clear();
        }
        self.move_start(start.date());
        self.time = start.time();
        self.end_time = Some(end.time());
        self.duration =
            ((end.date() - start.date()).num_days() + 1).clamp(1, u16::MAX as i64) as u16;
        if self.duration > 1 {
            if let Some(completed) = single_completed {
                self.occurrence_completed
                    .entry(start.date())
                    .or_insert(completed);
            }
        }
        self.refresh_done();
    }
    pub fn move_start(&mut self, date: NaiveDate) {
        if let Some(old) = self.date {
            let delta = date - old;
            self.occurrence_completed = std::mem::take(&mut self.occurrence_completed)
                .into_iter()
                .map(|(d, t)| (d + delta, t))
                .collect();
            self.occurrence_comments = std::mem::take(&mut self.occurrence_comments)
                .into_iter()
                .map(|(d, c)| (d + delta, c))
                .collect();
        }
        let was_unassigned = self.date.is_none();
        self.date = Some(date);
        if self.duration > 1 {
            if was_unassigned && self.done {
                if let Some(t) = self.completed {
                    for d in self.days() {
                        self.occurrence_completed.insert(d, t);
                    }
                }
            }
            self.refresh_done();
        }
    }
    pub fn days(&self) -> Vec<NaiveDate> {
        self.date
            .map(|d| {
                (0..self.duration.max(1))
                    .map(|i| d + Duration::days(i as i64))
                    .collect()
            })
            .unwrap_or_default()
    }
    pub fn occurs(&self, d: NaiveDate) -> bool {
        self.date.is_some_and(|start| {
            let len = self.duration.max(1) as i64;
            (d >= start && d < start + Duration::days(len))
                || (!self.repeat.is_none()
                    && (0..len).any(|k| self.repeat.matches(start, d - Duration::days(k))))
        })
    }
    pub fn done_on(&self, d: Option<NaiveDate>) -> bool {
        if self.duration > 1 && self.date.is_some() {
            d.is_some_and(|d| self.occurrence_completed.contains_key(&d))
        } else {
            self.done
        }
    }
    pub fn refresh_done(&mut self) {
        let Some(start) = self.date else {
            return;
        };
        let had_occurrences = !self.occurrence_completed.is_empty();
        let expected = self.duration.max(1) as i64;
        self.occurrence_completed.retain(|d, _| {
            let offset = (*d - start).num_days();
            offset >= 0 && offset < expected
        });
        if self.duration <= 1 {
            if had_occurrences {
                self.completed = self.occurrence_completed.get(&start).copied();
                self.done = self.completed.is_some();
                self.occurrence_completed.clear();
            } else if !self.done {
                self.completed = None;
            }
            return;
        }
        // Unique dates inside this inclusive span are complete iff their count
        // equals its length. This stays linear for schedules longer than 255 days.
        self.done = self.occurrence_completed.len() == expected as usize;
        self.completed = self.occurrence_completed.values().max().copied();
    }
}
/// The memos, remembering which ones were handed out for changing so that
/// saving only has to look at those.
#[derive(Default)]
pub struct Notes {
    items: Vec<Note>,
    touched: std::collections::BTreeSet<usize>,
    /// Any memo may have changed (the whole list was borrowed for writing).
    all: bool,
}
impl Notes {
    pub fn push(&mut self, n: Note) {
        self.touched.insert(self.items.len());
        self.items.push(n);
    }
    pub fn get_mut(&mut self, i: usize) -> Option<&mut Note> {
        self.touched.insert(i);
        self.items.get_mut(i)
    }
    /// The memos for a change that is not part of what is saved (such as
    /// releasing a picture's texture).
    pub fn untracked_mut(&mut self) -> &mut [Note] {
        &mut self.items
    }
    /// The memos handed out for changing since the last call; `None` when
    /// any of them may have changed.
    pub fn take_touched(&mut self) -> Option<Vec<usize>> {
        let touched = std::mem::take(&mut self.touched);
        if std::mem::take(&mut self.all) {
            None
        } else {
            Some(touched.into_iter().collect())
        }
    }
}
impl From<Vec<Note>> for Notes {
    fn from(items: Vec<Note>) -> Self {
        Self {
            items,
            touched: Default::default(),
            all: true,
        }
    }
}
impl std::ops::Deref for Notes {
    type Target = [Note];
    fn deref(&self) -> &[Note] {
        &self.items
    }
}
impl std::ops::DerefMut for Notes {
    fn deref_mut(&mut self) -> &mut [Note] {
        self.all = true;
        &mut self.items
    }
}
impl std::ops::Index<usize> for Notes {
    type Output = Note;
    fn index(&self, i: usize) -> &Note {
        &self.items[i]
    }
}
impl std::ops::IndexMut<usize> for Notes {
    fn index_mut(&mut self, i: usize) -> &mut Note {
        self.touched.insert(i);
        &mut self.items[i]
    }
}
pub struct Workspace {
    pub notes: Notes,
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
        notes[0].has_image = true;
        notes[1].has_table = true;
        notes[2].has_link = true;
        notes[0].starred = true;
        notes[0].wiki = true;
        notes[0].folder = Some(0);
        notes[0].wiki_tab = Some(0);
        notes[0].project = Some(0);
        notes[1].project = Some(0);
        notes[5].recurring = true;
        Self {
            notes: notes.into(),
            projects: vec!["OmniMemoJang".into(), "개인 업무".into()],
            links: vec![(0, 1)],
        }
    }
    pub fn complete(&mut self, i: usize, done: bool, now: DateTime<Local>) {
        let n = &mut self.notes[i];
        if n.duration > 1 && n.date.is_some() {
            for d in n.days() {
                if done {
                    n.occurrence_completed.entry(d).or_insert(now.naive_local());
                } else {
                    n.occurrence_completed.remove(&d);
                }
            }
            n.refresh_done();
        } else {
            if n.done != done {
                n.done = done;
                n.completed = if done { Some(now.naive_local()) } else { None };
            }
            n.occurrence_completed.clear();
            if !done {
                n.completed = None;
            }
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
        let mut stack = vec![p];
        let mut visited = std::collections::HashSet::new();
        while let Some(i) = stack.pop() {
            if i == c {
                return false;
            }
            if visited.insert(i) {
                stack.extend(self.notes[i].parents.iter().copied());
            }
        }
        if !self.notes[c].parents.contains(&p) {
            self.notes[c].parents.push(p);
        }

        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_repeating_note_occurs_on_each_repetition() {
        let start = NaiveDate::from_ymd_opt(2026, 11, 11).unwrap();
        let mut n = Note::new("회의", Some(start), 0);
        assert!(!n.occurs(start + Duration::days(7)));
        n.repeat = crate::repeat::Repeat::Weekly {
            every: 1,
            days: crate::repeat::bit(chrono::Weekday::Wed),
        };
        assert!(n.occurs(start));
        assert!(n.occurs(start + Duration::days(7)));
        assert!(!n.occurs(start + Duration::days(8)));
        // A two-day span repeats as a whole.
        n.duration = 2;
        assert!(n.occurs(start + Duration::days(8)));
    }
    #[test]
    fn nesting_rejects_cycles() {
        let mut w = Workspace::sample(Local::now().date_naive());
        assert!(w.reparent(1, 0));
        assert!(w.reparent(2, 1));
        assert!(!w.reparent(0, 2));
        assert!(!w.reparent(1, 1));
        assert!(w.notes[0].parents.is_empty());
        assert!(w.reparent(2, 0));
        assert_eq!(w.notes[2].parents, vec![1, 0]);
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

#[cfg(test)]
mod dag_extra_tests {
    use super::*;
    #[test]
    fn checks_every_parent_path() {
        let mut w = Workspace::sample(Local::now().date_naive());
        assert!(w.reparent(2, 0));
        assert!(w.reparent(2, 1));
        assert!(!w.reparent(1, 2));
        assert!(!w.reparent(0, 2));
        assert_eq!(w.notes[2].parents, vec![0, 1]);
    }
}

#[cfg(test)]
mod explicit_span_tests {
    use super::*;
    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }
    fn stamp(day: NaiveDate, hour: u32, minute: u32) -> NaiveDateTime {
        day.and_hms_opt(hour, minute, 0).unwrap()
    }
    #[test]
    fn explicit_span_exceeds_five_and_255_days_without_truncating_dates() {
        let start = date(2026, 12, 25);
        let end = start + Duration::days(399);
        let mut n = Note::new("장기간 일정", Some(start), 0);
        n.set_span(stamp(start, 9, 10), stamp(end, 18, 40));
        assert_eq!(n.duration, 400);
        assert_eq!(n.days().len(), 400);
        assert_eq!(n.days().first(), Some(&start));
        assert_eq!(n.days().last(), Some(&end));
        assert!(n.occurs(start + Duration::days(255)));
        assert!(n.occurs(end));
        assert!(!n.occurs(start - Duration::days(1)));
        assert!(!n.occurs(end + Duration::days(1)));
        assert_eq!(n.time, NaiveTime::from_hms_opt(9, 10, 0).unwrap());
        assert_eq!(
            n.end_time,
            Some(NaiveTime::from_hms_opt(18, 40, 0).unwrap())
        );
    }
    #[test]
    fn completing_a_long_span_is_idempotent_and_reopens_all_days() {
        let start = date(2026, 12, 25);
        let mut w = Workspace::sample(start);
        w.notes[0].set_span(
            stamp(start, 8, 0),
            stamp(start + Duration::days(399), 18, 0),
        );
        let now = Local::now();
        w.complete(0, true, now);
        assert!(w.notes[0].done);
        assert_eq!(w.notes[0].occurrence_completed.len(), 400);
        assert!(w.notes[0]
            .occurrence_completed
            .values()
            .all(|&t| t == now.naive_local()));
        w.complete(0, true, now + Duration::hours(1));
        assert!(w.notes[0]
            .occurrence_completed
            .values()
            .all(|&t| t == now.naive_local()));
        w.complete(0, false, now + Duration::hours(2));
        assert!(!w.notes[0].done);
        assert!(w.notes[0].completed.is_none());
        assert!(w.notes[0].occurrence_completed.is_empty());
        assert_eq!(w.notes[0].duration, 400);
        assert_eq!(
            w.notes[0].end_time,
            Some(NaiveTime::from_hms_opt(18, 0, 0).unwrap())
        );
    }
    #[test]
    fn moving_a_long_span_shifts_occurrence_identity_but_keeps_actual_completion_clock() {
        let old = date(2026, 12, 20);
        let mut n = Note::new("연말 작업", Some(old), 0);
        n.set_span(stamp(old, 8, 30), stamp(old + Duration::days(20), 17, 45));
        let created = n.created;
        let body = n.body.clone();
        let completed_day = old + Duration::days(12);
        let completed_at = stamp(date(2027, 1, 5), 14, 23);
        n.occurrence_completed.insert(completed_day, completed_at);
        n.occurrence_comments
            .insert(completed_day, vec!["작업 완료 후 보충".into()]);
        n.refresh_done();
        let new = old - Duration::days(35);
        n.move_start(new);
        let moved_day = new + Duration::days(12);
        assert_eq!(n.occurrence_completed.get(&moved_day), Some(&completed_at));
        assert_eq!(
            n.occurrence_comments.get(&moved_day),
            Some(&vec!["작업 완료 후 보충".to_owned()])
        );
        assert!(!n.occurrence_completed.contains_key(&completed_day));
        assert_eq!(n.completed, Some(completed_at));
        assert!(!n.done);
        assert_eq!(n.created, created);
        assert_eq!(n.body, body);
        assert_eq!(n.duration, 21);
        assert_eq!(n.time, NaiveTime::from_hms_opt(8, 30, 0).unwrap());
        assert_eq!(
            n.end_time,
            Some(NaiveTime::from_hms_opt(17, 45, 0).unwrap())
        );
    }
    #[test]
    fn changing_only_single_day_end_time_keeps_its_completed_state_and_clock() {
        let d = date(2026, 11, 10);
        let mut n = Note::new("완료한 단일 작업", Some(d), 0);
        let completed = stamp(d, 12, 21);
        n.done = true;
        n.completed = Some(completed);
        n.set_span(stamp(d, 9, 0), stamp(d, 18, 30));
        n.refresh_done();
        assert!(n.done);
        assert_eq!(n.completed, Some(completed));
        assert!(n.occurrence_completed.is_empty());
        assert_eq!(n.duration, 1);
        assert_eq!(
            n.end_time,
            Some(NaiveTime::from_hms_opt(18, 30, 0).unwrap())
        );
    }
    #[test]
    fn expanding_a_completed_single_day_keeps_only_its_first_occurrence_completed() {
        let old = date(2026, 11, 10);
        let new = old + Duration::days(3);
        let mut n = Note::new("기간으로 연장", Some(old), 0);
        let completed = stamp(old, 16, 5);
        n.done = true;
        n.completed = Some(completed);
        n.journal_comments.push("기존 완료 댓글".into());
        n.set_span(stamp(new, 9, 0), stamp(new + Duration::days(8), 17, 0));
        assert_eq!(n.duration, 9);
        assert!(!n.done);
        assert_eq!(n.occurrence_completed.len(), 1);
        assert_eq!(n.occurrence_completed.get(&new), Some(&completed));
        assert_eq!(n.completed, Some(completed));
        assert!(!n.done_on(Some(new + Duration::days(1))));
        assert_eq!(n.journal_comments, vec!["기존 완료 댓글"]);
    }
    #[test]
    fn shrinking_to_one_day_and_unchecking_cannot_resurrect_removed_completion() {
        let start = date(2026, 11, 10);
        let mut w = Workspace::sample(start);
        let completed = stamp(start, 12, 30);
        w.notes[0].set_span(stamp(start, 9, 0), stamp(start + Duration::days(9), 18, 0));
        w.notes[0].occurrence_completed.insert(start, completed);
        w.notes[0].occurrence_completed.insert(
            start + Duration::days(8),
            stamp(start + Duration::days(8), 17, 0),
        );
        w.notes[0].refresh_done();
        w.notes[0].set_span(stamp(start, 9, 0), stamp(start, 17, 0));
        assert!(w.notes[0].done);
        assert_eq!(w.notes[0].completed, Some(completed));
        assert!(w.notes[0].occurrence_completed.is_empty());
        w.complete(0, false, Local::now());
        w.notes[0].set_span(stamp(start, 9, 0), stamp(start, 19, 0));
        assert!(!w.notes[0].done);
        assert!(w.notes[0].completed.is_none());
        assert!(w.notes[0].occurrence_completed.is_empty());
    }
    #[test]
    fn reversed_explicit_datetimes_normalize_without_changing_completion_clock() {
        let old = date(2026, 11, 10);
        let mut n = Note::new("역순 선택", Some(old), 0);
        let completed = stamp(old, 10, 15);
        n.done = true;
        n.completed = Some(completed);
        let early = old - Duration::days(7);
        n.set_span(stamp(old, 18, 0), stamp(early, 8, 0));
        assert_eq!(n.date, Some(early));
        assert_eq!(n.duration, 8);
        assert_eq!(n.time, NaiveTime::from_hms_opt(8, 0, 0).unwrap());
        assert_eq!(n.end_time, Some(NaiveTime::from_hms_opt(18, 0, 0).unwrap()));
        assert_eq!(n.occurrence_completed.get(&early), Some(&completed));
    }
}

#[cfg(test)]
mod notes_tests {
    use super::*;
    #[test]
    fn notes_remember_which_memos_were_handed_out_for_changing() {
        let mut notes = Notes::from(vec![Note::new("a", None, 0), Note::new("b", None, 1)]);
        assert_eq!(notes.take_touched(), None);
        let _ = notes[0].title.len();
        let _ = notes.iter().count();
        assert_eq!(notes.take_touched(), Some(vec![]));
        notes[1].title = "c".into();
        notes.push(Note::new("d", None, 2));
        notes.untracked_mut()[0].fold = 1;
        assert_eq!(notes.take_touched(), Some(vec![1, 2]));
        for n in notes.iter_mut() {
            n.done = true;
        }
        assert_eq!(notes.take_touched(), None);
    }
}
