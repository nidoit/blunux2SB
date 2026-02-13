# blunux2 — Arch 기반 커스텀 리눅스 배포판

수동 설정 없이 Arch Linux를 사용할 수 있도록 설계된 커스텀 리눅스 배포판입니다. `config.toml` 하나로 모든 설치 옵션을 설정하고, 클릭 한 번으로 설치할 수 있습니다.

## 기술 스택

| 계층 | 언어 | 역할 |
|------|------|------|
| 오케스트레이터 | **Julia** | 위자드 단계 조율, `ccall`로 Rust 호출 |
| 코어 라이브러리 | **Rust** | 하드웨어 감지, GTK4 UI, TOML 파싱, Calamares YAML 생성 |
| 저수준 폴백 | **C/C++** | Rust 바인딩이 없는 커널 ioctl 등 |

> Python은 전체 스택에서 사용하지 않습니다.

## 프로젝트 구조

```
blunux2SB/
├── config.toml                        # 사용자 설정 파일 (설치 옵션)
├── prd.md                             # 제품 요구 사항 문서
├── Cargo.toml                         # Rust 워크스페이스
│
├── crates/
│   ├── blunux-config/                 # config.toml 타입 정의 (공유)
│   │   └── src/lib.rs                 #   BlunuxConfig 구조체, 로드/저장
│   │
│   ├── toml2cal/                      # config.toml → Calamares YAML 변환기
│   │   └── src/
│   │       ├── main.rs                #   CLI: generate, apply-packages, apply-input-method
│   │       ├── generate.rs            #   Calamares YAML 생성기 9종
│   │       └── packages.rs            #   패키지 불리언 → pacman 패키지명 매핑
│   │
│   └── libblunux/                     # 공유 라이브러리 (cdylib) — Julia FFI용
│       └── src/
│           ├── lib.rs
│           ├── hwdetect.rs            #   GPU, 오디오, UEFI, RAM 감지
│           ├── config.rs              #   설정 도우미 (로드, 저장, setter)
│           └── ffi.rs                 #   Julia ccall용 extern "C" 함수 14개
│
├── src/livecd/
│   └── main.jl                        # Julia 오케스트레이터
│
└── scripts/
    ├── startblunux                    # 라이브 세션 진입점 → Julia → Plasma
    └── calamares-blunux               # toml2cal 실행 → Calamares 실행
```

## 작동 방식

```
┌─────────────┐     ┌──────────────────┐     ┌───────────────────┐
│ config.toml │────▶│ blunux-toml2cal   │────▶│ Calamares YAML    │
│ (사용자 편집)│     │ (Rust 변환기)     │     │ settings.conf     │
│             │     │                   │     │ partition.conf    │
│             │     │                   │     │ locale.conf       │
│             │     │                   │     │ users.conf        │
│             │     │                   │     │ bootloader.conf   │
└─────────────┘     └──────────────────┘     └───────────────────┘
                                                      │
                                                      ▼
                                              ┌───────────────────┐
                                              │ Calamares 설치     │
                                              │ 파이프라인 실행     │
                                              └───────────────────┘
```

사용자는 Calamares YAML을 직접 편집할 필요가 없습니다. `config.toml`이 유일한 설정 파일이며, Rust 변환기가 나머지를 처리합니다.

## 빌드 방법

### 필수 도구

- Rust (1.75+)
- Julia
- GCC (C 폴백 컴포넌트용)

### Rust 워크스페이스 빌드

```bash
# 컴파일 확인
cargo check

# 릴리스 빌드
cargo build --release

# 테스트 실행
cargo test
```

빌드 결과물:
- `target/release/blunux-toml2cal` — TOML→Calamares 변환 CLI
- `target/release/libblunux.so` — Julia FFI용 공유 라이브러리

### toml2cal 사용법

```bash
# config.toml에서 Calamares YAML 전체 생성
blunux-toml2cal generate \
    --input config.toml \
    --output-dir /etc/calamares/modules \
    --settings /etc/calamares/settings.conf

# config.toml [packages.*]에 지정된 패키지 설치
blunux-toml2cal apply-packages --input config.toml

# 한글 입력기 설정 적용
blunux-toml2cal apply-input-method --input config.toml
```

## config.toml 설정

`config.toml`은 설치의 모든 옵션을 제어합니다:

| 섹션 | 설명 |
|------|------|
| `[blunux]` | 버전, 빌드 이름 |
| `[locale]` | 언어, 시간대, 키보드 레이아웃 |
| `[input_method]` | 한글 입력기 (kime, fcitx5, ibus) |
| `[kernel]` | 커널 선택 (linux, linux-lts, linux-zen) |
| `[install]` | 부트로더, 호스트명, 사용자, 비밀번호, 암호화, 자동 로그인 |
| `[disk]` | 스왑 공간 (none, small, suspend, file) |
| `[packages.*]` | 데스크톱, 브라우저, 오피스, 개발 도구, 멀티미디어, 게임, 가상화, 통신, 유틸리티 |

### 설정 예시

```toml
[locale]
language = ["ko_KR"]
timezone = "Europe/Stockholm"
keyboard = ["kr", "us"]

[install]
bootloader = "nmbl"          # EFISTUB 직접 부팅 (가장 빠름)
hostname = "nux"
username = "blu"

[disk]
swap = "small"               # RAM 절반 크기의 스왑

[packages.desktop]
kde = true

[packages.browser]
firefox = true
```

## 설계 결정

- **테마** — blunux2 기본 테마 하나만 제공합니다. 선택 화면이 없습니다.
- **드라이버** — Rust 하드웨어 감지가 자동 선택합니다 (NVIDIA→독점 드라이버, AMD/Intel→mesa). 사용자 선택이 필요 없습니다.
- **파일시스템** — btrfs 서브볼륨이 기본이며 유일한 옵션입니다.
- **부트로더** — GRUB, systemd-boot, nmbl(EFISTUB) 중 선택 가능합니다.

## 라이선스

MIT
