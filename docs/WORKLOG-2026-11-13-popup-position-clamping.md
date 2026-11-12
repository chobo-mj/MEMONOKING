# 2026-11-13 화면 밖으로 나가는 팝업 위치 보정 (#57)

## 요청
"화면 밖으로 나가는 팝업 위치 보정" — 어떤 팝업/메뉴가 화면 밖으로 나가는지 구체적으로 지정되지
않아, 코드 전체를 조사해 실제로 화면/보이는 영역 밖으로 나갈 수 있는 지점을 찾아 모두 고쳤다.

## 조사 결과 (실제로 위치 보정이 없던 3곳)
1. **스티커 더보기 메뉴 (`menu_window`, `src/studio.rs`)** — 가장 심각한 경우. 이 메뉴는 실제
   OS 창(테두리 없는 always-on-top 뷰포트)으로 열리는데, 화면 좌표를 그대로 `with_position`에
   넘겨서 전혀 보정하지 않았다. 스티커가 모니터 오른쪽/아래쪽 끝에 가까우면 메뉴가 화면 경계
   밖으로 열려 일부가 가려지거나 아예 안 보일 수 있다. 캘린더 상자 메뉴(`calendar_box_menu`)도
   같은 함수를 쓰므로 함께 해결된다.
2. **캘린더 우클릭 메뉴 (`calendar_view_menu`, `src/calendar_dock.rs`)** — 포인터 위치를 그대로
   `fixed_pos`에 넘겨서, 캘린더 창 오른쪽/아래쪽 끝 근처에서 우클릭하면 메뉴가 잘릴 수 있었다.
3. **그림 선택 시 뜨는 크기/복사/삭제 도구바 (`picture_toolbar_pos`, `src/composer.rs`)** — 그림
   위쪽에 고정 오프셋(-34px)으로 띄우는데, 그림이 스크롤 영역 맨 위 근처에 있으면 도구바가 보이는
   영역 위로 잘려 나갔다.

## 구현
- `src/ui.rs`: 새 공용 헬퍼 `clamp_popup_pos(ctx, pos, size)` 추가 — 현재 뷰포트의
  `screen_rect()` 안에 들어오도록 앵커 위치를 보정한다. 테스트
  `clamp_popup_pos_keeps_an_anchor_inside_the_screen`로 중앙/모서리/음수 좌표 3가지 확인.
- `src/studio.rs`: `menu_window()`가 뷰포트를 만들기 전에 `crate::native_window::work_area(ctx)`
  (모니터 작업 영역, 작업표시줄 제외)로 위치를 `fit_in()`(기존 함수 재사용)해서 클램프하도록 수정.
- `src/calendar_dock.rs`: `calendar_view_menu()`에서 `fixed_pos`로 넘기기 전에
  `ui::clamp_popup_pos`로 보정.
- `src/composer.rs`: 그림 도구바 위치 계산을 `picture_toolbar_pos()`로 분리하고, 위쪽에 자리가
  없으면 그림 아래쪽으로 내려가도록 변경. 테스트
  `picture_toolbar_drops_below_when_above_would_clip`로 양쪽 분기 확인.

## 가정 (명시)
- 요청에 특정 팝업이 지정되지 않아, 코드 조사로 찾은 보정 누락 지점 전부를 고쳤다. 사용자가 실제로
  겪은 경우가 이 중 다른 것(예: 표 안쪽 도구바, 설정 창의 특정 팝업)이라면 알려주시면 추가로
  보정하겠다.
- `menu_window`/`calendar_view_menu`의 클램프는 팝업이 **최종 크기로 리사이즈되기 전의 추정
  크기**를 기준으로 계산한다(실제 내용에 맞춰 다시 리사이즈되는 기존 동작은 그대로 유지). 추정치가
  실제보다 약간 작거나 크더라도 "화면 밖으로 완전히 나가는" 문제는 해결되고, 약간의 위치 오차는
  감수한다.

## 시험
- Linux: `cargo test` 127개 중 126 통과(윈도우 전용 글꼴 경로 테스트 1개는 기존 실패, 회귀 아님),
  `cargo clippy --all-targets` 경고 없음, `cargo fmt --check` 통과.
- `work_area()`의 실제 모니터 조회는 Windows 전용(`#[cfg(windows)]`)이라 Linux에서는 폴백 경로만
  검증된다. 실제 모니터 경계에서의 동작 확인은 비주얼 검증 때 요청.
