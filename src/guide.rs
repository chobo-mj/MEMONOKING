//! The usage guide: a "사용법" wiki tab with one memo per menu, each with a
//! screenshot. It is added to a new workspace and opens on the very first run.
use crate::composer::{placeholder, Block, Picture};
use crate::*;

pub const TAB: &str = "사용법";

struct Page {
    title: &'static str,
    image: &'static [u8],
    intro: &'static str,
    body: &'static str,
}

const PAGES: &[Page] = &[
    Page {
        title: "① 처음 시작하기",
        image: include_bytes!("../assets/guide/home.jpg"),
        intro: "OmniMemoJang은 메모, 할 일, 일정, 위키를 한곳에서 다루는 개인 작업 공간입니다. 이 사용법은 나만의 위키의 ‘사용법’ 탭에 있어 언제든 다시 볼 수 있습니다.",
        body: "· 왼쪽 메뉴창에서 화면을 고릅니다: 내 작업실, 오늘의 할일, 다음에 할일, 캘린더, 전체메모, 나만의 위키, 편의기능.
· ‘빠른 메모’를 누르면 바로 새 스티커 메모가 열립니다. 단축키 Ctrl+Alt+N(설정에서 변경)으로 다른 프로그램을 쓰는 중에도 열 수 있습니다.
· 메모·위키·설정은 자동으로 저장됩니다. 다음 실행부터는 내 작업실(또는 설정에서 고른 첫 화면)이 열립니다.
· 메인 창을 X로 닫아도 프로그램은 화면 오른쪽 아래 트레이에서 계속 실행되고 스티커 메모는 바탕화면에 남습니다. 트레이 아이콘을 클릭하면 다시 열리고, 우클릭 → ‘종료’로 끝냅니다.
· 메뉴창 맨 위 아이콘으로 메뉴창을 접고 펼칩니다.",
    },
    Page {
        title: "② 내 작업실",
        image: include_bytes!("../assets/guide/home.jpg"),
        intro: "날짜별로 묶인 메모 목록과 캘린더를 함께 봅니다.",
        body: "· 메모를 누르면 스티커로 열립니다. 오른쪽 위 캘린더 아이콘으로 자세히 보기(목록 + 미리보기)로 바꿀 수 있습니다.
· 동그라미를 누르면 완료됩니다. 여러 날 이어지는 일정은 ‘이 날짜만’ 또는 ‘전체 일정’ 중에서 고릅니다.
· 위쪽 필터: 프로젝트, 정기 일정 포함, 생성일 기준, 완료 메모 보기.
· ‘+ 새 메모’로 메모를 만들고, 메모를 캘린더 날짜로 끌면 일정이 정해집니다.
· 행의 ☆로 중요 표시, 연필로 편집창을 엽니다.",
    },
    Page {
        title: "③ 오늘의 할일 · 다음에 할일",
        image: include_bytes!("../assets/guide/today.jpg"),
        intro: "오늘 할 일과 앞으로의 일을 모아 보고, 오른쪽 미리보기에서 바로 고칩니다.",
        body: "· 메모를 누르면 오른쪽 미리보기에서 제목과 본문을 바로 편집합니다.
· 미리보기 아래 도구: 알람, 일정, 프로젝트 선택, 연결된 메모. 제목 줄 오른쪽 동그라미는 완료 체크입니다.
· 행 왼쪽 눈 아이콘을 누르면 스티커로 띄웁니다.
· 설정 › 일반의 ‘매일 오늘 메모 자동으로 띄우기’를 켜면 매일 새벽 5시 기준으로 지난 미완료 메모와 오늘 메모가 화면 오른쪽에 제목 스티커로 정렬됩니다.",
    },
    Page {
        title: "④ 스티커 메모",
        image: include_bytes!("../assets/guide/sticker.jpg"),
        intro: "메모 하나가 바탕화면 위의 창 하나입니다.",
        body: "· 위쪽 도구: + 새 메모, 색, … 더보기(일정·잠금·위키 보관·프로젝트·정렬·복제·삭제), 알람, 일정, 편집창, 항상 위, 완료, 접기, 닫기.
· 접기를 누를 때마다 스티커 → 제목 한 줄 → 색상 상자로 작아집니다. 색상 상자는 한 번 누르면 펼쳐지고, 끌면 옮겨집니다.
· 아래쪽: 본문 탭(+로 탭 추가), 글자 크기·굵게·기울임·밑줄·취소선, 투명도, ▾ 프로젝트 선택, › 연결된 메모.
· 우클릭하면 스티커 정렬 메뉴(오른쪽·왼쪽·계단식, 모두 제목만 보기 등)가 나옵니다.
· 본문 첫 줄을 ‘#프로젝트이름 ’으로 시작하면 그 프로젝트로 분류됩니다. Esc를 두 번 누르면 닫힙니다.",
    },
    Page {
        title: "⑤ 캘린더",
        image: include_bytes!("../assets/guide/calendar.jpg"),
        intro: "작성일·일정·알람 기준으로 메모를 달력에 펼쳐 봅니다.",
        body: "· 여러 날 이어지는 일정은 색띠 하나로 보입니다. 띠를 누르면 그 날짜의 메모가 열립니다.
· 날짜에 마우스를 올리면 + 버튼으로 그 날짜에 새 스티커를 만듭니다. 메모를 다른 날짜로 끌어 일정을 옮깁니다.
· ‘근무자 보기’ 버튼은 다음 버전에서 연결된 엑셀 파일의 근무자를 보여 줄 자리입니다.
· 설정 › 캘린더에서 테마, 메모 글자 크기, 완료 일정 숨기기·취소선, 보기 단위(한 달·1주·2주)를 정합니다.",
    },
    Page {
        title: "⑥ 캘린더 분리창 · 스탠딩 캘린더",
        image: include_bytes!("../assets/guide/calwindow.jpg"),
        intro: "캘린더를 별도 창으로 띄워 두고, 바탕화면에 고정할 수도 있습니다.",
        body: "· 화면 오른쪽 위 분리 아이콘(또는 Ctrl+Alt+C)으로 분리창을 엽니다.
· 위쪽 동그라미로 테마(기본·종이·세이지·나이트)를 고르고, 창 아무 곳이나 우클릭해 한 달·1주·2주 보기를 바꿉니다.
· ‘최소화’를 누르면 달력 아이콘의 작은 색상 상자로 접힙니다. 상자를 누르면 다시 열리고, 분리창의 X를 누르면 상자로 돌아갑니다. 상자는 끌어서 옮기고, 모서리로 크기를 바꾸고, 우클릭으로 색을 바꾸거나 닫습니다.
· 스탠딩 아이콘(받침대 모양)을 누르면 보이는 크기 그대로 바탕화면에 고정됩니다. 바탕화면 아이콘을 그 위에 둘 수 있습니다. 같은 자리의 스탠딩 아이콘을 다시 누르면 풀립니다.",
    },
    Page {
        title: "⑦ 전체메모 · 찾아 바꾸기",
        image: include_bytes!("../assets/guide/allmemo.jpg"),
        intro: "모든 메모를 폴더와 표로 정리하고, 한꺼번에 찾고 바꿉니다.",
        body: "· 폴더 보기, 전체 메모 보기(표), 휴지통을 오가며 메모를 정리합니다. 메모를 폴더로 끌어 넣습니다.
· Ctrl+H 또는 찾기 막대의 버튼으로 찾아 바꾸기를 엽니다. 범위(전체·선택 프로젝트)를 고르고 찾기·모두 찾기·바꾸기·모두 바꾸기를 씁니다.
· ‘모두 찾기’ 결과는 오른쪽에 메모별로 보이고, 원하는 결과만 골라 바꿀 수 있습니다.
· 지운 메모는 휴지통에 보관되며, 설정 › 일반의 보관 기간이 지나면 프로그램을 켤 때 완전히 지워집니다.",
    },
    Page {
        title: "⑧ 나만의 위키",
        image: include_bytes!("../assets/guide/wiki.jpg"),
        intro: "오래 남길 지식을 탭과 폴더로 정리하고, 두 영역에 나란히 띄워 비교합니다.",
        body: "· 위쪽 탭마다 자기 폴더 트리가 있습니다. +로 탭을 더하고, 더블클릭으로 이름을 바꿉니다. 탭을 바꿔도 열어 둔 영역의 내용은 그대로입니다.
· 왼쪽 위는 트리, 왼쪽 아래는 아직 분류하지 않은 위키 메모입니다. 메모를 탭·폴더로 끌어 분류합니다.
· 트리에서 우클릭하면 새 위키 노트, 새 폴더, 이름 바꾸기를 할 수 있습니다.
· 오른쪽 영역 1·2 중 누른 영역에 메모가 열립니다. 영역 2의 ‘빈 메모 열기’로 만든 메모는 영역 2를 닫을 때까지 유지되고, 그동안 고른 메모는 영역 1에 열립니다.
· 영역 위쪽 아이콘: 스티커로 열기, 댓글 보기, 닫기.",
    },
    Page {
        title: "⑨ 편의기능",
        image: include_bytes!("../assets/guide/utilities.jpg"),
        intro: "업무일지와 메모 속 표·이미지·링크를 모아 보는 보조 화면입니다.",
        body: "· 업무일지: 날짜별·월별로 완료한 일을 모아 보고 댓글을 남깁니다.
· 표 모아보기, 이미지 모아보기, 링크 · 즐겨찾기에서 메모 속 자료를 한눈에 봅니다.
· 메뉴가 필요 없으면 설정 › 일반 › 메뉴 숨기기에서 숨길 수 있습니다.",
    },
    Page {
        title: "⑩ 설정 · 백업",
        image: include_bytes!("../assets/guide/settings.jpg"),
        intro: "왼쪽 아래 ‘설정’에서 프로그램을 내 방식에 맞춥니다.",
        body: "· 일반: Windows 시작 시 자동 실행(트레이에서 시작하고 열어 둔 스티커가 제자리에 다시 뜹니다), 새 스티커 크기, 매일 오늘 메모, 정렬할 모니터, 첫 화면, 메뉴 숨기기, 휴지통 보관 기간, 잠긴 메모 자동 잠금.
· 메인창·메모창: 테마(라이트·다크), 글꼴, 글자 크기, 투명도.
· 캘린더: 테마, 글자 크기, 완료 일정 표시, 보기 단위.
· 알람: 소리, 팝업 알림, 전체 알림 끄기. 알람 시각이 되면 화면 오른쪽 아래에 알림이 뜨고 열기·5분 뒤·확인을 고릅니다.
· 백업 / 복원: 데이터 위치, ‘지금 백업’, ‘백업에서 복원’. 백업은 데이터 폴더의 backups 안에 날짜별로 쌓입니다.
· 정보: 버전과 이 사용법 다시 보기.",
    },
];

