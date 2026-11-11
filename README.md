# OmniMemoJang

> 이전 이름: MEMONOKING. 구조는 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), 기능 명세와 구현 상태는 [docs/SPEC.md](docs/SPEC.md)에 있습니다.

Windows 10/11용 개인 메모·할 일·일정·위키 작업 공간입니다(Rust + egui/eframe). 메모 하나가 바탕화면의 스티커 창 하나이고, 내 작업실·캘린더·전체메모·나만의 위키에서 같은 메모를 다룹니다. 메모·위키·설정은 자동으로 저장되며, 처음 실행하면 나만의 위키의 ‘사용법’ 탭이 열립니다.

## 내려받기

최신 사전 배포는 [v0.3.0-ui-preview.1](https://github.com/chobo-mj/MEMONOKING/releases/tag/v0.3.0-ui-preview.1)입니다.

| 파일 | 용도 |
|---|---|
| [Windows x64 설치파일](https://github.com/chobo-mj/MEMONOKING/releases/download/v0.3.0-ui-preview.1/OmniMemoJang-0.3.0-ui-preview.1-Windows-x64-Setup.exe) | 현재 사용자에게 설치합니다(관리자 권한 불필요). 시작 메뉴 바로가기·선택적 바탕화면 바로가기·제거를 지원합니다 |
| [Windows x64 포터블 ZIP](https://github.com/chobo-mj/MEMONOKING/releases/download/v0.3.0-ui-preview.1/OmniMemoJang-0.3.0-ui-preview.1-Windows-x64-Portable.zip) | 압축을 푼 뒤 OmniMemoJang.exe를 실행합니다 |
| [SHA256SUMS.txt](https://github.com/chobo-mj/MEMONOKING/releases/download/v0.3.0-ui-preview.1/SHA256SUMS.txt) | 설치파일·ZIP 해시 대조용 |

설치 기본 위치는 `%LOCALAPPDATA%\Programs\OmniMemoJang`입니다. 데이터는 `%APPDATA%\OmniMemoJang`에 저장되며 프로그램을 제거해도 남습니다. 실행 PC에 Rust·C++ 빌드 도구나 Visual C++ 런타임이 필요하지 않습니다. 배포 파일은 정적 C 런타임 검사와 설치·제거 검사를 통과한 것만 게시합니다. 자세한 내용은 [설치·패키징 안내](packaging/README.md)에 있습니다.

## 소스에서 실행

Rust stable과 Windows C++ 빌드 도구를 설치한 뒤 `cargo run --release --locked`를 실행합니다. 빌드할 때 의존성 다운로드에 인터넷이 필요합니다. 설치된 Windows 글꼴을 쓰고, 없는 글꼴은 기본 글꼴로 표시합니다.

## 주요 기능

- **스티커 메모:** 스티커 → 제목 한 줄 → 색상 상자로 접힙니다(상자는 한 번 클릭하면 펼쳐짐). 메모마다 투명도, 항상 위, 본문 탭, 서식, 표·이미지 붙여넣기, 프로젝트·연결 메모, 알람·일정을 둘 수 있습니다. 더보기와 우클릭 메뉴(정렬 포함)는 스티커 밖 별도 창에 뜹니다.
- **내 작업실 · 오늘의 할일 · 다음에 할일:** 날짜별 목록, 오른쪽 미리보기에서 바로 편집, 필터(프로젝트·정기 일정·생성일 기준·완료 메모 보기), 찾아 바꾸기.
- **캘린더:** 작성일·일정·알람 기준, 여러 날 일정은 색띠 하나, 한 달·1주·2주 보기, 완료 일정 숨기기·취소선, 테마(기본·종이·세이지·나이트). 근무자 보기 버튼은 자리만 있습니다.
- **캘린더 분리창:** 최소화하면 이동·크기 조절·색 변경이 되는 달력 상자로 접히고, 스탠딩 캘린더로 바탕화면에 고정할 수 있습니다. 우클릭으로 보기 단위를 바꿉니다.
- **나만의 위키:** 탭별 폴더 트리, 미분류 목록, 두 영역 나란히 보기, 트리 우클릭 새 노트, 끌어서 분류합니다.
- **알람:** 시각이 되면 소리와 함께 오른쪽 아래에 알림 창이 뜹니다(열기·5분 뒤·확인). 꺼져 있던 동안 지난 알람도 알려 줍니다.
- **설정:** 첫 화면, 메뉴 숨기기, 새 스티커 크기, 매일 오늘 메모 자동 띄우기(05:00 기준), 정렬 모니터, Windows 시작 시 자동 실행(트레이에서 시작, 스티커 복원), 휴지통 보관 기간, 잠금 메모 자동 잠금, 백업·복원, 전역 단축키(기본 Ctrl+Alt+N 빠른 메모·새 스티커, Ctrl+Alt+C 캘린더 분리창, Ctrl+Alt+S 새 스티커).

## 현재 한계

- 잠금 메모의 비밀번호는 해시로만 저장하지만, 본문은 암호화하지 않습니다.
- 근무자 보기(엑셀 연동), Windows 알림 센터 토스트는 후속입니다.
- 공휴일은 고정 양력 공휴일만 표시합니다(음력·대체·임시 공휴일은 표시하지 않음).
- 이미지 64 MB/8192×8192, HTML 10 MB 반입 제한이 있습니다. 보안 락이 걸린 PDF/PPT/Excel을 읽거나 잠금을 우회하지 않습니다.
- 설치파일은 코드 서명되지 않았습니다.

## 개발 기록

작업별 기록은 `docs/WORKLOG-*.md`에 있습니다. 예전 계획과 화면 검증 문서([plan.md](plan.md), [plans/](plans), [docs/visual-qa-v7.md](docs/visual-qa-v7.md))는 당시 기록으로 보존합니다.
