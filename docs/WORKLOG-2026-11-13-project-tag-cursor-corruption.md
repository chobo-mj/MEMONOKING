# #62: `#프로젝트명` 입력 시 텍스트 깨짐 버그 수정

## 증상

메모 본문에서 `#프로젝트명` 뒤에 공백을 입력하면 해당 텍스트가 프로젝트 태그로
변환되고 본문에서 제거되는데, 이 변환 직후부터 입력하는 텍스트(특히 한글)가
깨지는 문제가 보고되었습니다. 이는 기존 요구사항 문서(`docs/ppt-requirements-v18.md`,
P18-S14.4)의 "`#project Space·인라인체크·취소선, 변환뒤 한글조합 정상`" 항목과도
일치하는, 변환 이후 한글 조합이 정상적으로 유지되어야 한다는 요구사항입니다.

## 원인

`src/desktop.rs`의 `consume_body_project`는 메모 본문 문자열(`n.body`, "legacy body")에서
`#프로젝트명 ` 접두사를 매 프레임 감지해 잘라내고 프로젝트를 연결합니다. 이 함수는
`crate::composer::show_with`/`content` 호출(실제 리치 에디터 렌더링) **다음에** 매
프레임 호출되며, `n.body`를 egui `TextEdit` 위젯 바깥에서 직접
`String::replace_range`로 수정합니다.

문제는 `src/composer.rs`의 `content_impl`이 다음 프레임에 `doc.tabs[0].body`가
외부에서 바뀐 `legacy_body`와 다르다는 것을 감지해 내부 문서 본문을 갱신하지만
(기존 코드), 그 시점에 **egui `TextEdit`가 이전 프레임에 저장해 둔 커서/IME
상태(char 인덱스 기준)는 그대로 남아 있었다는 점**입니다. 텍스트가 바뀐 폭만큼
커서 인덱스가 더 이상 유효하지 않게 되어, 다음 입력(특히 한글 IME 조합 중인
입력)이 잘못된 위치에 삽입/삭제되며 텍스트가 깨지는 현상으로 나타났습니다.

## 수정

`src/composer.rs`에 `remap_cursor(old, new, idx)` 순수 함수를 추가했습니다. 바뀌기
전/후 텍스트의 공통 접두사·접미사를 비교해:

- 커서가 공통 접두사 안에 있으면 그대로 유지
- 커서가 공통 접미사 안에 있으면 끝으로부터의 거리를 유지
- 커서가 바뀐 중간 구간 안에 있었으면 공통 접두사 바로 뒤로 보정

`content_impl`에서 `doc.tabs[0].body != *legacy_body`를 감지해 본문을 갱신하는
지점(외부에서 본문이 바뀐 것을 감지하는 유일한 지점)에, 활성 탭이 0번일 때
저장된 `TextEdit` 커서 상태를 이 함수로 재계산해 다시 저장하도록 했습니다.

이 수정은 `consume_body_project`에 특화된 코드가 아니라 `content_impl`의 공용
동기화 지점에 있으므로, legacy body를 외부에서 바꾸는 다른 경로(예:
`catalog.rs`의 찾기/바꾸기)에도 동일하게 적용됩니다. `desktop.rs`나
`consume_body_project` 자체는 수정하지 않았습니다.

## 가정 및 범위

- 원인을 "egui TextEdit의 영속 커서 상태가 외부 본문 변경을 따라가지 못함"으로
  특정했습니다. 코드 추적(IME 이벤트 처리, `content_impl`의 동기화 순서,
  `consume_body_project`의 호출 시점)으로 확인했으며, 실제 PC 환경에서 한글
  IME로 재현해 보지는 못했습니다(이 라운드는 비주얼 검증을 보류하기로 한
  지침에 따름). 비주얼 검증 단계에서 실제로 `#프로젝트명 ` + 스페이스 입력 후
  이어서 한글을 입력했을 때 더 이상 깨지지 않는지 확인이 필요합니다.
- `src/composer.rs`에 동일한 `#` 프로젝트 태그 처리 로직이 따로 있는지 확인했으나
  없었습니다. 태그 변환 자체는 `desktop.rs`에만 있고, `composer.rs`는 그 결과로
  바뀐 `legacy_body`를 받아들이는 입장입니다.

## 테스트

`src/composer.rs`에 추가:

- `remap_cursor_moves_to_the_cut_point_when_a_leading_prefix_is_removed`
- `remap_cursor_keeps_a_position_inside_an_untouched_common_prefix`
- `remap_cursor_clamps_a_position_inside_the_replaced_middle_region`
- `project_tag_strip_remaps_a_live_cursor_instead_of_leaving_it_stale`
  (실제 `content_impl`을 두 프레임 걸쳐 구동해, 수정 전에는 실패하고 수정 후에는
  통과함을 확인한 종단 테스트)

`cargo build` / `cargo clippy --all-targets` / `cargo fmt --check` / `cargo test` 모두
통과(기존부터 있던, Windows 전용 폰트 경로 때문에 Linux에서 실패하는
`content_collections::tests::table_export_preserves_variable_columns_and_escapes_html`
1건은 이 수정과 무관하며 그대로 남아 있음).