/// Add the guide tab and its memos.
pub fn install(a: &mut App) {
    let tab = a.extra.wiki_tabs.len();
    a.extra.wiki_tabs.push(TAB.into());
    for (k, page) in PAGES.iter().enumerate() {
        let i = a.data.notes.len();
        let mut n = Note::new(page.title, None, k);
        n.wiki = true;
        n.wiki_tab = Some(tab);
        n.color = PALETTE[k % PALETTE.len()];
        n.body = format!("{}\n", page.intro);
        if let Ok(img) = image::load_from_memory(page.image) {
            let img = img.to_rgba8();
            let size = [img.width() as usize, img.height() as usize];
            let picture = Picture::from_rgba(
                0,
                format!("{}.jpg", page.title),
                img.into_raw(),
                size,
                1.,
                true,
            );
            n.document.tabs[0].blocks.push(Block::Picture(picture));
            n.body.push(placeholder(0));
            n.body.push('\n');
            n.has_image = true;
        }
        n.body.push_str(page.body);
        a.data.notes.push(n);
        let _ = i;
    }
}

impl App {
    /// Memos of the guide tab stay in the wiki: the to-do lists skip them.
    pub fn is_guide(&self, i: usize) -> bool {
        self.data.notes[i]
            .wiki_tab
            .and_then(|t| self.extra.wiki_tabs.get(t))
            .is_some_and(|t| t == TAB)
    }
    /// Show the usage guide: My wiki, the guide tab, first page in area 1.
    pub fn open_guide(&mut self) {
        self.page = 1;
        if let Some(tab) = self.extra.wiki_tabs.iter().position(|t| t == TAB) {
            self.extra.wiki_tab = tab;
            let first = self.data.notes.iter().position(|n| {
                n.wiki && !n.deleted && n.wiki_tab == Some(tab) && n.folder.is_none()
            });
            if first.is_some() {
                self.extra.wiki_panes[0] = first;
                self.extra.wiki_focus = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_new_workspace_holds_the_guide_and_opens_it() {
        let mut a = App::sample(Local::now().date_naive());
        a.start_fresh();
        let tab = a.extra.wiki_tabs.iter().position(|t| t == TAB).unwrap();
        let pages: Vec<_> = a
            .data
            .notes
            .iter()
            .filter(|n| n.wiki_tab == Some(tab))
            .collect();
        assert_eq!(pages.len(), PAGES.len());
        // Every page carries its screenshot.
        assert!(pages.iter().all(|n| n.has_image
            && matches!(n.document.tabs[0].blocks.first(), Some(Block::Picture(p)) if p.width > 0)));
        a.open_guide();
        assert_eq!(a.page, 1);
        assert_eq!(a.extra.wiki_tab, tab);
        assert!(a.extra.wiki_panes[0].is_some());
    }
}
