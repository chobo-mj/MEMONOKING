# OmniMemoJang 코드 아키텍처

OmniMemoJang(이전 이름 MEMONOKING)은 Windows용 개인 메모·일정 작업 공간이다. 이 문서는 지금 코드가 어떻게 짜여 있는지와, 최종판을 완성하려면 무엇을 더 붙여야 하는지를 정리한다. 기능 요구사항은 [SPEC.md](SPEC.md)에 있다.

## 1. 기술 스택

| 항목 | 선택 | 이유 |
|---|---|---|
| 언어 | Rust 2021 | 단일 실행 파일, 런타임 설치 불필요 |
| UI | eframe / egui 0.31.1 (glow 백엔드) | 즉시 모드 UI. 메모 창마다 OS 창(viewport)을 쓴다 |
| 창 시스템 | winit 0.30 (eframe 내장) | 여러 OS 창 관리 |
| Windows API | windows-sys 0.59 | 창 모양·투명도, 바탕화면 고정, 모니터 정보, 마우스 훅 |
| 날짜 | chrono | 일정·반복·완료 시각 |
| 저장 | rusqlite(bundled SQLite), serde_json | 메모별 행 저장, 바뀐 메모만 쓰기 |
| Windows | tray-icon, windows-sys | 트레이 아이콘, 단일 실행, 자동 실행, 창 스타일 |
| 기타 | global-hotkey, arboard, rfd, image, fontdue, scraper | 전역 단축키, 클립보드, 파일 대화상자, 이미지, 표 이미지화, 링크 미리보기 |

빌드는 `cargo build --release`이며 결과물은 `omnimemojang.exe` 하나다. 배포 구성은 `packaging/OmniMemoJang.iss`(Inno Setup)와 `.github/workflows/release.yml`이다.

## 2. 창 구조

```
루트 호스트 (1×1 투명, 보이지 않음)
├─ 메인 창            main_window()        메뉴창 + 각 페이지
├─ 스티커 메모 창 N개  studio::stickers()   메모 하나당 OS 창 하나 (스티커/편집/제목/색상상자)
│   └─ 메뉴 창         studio::menu_window  더보기·우클릭·프로젝트·연결 메모 팝업 (+ 하위 메뉴 창)
├─ 캘린더 분리창       calendar_dock        또는 캘린더 상자 / 스탠딩 캘린더
├─ 대화상자 창들       ui::dialog           설정, 알람, 일정, 잠금, 찾아바꾸기, 완료 범위 …
└─ 모니터 선택 창      studio::monitor_picker
```

- **루트 호스트:** 모든 창은 egui의 *immediate viewport*이고, 보이지 않는 루트가 이들을 거느린다. 그래서 메인 창을 최소화해도 스티커는 바탕화면에 그대로 남는다.
- **창 모양:** 대부분의 GPU에서 OpenGL 창은 픽셀 단위 투명을 지원하지 않는다. 그래서 테두리 없는 창은 불투명한 사각형으로 그리고 둥근 모서리(DWM)와 창 전체 투명도(레이어드 창)는 Windows가 맡는다. 이 처리는 `win_style.rs`에 있다. winit이 창 스타일을 다시 쓰면 레이어드 속성이 사라지므로 매 프레임 확인해 다시 적용한다.
- **창 이동·크기:** 이동과 크기 조절은 OS에 맡긴다. 앱은 결과를 관찰만 하고, 크기·위치 요청을 매 프레임 되먹이지 않는다. 이 처리는 `native_window.rs`에 있다.
- **반복 갱신:** 루트가 100ms마다 다시 그려, 매일 메모 05:00 전환·알람·스탠딩 캘린더 같은 시간 기반 동작을 처리한다.

## 3. 모듈 구성 (src/)

