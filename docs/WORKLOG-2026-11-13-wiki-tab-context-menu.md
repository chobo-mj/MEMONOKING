# 2026-11-13 위키 탭 우클릭 메뉴: 색바꾸기/삭제 추가 (#60)

## 요청
"위키 탭 우클릭 메뉴 (색바꾸기/삭제 추가)" — 나만의 위키 상단 탭을 우클릭했을 때 "이름 바꾸기"
외에 색 바꾸기와 삭제 항목도 있어야 한다는 요청.

## 조사
- `src/refinements.rs`의 `wiki_tabs_bar()`에서 탭의 우클릭 메뉴(`r.context_menu`)에는 "이름
  바꾸기" 하나뿐이었다.
- 탭은 지금까지 `Vec<String>`(이름만)으로 저장되어 색상 개념이 없었고, 탭 삭제 기능도 전혀
  없었다.
- 프로젝트 삭제(`deleted_projects: HashSet<usize>`)가 이미 "실제로 벡터에서 제거하지 않고
  숨김 집합에 인덱스만 추가"하는 방식을 쓰고 있어, 같은 패턴을 위키 탭에도 적용했다. 벡터에서
  실제로 제거하면 `Folder::tab`이 가리키는 인덱스가 전부 틀어지므로, 재배치가 필요 없는 소프트
  삭제가 더 안전하다고 판단했다.

## 가정 (명시)
- "삭제"는 소프트 삭제로 구현했다: 탭이 탭 바에서 사라지고 선택할 수 없게 되지만, 탭 이름·색상과
  그 탭에 속한 위키 폴더/메모 데이터 자체는 그대로 남아 있다(프로젝트 삭제와 동일한 안전 장치).
  복원 UI(프로젝트 관리자 화면 같은 것)는 이번에 만들지 않았다 — 필요하시면 별도로 요청해주시면
  추가하겠다.
- 마지막 남은 탭 하나는 삭제할 수 없게 막았다(위키 화면 자체가 최소 탭 1개를 전제로 동작하기
  때문).
- 색상은 기존 PALETTE(메모 색상 등에서 쓰는 10색)를 그대로 재사용했다. 전용 색상표를 새로
  만들지는 않았다.

## 구현
- `src/extras.rs`: `Extra`에 `wiki_tab_colors: HashMap<usize, Color32>`(탭별 커스텀 색, 없으면
  기본 강조색)와 `deleted_wiki_tabs: HashSet<usize>`(숨긴 탭의 인덱스) 추가.
- `src/refinements.rs`
  - 탭 바 렌더링에서 숨겨진 탭은 건너뛰고, 탭 채우기 색을 커스텀 색(선택 시 원색, 비선택 시
    배경과 블렌딩한 톤)으로 바꿨다.
  - 탭의 현재 선택 인덱스가 범위를 벗어나거나 삭제된 경우 다음으로 넘어가는 로직을
    `next_wiki_tab()`라는 순수 함수로 분리해, 처음 로드 시의 보정과 삭제 직후의 선택 이동 양쪽에서
    재사용했다.
  - 우클릭 메뉴에 "색 바꾸기"(PALETTE 10색 스와치 서브메뉴, 현재 색에 테두리 표시)와 "삭제"(탭이
    하나만 남았을 때는 비활성화)를 추가.
  - 새 테스트 `next_wiki_tab_skips_deleted_tabs_and_falls_back_to_zero`,
    `deleting_the_active_wiki_tab_moves_selection_to_the_next_one` — 선택 이동 로직과 소프트
    삭제가 데이터를 보존하는지 확인.

## 시험
- Linux: `cargo test` 132개 중 131 통과(윈도우 전용 글꼴 경로 테스트 1개는 기존 실패, 회귀 아님),
  `cargo clippy --all-targets` 경고 없음, `cargo fmt --check` 통과.

## 후속 수정 — 삭제 확인 경고창 추가 (비주얼 검증에서 요청)

"삭제" 버튼을 누르면 확인 없이 바로 소프트 삭제되던 것을, `delete_project`와 동일한 패턴(대기 상태
+ 확인 다이얼로그)으로 바꿨다.

- `src/extras.rs`: `Extra`에 `delete_wiki_tab: Option<usize>`(영속화되지 않는 일시 상태, 기존
  `wiki_rename`과 같은 위치) 추가.
- `src/refinements.rs`: 탭 컨텍스트 메뉴의 "삭제" 클릭은 이제 바로 지우지 않고
  `self.extra.delete_wiki_tab = Some(t)`만 설정한다. `wiki()` 끝에서 호출하는 새
  `wiki_tab_delete_dialog()`가 "‘{이름}’ 탭을 정말 삭제하시겠습니까?" 문구와 삭제/취소 버튼을
  보여주고, 확인을 누른 경우에만 실제 삭제 로직(`confirm_wiki_tab_delete`, 기존
  `deleted_wiki_tabs` 소프트 삭제 + 선택 이동)을 수행한다. 취소하거나 다이얼로그를 닫으면 아무
  변화 없이 대기 상태만 비운다.
- 새 테스트 `wiki_tab_delete_asks_for_confirmation_before_it_takes_effect` — 대기 상태로 설정한
  것만으로는 삭제되지 않고, 취소해도 그대로 남아 있으며, `confirm_wiki_tab_delete`를 호출해야만
  실제로 삭제되는지 확인.

### 삭제된 탭 복원(휴지통) — 아직 미해결
사용자가 삭제한 위키 탭이 메모 휴지통(`trash_ui`)에 보이지 않는다고 확인했다. 현재 "삭제"는
`deleted_wiki_tabs`에 숨기는 소프트 삭제라 데이터 자체(이름/색/그 안의 폴더·메모 분류)는 남아
있지만, 이를 되돌리는 UI가 전혀 없다(이는 애초에 #60 작업 당시 작업 범위 밖으로 명시했던 부분).
위키 탭은 `Note`가 아니라서 기존 `휴지통` 화면이 쓰는 표(`catalog_table`/`TableRow`) 구조에 바로
끼워 넣기 어렵다 — 어떤 방식으로 복원 기능을 원하시는지(휴지통 화면에 통합할지, 위키 화면 자체에
작은 "삭제된 탭 복원" 목록을 둘지) 확인 후 별도로 진행하기로 함.

- `cargo build`/`cargo clippy --all-targets`/`cargo fmt --check`/`cargo test` 모두 통과.
