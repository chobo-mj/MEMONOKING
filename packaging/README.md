# OmniMemoJang Windows UI Preview

Windows 10/11 x64에서 화면과 조작을 시험하기 위한 사전 배포입니다. **프로그램을 종료하면 메모와 설정이 초기화됩니다. 실제 업무 자료를 보관하는 용도로 사용하지 마세요.**

잠금은 비밀번호를 입력해 화면을 가리는 UI 시뮬레이션입니다. 실제 암호화·보안 통제를 제공하지 않습니다. 영구 저장·백업/복원·Windows 알림센터·트레이 상주·자동실행·기한별 자동삭제는 후속입니다. 설정에 보이는 해당 옵션은 선택과 미리보기만 제공합니다. 보안 락이 걸린 PDF/PPT/Excel을 읽거나 잠금을 우회하지 않습니다.

## 설치하거나 바로 실행하기

- 설치형: `OmniMemoJang-0.2.0-ui-preview.2-Windows-x64-Setup.exe`를 실행합니다. 기본 설치 위치는 현재 사용자의 `%LOCALAPPDATA%\Programs\OmniMemoJang`이며 관리자 권한을 요청하지 않습니다. 시작 메뉴 바로가기를 만들고, 바탕화면 바로가기는 사용자가 선택할 수 있습니다.
- 포터블: `OmniMemoJang-0.2.0-ui-preview.2-Windows-x64-Portable.zip`을 쓰기 가능한 폴더에 풀고 `OmniMemoJang\OmniMemoJang.exe`를 실행합니다.
- 실행 PC에는 Rust·C++ 빌드 도구·Inno Setup이 필요하지 않습니다. 실행 중 인터넷이나 Visual C++ 런타임 다운로드를 요구하지 않습니다. 운영체제의 그래픽 드라이버와 Windows 글꼴을 사용합니다.
- 제거: Windows의 설치된 앱에서 `OmniMemoJang UI Preview`를 제거하거나 설치 폴더의 `unins000.exe`를 실행합니다. 포터블은 프로그램을 닫고 압축을 푼 폴더를 정리합니다. 사용자가 다른 위치에 내보낸 이미지 등은 제거 대상이 아닙니다.
- 설치파일은 코드 서명되지 않았습니다. 사내 반입·실행 정책에 따라 시험하며, 차단된 정책을 변경하거나 우회하지 않습니다.

다음 배포는 파일명에 해당 버전이 들어갑니다. 설치/포터블 패키지에는 실행파일·이 안내문·체크섬만 포함하며 원본 PPT, 화면 검토용 첨부, 실제 사용자 파일·메모를 포함하지 않습니다. 프로그램에 내장된 데모 메모는 화면 시험용 예시입니다.

## 파일 확인

GitHub Release의 `SHA256SUMS.txt`에는 설치파일과 포터블 ZIP의 SHA-256이 기록되어 있습니다. PowerShell에서 다음처럼 계산해 같은 파일명의 값과 대조합니다.

```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath '.\OmniMemoJang-0.2.0-ui-preview.2-Windows-x64-Setup.exe'
Get-FileHash -Algorithm SHA256 -LiteralPath '.\OmniMemoJang-0.2.0-ui-preview.2-Windows-x64-Portable.zip'
```

압축을 푼 `OmniMemoJang` 폴더의 `SHA256SUMS.txt`는 내부 `OmniMemoJang.exe`와 `README.txt`를 대조할 때 사용합니다. 해시는 전달된 파일이 같은지 확인하는 값이며 코드 서명이나 사내 보안 승인을 대체하지 않습니다.

## 개발자용 패키징

배포 구성은 `packaging/OmniMemoJang.iss`와 `.github/workflows/release.yml`입니다. GitHub Actions의 Windows 환경에서 다음 순서로 처리합니다.