| 모듈 | 책임 |
|---|---|
| `main.rs` | `App` 상태, 시작(글꼴·설정 불러오기·단축키), 프레임 순서, 메뉴창(사이드바)·페이지 전환, 전역 대화상자 |
| `model.rs` | `Note`, `Workspace`. 일정·기간·반복 발생(`occurs`), 날짜별 완료, 연결(parents) DAG |
| `repeat.rs` | 반복 규칙(매주·격주·매월 n째 주 …)과 설명 문자열 |
| `storage.rs` | 데이터 저장·불러오기(SQLite `data.db`, 메모별 행), 그림 파일·지연 읽기·썸네일, 입력 기반 자동 저장, 열린 메모 창 복원, 백업, 비밀번호 해시, 시작(`load_or_start`) |
| `tray.rs` | 트레이 아이콘·메뉴(열기/새 스티커 메모/종료), 단일 실행(두 번째 실행은 메인 창만 다시 띄움) |
| `lists.rs` | 긴 목록: 화면에 보이는 행만 그리기(`lazy_row`), 거르고 정렬한 목록 캐시(`Cache`) |
| `guide.rs` | 사용법 위키 탭(메뉴별 메모 10개와 화면 이미지, `assets/guide/*.jpg`를 실행 파일에 포함) |
| `alarm.rs` | 알람 확인과 오른쪽 아래 알림 창 |
| `system.rs` | Windows 자동 실행(Run 키, `--tray`로 트레이에서 시작), 알림 소리 |
| `prefs.rs` | 재시작 후에도 남는 설정(첫 화면, 숨긴 메뉴, 캘린더 옵션)의 저장·불러오기 |
| `ui.rs` | 색·톤(`Tone`), 공용 위젯(pill, segmented, toggle_row, 검색창, 시간 입력·달력), 대화상자 창, 배너 |
| `icons.rs` | 벡터 아이콘 그리기와 아이콘 버튼 |
| `studio.rs` | 스티커 창: 모드(스티커/편집/제목/색상상자), 머리·바닥 도구, 메뉴 창, 정렬(arrange), 매일 메모, 모니터 선택 |
| `composer.rs` | 메모 본문 편집기: 탭, 서식, 표, 그림, 도구막대 |
| `schedule.rs` | 일정 대화상자(시작·끝·반복) |
| `views.rs` | 내 작업실·오늘/다음 할 일 목록, 할 일 행, 캘린더(월/1주/2주, 연속 일정 색띠), 메모 편집 보조 |
| `calendar_dock.rs` | 캘린더 분리창, 캘린더 상자, 스탠딩 캘린더, 캘린더 테마 |
| `catalog.rs` | 전체메모 목록·폴더, 휴지통, 찾아 바꾸기 |
| `refinements.rs` | 기간 일정 완료 범위, 나만의 위키(탭·트리·미분류·두 영역), 프로젝트 필터 도우미 |
| `extras.rs` | 부가 상태(`Extra`), 댓글, 일지, 날짜 선택기, 프로젝트 관리, 편의기능 |
| `desktop.rs` | 설정 화면, 앱 안 미리보기 편집(`inline_note`), 잠금·알람 대화상자, 휴지통 이동 |
| `dragging.rs` | 창을 넘나드는 메모 끌어놓기(날짜·연결·위키 분류) |
| `media.rs`, `content_collections.rs` | 이미지·표·링크 모음, 내보내기 |
| `native_window.rs` | 창 이동·크기 관찰, 작업 영역·모니터 목록 |
| `win_style.rs` | 창 모양·투명도, HWND 찾기, 바탕화면(WorkerW) 고정, 저수준 마우스 훅 |

## 4. 데이터 모델

```
App
├─ data: Workspace
│   ├─ notes: Vec<Note>      (인덱스가 곧 메모 ID. 지울 때는 deleted 표시만 한다)
│   ├─ projects: Vec<String>
│   └─ links: Vec<(usize, usize)>
├─ desktop: Desktop          설정 값 (data.db의 workspace 행 settings로 저장)
├─ prefs: Prefs              시작 전에 필요한 설정 (settings.txt)
├─ saving: Saving            자동 저장 상태 (마지막으로 쓴 메모별 레코드, DB 연결)
├─ tray / main_open          트레이 아이콘, 메인 창 표시 여부
├─ lists: lists::Cache       거르고 정렬한 목록 캐시
├─ alarms: Alarms            지금 떠 있는 알람
├─ extra: Extra              위키 폴더·탭·영역, 필터, 댓글 초안 …
├─ catalog: Catalog          전체메모 폴더·선택·찾아바꾸기 상태
├─ studio: Studio            열린 스티커 창들(Window: 모드·크기·메뉴)
└─ cal: Dock                 캘린더 분리창·상자·스탠딩·테마
```

`Note`의 주요 필드:

| 필드 | 뜻 |
|---|---|
| `title`, `body`, `document` | 제목, 본문, 서식 문서(탭·표·그림) |
| `date`, `duration`, `time`, `end_time`, `repeat` | 일정 |
| `done` / `completed` | 하루짜리 일정의 완료 |
| `occurrence_completed` | 기간 일정의 날짜별 완료 |
| `reminder` | 알람 |
| `project` | 프로젝트 |
| `parents` | 연결 메모 |
| `starred`, `locked`, `color` | 중요, 잠금, 색 |
| `wiki`, `wiki_tab`, `folder`, `wiki_comments` | 위키 속성·분류·댓글 |
| `deleted`, `purged` | 휴지통 |

## 5. 프레임 흐름

`eframe::App::update`가 매 프레임 아래 순서로 실행된다.

