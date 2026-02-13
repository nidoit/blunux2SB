# blunux2 — Arch 기반 커스텀 리눅스 배포판

수동 설정 없이 Arch Linux를 사용할 수 있도록 설계된 커스텀 리눅스 배포판입니다. `config.toml` 하나로 모든 설치 옵션을 설정하고, 클릭 한 번으로 설치할 수 있습니다.

## 기술 스택

| 구성 요소 | 언어 | 위치 | 역할 |
|-----------|------|------|------|
| `build.jl` | **Julia** | 개발 PC | ISO 빌드 오케스트레이션 (config.toml 읽기 → archiso 프로파일 생성 → mkarchiso 실행) |
| `blunux-wizard` | **Rust** | ISO 내부 | 하드웨어 감지, config.toml 로딩, 라이브 세션 설정, 데스크톱 실행 |
| `blunux-toml2cal` | **Rust** | ISO 내부 | config.toml → Calamares YAML 변환 |
| 저수준 폴백 | **C/C++** | ISO 내부 | Rust 바인딩이 없는 커널 ioctl 등 |

> ISO 내부에는 Python, Julia 없이 Rust + C만 포함됩니다. Julia는 개발자 PC에서 빌드용으로만 사용합니다.

## 프로젝트 구조

```
blunux2SB/
├── config.toml                        # 사용자 설정 파일 (설치 옵션)
├── build.jl                           # Julia ISO 빌드 오케스트레이터 (개발자 PC용)
├── prd.md                             # 제품 요구 사항 문서
├── Cargo.toml                         # Rust 워크스페이스
│
├── profile/                           # archiso 프로파일 (build.jl이 동적 생성)
│   ├── profiledef.sh                  #   ISO 마스터 설정 (이름, 부트모드, squashfs)
│   ├── pacman.conf                    #   패키지 관리자 설정
│   ├── packages.x86_64                #   ISO에 포함될 패키지 목록 (자동 생성)
│   ├── grub/grub.cfg                  #   GRUB 부트 메뉴
│   ├── syslinux/syslinux.cfg          #   BIOS 부트 설정
│   ├── efiboot/loader/                #   UEFI systemd-boot 설정
│   └── airootfs/                      #   루트 파일시스템 오버레이 (자동 생성)
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
│   └── wizard/                        # 셋업 위자드 (Rust 바이너리)
│       └── src/
│           ├── main.rs                #   하드웨어 감지 → config 로딩 → 데스크톱 실행
│           └── hwdetect.rs            #   GPU, 오디오, UEFI, RAM 감지
│
└── scripts/
    ├── startblunux                    # 라이브 세션 진입점 → Rust wizard → Plasma
    └── calamares-blunux               # toml2cal 실행 → Calamares 실행
```

## 작동 방식

### 라이브 세션 부팅

```
Bash (startblunux) → Rust (blunux-wizard) → exec startplasma-wayland
```

`blunux-wizard`가 직접 하드웨어 감지, config.toml 로딩, 로케일/키보드 적용을 수행한 뒤 Plasma 세션을 실행합니다. 중간 레이어 없이 단일 바이너리로 처리합니다.

### 설치 과정

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

- **Julia** (1.9+) — ISO 빌드 오케스트레이션
- **Rust** (1.75+) — 바이너리 컴파일
- **GCC** — C 폴백 컴포넌트
- **archiso** — Arch Linux ISO 생성 (`sudo pacman -S archiso`)

### ISO 빌드 (전체)

```bash
# Julia 패키지 설치 (최초 1회)
julia -e 'using Pkg; Pkg.add("TOML")'

# ISO 빌드 (config.toml 읽기 → Rust 빌드 → archiso 프로파일 생성 → ISO 생성)
julia build.jl
```

빌드 과정:
1. `config.toml`을 읽어 설정 분석
2. `packages.x86_64` 생성 (config에 맞는 패키지 목록)
3. `airootfs/` 오버레이 생성 (hostname, locale, mkinitcpio)
4. `cargo build --release` 실행 (Rust 바이너리 컴파일)
5. 바이너리 + 스크립트를 `airootfs/usr/bin/`에 복사
6. `sudo mkarchiso` 실행 → ISO 파일 출력

### 빌드 옵션

```bash
# 프로파일만 생성 (ISO 빌드 없이)
julia build.jl --profile-only

# Rust 빌드 건너뛰기 (기존 바이너리 사용)
julia build.jl --skip-rust
```

### 환경 변수

| 변수 | 기본값 | 설명 |
|------|--------|------|
| `BLUNUX_WORK` | `/tmp/blunux2-work` | mkarchiso 작업 디렉토리 |
| `BLUNUX_OUT` | `./out` | ISO 출력 디렉토리 |

### Rust만 빌드 (개발용)

```bash
cargo check          # 컴파일 확인
cargo build --release  # 릴리스 빌드
cargo test           # 테스트 실행
```

빌드 결과물:
- `target/release/blunux-wizard` — 셋업 위자드 (하드웨어 감지 + 데스크톱 실행)
- `target/release/blunux-toml2cal` — TOML→Calamares 변환 CLI

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
- **Julia** — ISO 내부가 아닌 개발자 PC에서 빌드 오케스트레이션용으로만 사용합니다. ISO 런타임에는 Julia가 포함되지 않아 ISO 크기가 줄어들고 JIT 시작 지연이 없습니다.

## 라이선스

MIT