1. 이미 존재하는 `vMAJOR.MINOR.PATCH-ui-preview` 또는 `vMAJOR.MINOR.PATCH-ui-preview.N` 태그와 Cargo 버전을 확인합니다. 예정 태그는 `v0.2.0-ui-preview.2`, Cargo 버전은 `0.2.0`입니다.
2. 형식 검사·Clippy·테스트를 통과한 뒤 `x86_64-pc-windows-msvc`로 빌드합니다. `RUSTFLAGS=-C target-feature=+crt-static`으로 C 런타임을 정적 연결하고 `dumpbin /DEPENDENTS`로 별도 Visual C++ 런타임 DLL이 없는지 확인합니다.
3. `dist/portable/OmniMemoJang`에 `OmniMemoJang.exe`, 이 문서의 사용 안내 부분을 담은 `README.txt`, 내부 `SHA256SUMS.txt`만 명시적으로 배치합니다. 소스 트리 전체나 로컬 작업 폴더를 복사하지 않습니다.
4. Windows 환경에 있는 Inno Setup의 `ISCC.exe`를 먼저 사용합니다. 실행파일의 버전 리소스에 의존하지 않고 출력 파일을 만들지 않는 짧은 컴파일로 실제 엔진 버전을 확인합니다. 없으면 GitHub 빌드 환경에만 Chocolatey로 Inno Setup을 설치합니다. 한국어 UTF-8 스크립트와 `x64compatible` 지시문을 지원하는 Inno Setup 6.3 이상이 필요합니다.
5. 현재 사용자 영역에 무인 설치하고 실행파일 해시·시작 메뉴 바로가기·선택하지 않은 바탕화면 바로가기 부재를 확인한 뒤 제거합니다. 앱 자체를 자동 실행하거나 Windows 시작/네트워크/보안 설정을 변경하지 않습니다.
6. 결과 체크섬을 만들고 세 파일을 GitHub Actions 산출물로 남깁니다. 이후 기본 `GITHUB_TOKEN`의 `contents: write` 권한으로 해당 태그의 **사전 배포**에 게시합니다. 별도 개인 토큰이 필요하지 않습니다.

결과물은 다음과 같습니다.

```text
dist/
  OmniMemoJang-0.2.0-ui-preview.2-Windows-x64-Setup.exe
  OmniMemoJang-0.2.0-ui-preview.2-Windows-x64-Portable.zip
  SHA256SUMS.txt
```

태그를 push하면 릴리즈 작업이 시작됩니다. 작업 수동 실행은 이미 존재하는 같은 태그에 한해서 사용할 수 있습니다. 같은 사전 배포를 다시 실행하면 동일 이름의 첨부파일을 교체하며 정식 배포에는 덮어쓰지 않습니다. 빌드·테스트·런타임 의존성·설치/제거 검사가 실패하면 게시하지 않습니다. 이 안내문이 있다는 사실만으로 설치파일 빌드·배포·사내 PC 실행 검증이 완료되었다는 뜻은 아닙니다.

로컬에서 패키지를 만들려면 Rust stable, MSVC x64 빌드 도구, Inno Setup 6.3 이상을 준비하고 같은 스테이징 디렉터리를 만든 다음 아래처럼 컴파일합니다. `.iss`의 `StageDir`에는 실행파일·안내문·내부 체크섬 3개가 있어야 합니다.

```powershell
$stage = Join-Path $PWD 'dist\portable\OmniMemoJang'
$output = Join-Path $PWD 'dist'
& 'C:\Program Files (x86)\Inno Setup 6\ISCC.exe' `
  '/DAppVersion=0.2.0' '/DReleaseLabel=0.2.0-ui-preview.2' `
  "/DStageDir=$stage" "/DPackageOutputDir=$output" 'packaging\OmniMemoJang.iss'
```

공식 기준: [Inno 사용자 권한](https://jrsoftware.org/ishelp/topic_setup_privilegesrequired.htm), [Inno 아키텍처](https://jrsoftware.org/ishelp/topic_setup_architecturesallowed.htm), [Inno Unicode](https://jrsoftware.org/ishelp/topic_unicode.htm), [Rust 정적 C 런타임](https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes), [GitHub 기본 토큰](https://docs.github.com/en/actions/tutorials/authenticate-with-github_token), [GitHub 사전 배포 생성](https://cli.github.com/manual/gh_release_create).