1. **설정 처리:** 테마를 적용하고 전역 단축키를 처리한다. `keep_prefs()`로 바뀐 설정을 저장하고, 숨긴 메뉴가 현재 페이지면 내 작업실로 돌아간다.
2. **창 그리기:** 메인 창(`main_window`)을 그린 다음 캘린더 분리창·상자(`calendar_dock`)를 그린다.
3. **자동 동작:** `daily_today`(05:00 기준 오늘 메모 띄우기), `monitor_picker`, `apply_arrangement`(스티커 자동 정렬)를 차례로 실행한다.
4. **나머지 창:** 스티커와 기타 창, 전역·설정 대화상자를 그린다.
5. **마무리:** `check_alarms`(알람 창), `auto_relock`(잠금 메모 다시 잠그기), `apply_autostart`(자동 실행 설정 반영), `autosave`(입력이 있었으면 2초마다, 없으면 1분마다 바뀐 메모만 저장)를 실행한다.

창 사이 끌어놓기는 각 창이 그릴 때 `sources`와 `targets`를 등록하고, `drag_input`이 포인터 사건으로 놓을 곳을 결정하는 방식이다.

## 6. 저장 계층과 남은 부분

**저장 (`storage.rs`)**

- **형식:** SQLite `data.db`(WAL). `notes(id, rec)`에 메모 하나가 한 행(JSON), `meta('workspace')`에 프로젝트·연결·위키 탭/폴더·전체메모 폴더·휴지통 시각·비밀번호 해시·사용자 설정·열린 메모 창(스티커 위치·크기·접힘 상태)을 둔다. 0.3의 `data.json`은 처음 시작할 때 DB로 옮기고 `data.before-db.json`으로 남긴다. 옮긴 뒤에 이전 버전이 `data.json`을 다시 쓰면(DB보다 새 파일) DB를 백업하고 그 새 내용을 가져온다.
- **그림:** `images\<내용 해시>.png`로 한 번만 쓰고 지우지 않는다. 저장된 그림은 메모리에 픽셀을 두지 않고(`Picture::file`), 화면에 보일 때만 파일을 읽어 텍스처로 만든다. 목록·갤러리는 `images\thumbs\`의 320px 썸네일을 쓴다. 그래서 그림이 수십 GB여도 시작 시간과 메모리는 메모 글 양에만 비례한다.
- **그림:** 메모 안 그림은 픽셀 내용의 해시를 이름으로 `images\<hash>.png`에 한 번만 쓴다. JSON에는 파일 이름만 남는다.
- **자동 저장:** 모든 창의 입력(키·클릭·휠·파일 끌어놓기)을 `storage::watch_input`이 표시한다. 표시가 있으면 2초 뒤, 없으면 1분마다 메모별 레코드를 마지막으로 쓴 것과 비교해 바뀐 행만 한 트랜잭션으로 쓴다. 메인 창을 트레이로 내릴 때와 종료할 때(`on_exit`, 트레이 ‘종료’)도 저장한다. 전원을 그냥 꺼도 마지막 입력 2초 뒤까지는 남는다.
- **시작:** `load_or_start`가 `data.db`(없으면 `data.json`)를 읽고, 열려 있던 메모 창을 같은 자리·모드로 다시 연다(포커스는 가져가지 않음). 없으면 빈 작업 공간에 사용법(`guide.rs`)을 넣는다. 읽을 수 없으면 파일을 `data.broken-*.db/json`으로 옮기고 새로 시작한다.
- **백업/복원:** `VACUUM INTO`로 `backups\<날짜시각>\data.db`를 만든다. 그림은 바뀌거나 지워지지 않으므로 복사하지 않고 모든 백업이 `images\`를 함께 쓴다. 복원은 `data.db`와 이전 버전의 `data.json`(+그 폴더의 그림)을 모두 받는다. 복원 전에 현재 데이터를 한 번 더 백업한다.
- **데이터 폴더 변경:** 새 폴더에 전부 다시 쓰고, 이전 폴더의 그림은 백그라운드로 복사하며 그동안 이전 폴더에서 읽는다.

**트레이와 단일 실행 (`tray.rs`)**

- 메인 창 X는 종료가 아니라 트레이로 내린다(`main_open = false`). 트레이 아이콘 클릭·‘열기’로 다시 띄우고, ‘종료’로만 끝난다. 트레이 아이콘을 만들 수 없으면 예전처럼 X가 종료한다.
- 자동 실행 값은 `"…\OmniMemoJang.exe" --tray`이다. `--tray`로 시작하면 메인 창 없이 트레이와 스티커만 뜬다. 켜 둔 자동 실행은 시작할 때 다시 써서 0.3에서 올라온 값도 바뀐다.
- 이름 있는 뮤텍스로 한 번만 실행한다. 두 번째 실행은 이름 있는 이벤트로 첫 실행에 메인 창을 띄우라고 알리고 끝난다(같은 DB를 두 프로세스가 쓰지 않도록).

**긴 목록 (`lists.rs`)**

- 내 작업실·오늘/다음에 할일·상세 목록·전체메모 표는 `lazy_row`로 화면에 보이는 행만 그린다. 안 보이는 행은 마지막으로 그린 높이만 차지한다. 선택된 행은 항상 그린다(키보드 이동 때 스스로 화면에 들어오도록).
- 거르고 정렬한 행 목록은 `lists::Cache`에 두고, 입력이 있었거나(입력이 있던 프레임 끝에 한 번 더) 날짜·메모 수가 바뀔 때만 다시 계산한다. 입력이 없는 100ms 주기 다시 그리기에서는 다시 계산하지 않는다.
- **경계:** UI 모듈은 `App` 상태만 바꾸고 디스크를 직접 만지지 않는다. 디스크는 `storage`와 `prefs`만 다룬다.

**알람 (`alarm.rs`)과 시스템 연동 (`system.rs`)**

- 1초마다 알람 시각이 지난 메모를 모아 소리(`MessageBeep`)와 오른쪽 아래 알림 창으로 알린다. 확인하면 메모의 알람이 지워지므로 다음 실행 때 다시 뜨지 않는다.
- 자동 실행은 `HKCU\…\Run`의 `OmniMemoJang` 값으로 켜고 끈다(관리자 권한 불필요).

**남은 부분**

| 영역 | 현재 | 다음 단계 |
|---|---|---|
| 근무자 보기 | 빈 버튼(캘린더 머리줄) | `duty` 모듈. 연결한 엑셀 파일(calamine으로 읽기)에서 날짜별 근무자를 읽어 캘린더 칸에 표시한다. 파일 경로는 `prefs`에 둔다 |
| 잠금 메모 본문 | 비밀번호는 해시 저장, 본문은 평문 | 비밀번호에서 키를 만들어(argon2) 본문을 암호화해 저장한다 |
| 알림 센터 | 자체 알림 창과 소리 | Windows 토스트(AUMID 등록) |
| 메모 ID | 배열 인덱스 | 지운 메모는 내용만 비운 채 자리를 지킨다(`purged`). 메모가 아주 많아지면 고정 ID로 바꾼다 |
| 큰 모듈 정리 | `catalog.rs`, `composer.rs`가 2,800줄 | 기능 단위로 쪼갠다. 예: composer → 표·그림·서식 하위 모듈 |

## 7. 플랫폼 연동 지점 (Windows)

- **창 모양:** HWND를 창 제목과 화면 위치로 찾는다(`win_style::find`). egui가 보조 창의 핸들을 주지 않기 때문이다.
- **스탠딩 캘린더:** Progman에 0x052C 메시지를 보내 WorkerW를 만들고, 캘린더 창을 그 자식으로 붙인다. Windows 11 24H2에서는 Progman의 자식 WorkerW를 쓴다. 바탕화면 아이콘이 위에 오므로 캘린더는 입력을 받지 못한다. 그래서 해제 클릭은 저수준 마우스 훅(`WH_MOUSE_LL`)을 전용 스레드에서 돌려 감지한다.
- **모니터 목록:** `EnumDisplayMonitors`로 얻는다.
- **좌표 단위:** 화면 좌표는 물리 픽셀로 다룬다. egui 포인트에 `native_pixels_per_point`를 곱해 바꾼다.

## 8. 테스트와 검증

- **단위 테스트(`cargo test`, 111개):** 일정·반복·완료 범위, 끌어놓기, 찾아 바꾸기, 위키 영역 규칙, 연속 일정 배치, 매일 메모 05:00 경계, 설정 저장 왕복, 데이터 저장·복원(그림·비밀번호 해시 포함), 알람 대상, 사용법 설치, 바뀐 메모만 저장·스티커 창 복원, data.json→DB 이전·DB 백업, 목록 캐시·보이는 행만 그리기, 트레이 아이콘을 다룬다.
- **정적 검사:** Linux에서 `cargo clippy --target x86_64-pc-windows-msvc`로 Windows 전용 코드까지 검사한다.
- **수동 검증:** 실제 Windows PC에서 확인하고 결과를 `docs/WORKLOG-*.md`에 남긴다. 화면 확인은 Xvfb 스크린샷을 쓴다.
- **테스트 시계:** `OMNIMEMOJANG_TEST_NOW="YYYY-MM-DD HH:MM:SS"`로 매일 메모의 시계를 지정할 수 있다.
