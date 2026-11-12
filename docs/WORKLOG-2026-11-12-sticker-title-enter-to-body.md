# 스티커 제목줄 엔터 시 본문으로 커서 이동

## 배경

요청: 스티커 메모의 제목 입력창에서 Enter를 누르면 커서가 본문으로 내려가야 한다.

## 구현

`src/studio.rs`의 스티커 제목 `TextEdit::singleline`에서 `response.lost_focus() &&
key_pressed(Enter)`(egui가 권장하는 표준 패턴)를 감지해, 본문 문서의
`requested_selection = Some(0..0)`을 거는 방식으로 구현했다. 이 필드는 새 스티커를
만들 때(`new_sticker()`) 본문에 바로 커서를 두는 데 이미 쓰이던 기존 메커니즘이다.

## 발견된 버그와 수정

1차 구현에서는 Enter를 감지한 **그 프레임 안에서 바로** `requested_selection`을
설정했는데, 사용자가 직접 테스트하면서 다음 두 가지 증상을 발견했다:

- 제목을 입력한 직후(커서를 옮기지 않고) 바로 Enter: 본문에 마지막 글자가
  중복 삽입되고 줄바꿈도 함께 생김.
- 제목 입력 후 오른쪽 방향키로 커서를 한 번 움직이고 Enter: 중복은 없지만 본문에
  빈 줄이 하나 추가되어 기존 내용이 한 줄 밀려 내려감.

원인: 제목 `TextEdit::singleline`의 Enter 처리(`return_key`)는 포커스만 넘길 뿐,
그 프레임의 입력 이벤트 목록에서 Enter 키 이벤트(및 IME 조합 중이었다면 그 커밋
이벤트)를 제거하지 않는다. 같은 프레임 안에서 본문에 포커스를 넘기면, 본문의
`TextEdit::multiline`이 (멀티라인이므로 Enter → 줄바꿈 삽입이 기본 동작) 아직
남아 있는 그 Enter 이벤트를 자신도 한 번 더 처리해 새 커서 위치(0)에 개행을
삽입했다. IME 조합이 끝나기 전이었다면 마지막 글자의 커밋 이벤트도 함께 남아있어
본문에 같은 글자가 한 번 더 삽입됐다.

수정: `Window`에 `move_caret_to_body` 플래그를 추가해, Enter 감지 시점에는 플래그만
세우고, **다음 프레임** 시작 시점(`self.theme(ctx)` 직후)에 플래그를 소비하며
`requested_selection`을 실제로 설정하도록 미뤘다. 한 프레임이 지나면 그 Enter/IME
이벤트는 이미 사라진 뒤이므로 본문 쪽이 다시 반응하지 않는다.

## 검증

- Linux: `cargo build`, `cargo clippy --all-targets`, `cargo fmt --check` 통과,
  `cargo test` 113/114 통과(폰트 관련 기존 실패 1건은 무관).
- Windows(PC): `cargo build`, `cargo test` 114/114 통과.
- 사용자가 PC에서 직접 재현 시나리오(즉시 엔터 / 방향키 후 엔터) 두 가지 모두
  다시 테스트해 정상 동작함을 확인함.

커밋: `238d3d9`(기능) → `841556f`(버그 수정) → 푸시 예정.
