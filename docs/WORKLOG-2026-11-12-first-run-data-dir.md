# 2026-11-12 — 최초 실행시 데이터 폴더 선택 안내

## 배경

사용자 요청 (대규모 작업 목록 중 1번 항목):
> "프로그램을 최초 실행할때 data를 저장할 폴더를 선택할수있게 안내문구와 함께 폴더 선택창을 띄워줘."

기존에는 최초 실행 시 사용자에게 묻지 않고 곧바로 Windows 앱 데이터 폴더
(`%APPDATA%\OmniMemoJang\data`)에 저장을 시작했다. 어디에 저장되는지 알거나
바꿀 방법이 없었다.

## 변경 내용

`src/storage.rs`의 `load_or_start()`에 다음을 추가:

- `prefs.first_run_done`이 아직 `false`이고 `prefs.data_path`가 비어있을 때만
  (= 진짜 최초 실행이고, 사용자가 이미 경로를 지정하지 않았을 때만)
  `prompt_first_run_data_dir()`를 호출한다.
- `prompt_first_run_data_dir()`:
  1. `rfd::MessageDialog`로 안내 문구를 띄운다 — 저장 위치를 고를 수 있다는 것과,
     선택하지 않으면 기본 위치(앱 데이터 폴더)에 저장된다는 것, 설정에서 나중에
     바꿀 수 있다는 것을 알린다.
  2. 이어서 `rfd::FileDialog::pick_folder()`로 네이티브 폴더 선택창을 띄운다.
  3. 폴더를 고르면 `data_path`에 그 경로를 저장한다. 취소하면 `data_path`는
     비어있는 그대로 남아 기존 기본 동작(앱 데이터 폴더)으로 이어진다.

기존 설정 화면의 "폴더 선택" 버튼과 동일한 `rfd` API를 사용했다 (새 의존성 없음).

이후 로직(`data_dir(&self.prefs.data_path)` 호출)은 그대로이므로, 사용자가
고른 경로든 비워둔 기본값이든 동일하게 처리된다.

## 검증

- Linux: `cargo build` / `cargo clippy --all-targets` / `cargo fmt -- --check`
  모두 깨끗하게 통과.
- Linux: `cargo test` — 113 passed, 1 failed (기존부터 있던, Windows 전용 폰트가
  없어서 Linux 컨테이너에서만 실패하는 `content_collections::tests::
  table_export_preserves_variable_columns_and_escapes_html` — 이번 변경과 무관,
  Windows에서는 정상 통과).
- Windows (실제 PC): `cargo build` 성공, `cargo test` — **114 passed, 0 failed**
  (위 테스트 포함 전부 통과).
- 기존 테스트들은 `App::new()`/`load_or_start()`의 최초 실행 경로를 거치지 않고
  `Self::sample(...)`로 직접 구성하므로, 새로 추가된 다이얼로그 호출 경로는
  기존 테스트에 영향이 없음을 확인.
- UI 직접 클릭 테스트(다이얼로그가 실제로 뜨는지)는 이번 라운드에서는 하지
  않음 — 코드 경로상 기존 설정화면의 폴더선택 버튼과 동일한 API이므로 동작은
  보장되나, 실제 최초 실행 시나리오(데이터 폴더가 없는 상태)를 인위적으로
  만들어 눈으로 확인하는 것은 다음 라운드에서 필요시 진행.

## 커밋

- `c86321e` — `Prompt for a data folder on first run`

## 남은 것

사용자가 보낸 대규모 작업 목록(총 20개) 중 1번만 처리했다. 나머지는 각각 별도
라운드로 순서대로 진행:

- 스탠딩 캘린더 사라짐 버그 (Win11 24H2)
- 스탠딩 캘린더 실시간 반영
- 캘린더 분리창 가로배치 모드
- 캘린더 최소화 컬러박스 항상위 해제
- 설정창 크기 조정
- 스티커 더블클릭 전체화면 토글
- 스티커 제목줄 엔터 → 본문 이동
- 표 붙여넣기 서식 버그
- 폰트 추가/수정
- 메모 리스트 컬럼 너비조정/표시선택
- '+ 새 메모' → '+ 새 일정'
- 캘린더 우클릭 메뉴
- 화면 밖 팝업 위치 보정
- 스티커 정렬 메뉴 재정비
- 위키 영역 너비 조절
- 위키 탭 우클릭 메뉴(색/삭제)
- `#프로젝트명` 입력시 텍스트 깨짐 버그 확인
