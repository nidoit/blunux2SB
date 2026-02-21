# blunux2 — Arch 기반 커스텀 리눅스 배포판

수동 설정 없이 Arch Linux를 사용할 수 있도록 설계된 커스텀 리눅스 배포판입니다.
`config.toml` 하나로 모든 설치 옵션을 설정하고, 클릭 한 번으로 설치할 수 있습니다.

---

## 3대 차별화 기능

```
┌──────────────────────────────────────────────────────────────┐
│                 Blunux 차별화 3종 세트                         │
│                                                              │
│  1. 쉬운 한글화       kime/fcitx5 자동 설정                   │
│  2. App Installer    GUI 원클릭 설치 도구 (Tauri)             │
│  3. AI Agent         Claude/DeepSeek + WhatsApp              │
│                                                              │
│  핵심 가치: "초보자도 쉽게" + "Arch Linux의 파워"               │
└──────────────────────────────────────────────────────────────┘
```

---

## 기술 스택

| 구성 요소 | 언어 | 위치 | 역할 |
|-----------|------|------|------|
| `build.jl` | **Julia** | 개발 PC | ISO 빌드 오케스트레이션 |
| `blunux-wizard` | **Rust** | ISO 내부 | 하드웨어 감지 + 라이브 세션 |
| `blunux-toml2cal` | **Rust** | ISO 내부 | config.toml → Calamares YAML |
| `blunux-ai` | **Rust** | ISO 내부 (선택) | AI Agent CLI / 데몬 |
| `blunux-whatsapp-bridge` | **Node.js** | 설치 후 | WhatsApp ↔ AI Agent IPC 브릿지 |

> ISO 내부에는 Python, Julia 없이 Rust + C만 포함됩니다.
> Node.js와 AI 관련 의존성은 App Installer를 통해 설치 후에 추가됩니다.

---

## 프로젝트 구조

```
blunux2SB/
├── config.toml                        # 유일한 설정 파일 (전체 빌드 및 설치 옵션)
├── build.jl                           # Julia ISO 빌드 오케스트레이터
├── Cargo.toml                         # Rust 워크스페이스
│
├── crates/
│   ├── blunux-config/                 # config.toml 타입 정의 (워크스페이스 공유)
│   ├── toml2cal/                      # config.toml → Calamares YAML 변환기
│   ├── wizard/                        # 셋업 위자드 (하드웨어 감지 + 데스크톱 실행)
│   └── ai-agent/                      # AI Agent 코어 (Rust, v0.4.0)
│       └── src/
│           ├── main.rs                #   CLI: chat, setup, daemon, status
│           ├── agent.rs               #   Agent 루프 (AI → 도구 실행 → 응답)
│           ├── automations.rs         #   자동화 스케줄러 (cron + TOML 파서)
│           ├── config.rs              #   AgentConfig, WhatsAppConfig 로드/저장
│           ├── daemon.rs              #   Unix 소켓 데몬 + poll_notifications
│           ├── ipc.rs                 #   IPC 메시지 타입 (JSON 직렬화)
│           ├── memory.rs              #   마크다운 기반 로컬 메모리
│           ├── providers/             #   AI 프로바이더 트레이트 + 구현체
│           │   ├── claude_api.rs      #     Claude HTTP API (reqwest)
│           │   ├── claude_oauth.rs    #     Claude Code OAuth (subprocess)
│           │   └── deepseek.rs        #     DeepSeek HTTP API (reqwest)
│           ├── setup.rs               #   TUI 설정 마법사
│           ├── strings.rs             #   한국어/영어 이중 언어 문자열
│           └── tools/                 #   시스템 도구 레지스트리
│               ├── system.rs          #     pacman, systemctl, journalctl 등
│               ├── packages.rs        #     패키지 설치 (App Installer 연동)
│               └── safety.rs          #     권한 체크, 위험 명령 차단
│
├── blunux-whatsapp-bridge/            # WhatsApp 브릿지 (Node.js)
│   ├── package.json
│   └── src/
│       ├── index.js                   #   서비스 진입점
│       ├── bridge.js                  #   WhatsApp ↔ IPC 라우팅 + 자동화 알림 폴러
│       ├── ipc.js                     #   IpcClient (Unix 소켓, 재연결 로직)
│       └── config.js                  #   TOML 파서 (whatsapp/agent 섹션 읽기)
│
├── blunux-ai-installer/               # App Installer용 AI Agent 패키지
│   ├── install-ai-agent.sh            #   설치 스크립트 (App Installer 카드가 실행)
│   └── ai-agent.card.json             #   App Installer 카드 정의 (이중 언어)
│
├── profile/                           # archiso 프로파일 (build.jl이 동적 생성)
│   ├── packages.x86_64                #   ISO 포함 패키지 목록 (자동 생성)
│   └── airootfs/                      #   루트 파일시스템 오버레이 (자동 생성)
│
└── scripts/
    ├── startblunux                    # 라이브 세션 진입점
    └── calamares-blunux               # Calamares 실행 래퍼
```

---

## 빌드 방법

### 필수 도구

```bash
# Julia 1.9+ (ISO 빌드 오케스트레이션용)
sudo pacman -S julia

# Rust 1.75+ (Rust 바이너리 컴파일용)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# archiso (ISO 생성용)
sudo pacman -S archiso
```

### ISO 전체 빌드

```bash
# Julia TOML 패키지 설치 (최초 1회)
julia -e 'using Pkg; Pkg.add("TOML")'

# config.toml 편집 후 빌드 실행
julia build.jl
```

빌드 순서:

```
1. config.toml 로드
2. packages.x86_64 생성  → config에 맞는 패키지 목록
3. airootfs/ 생성        → hostname, locale, mkinitcpio 설정
4. cargo build --release  → 전체 Rust 워크스페이스 컴파일
   ├── blunux-wizard
   ├── blunux-toml2cal
   ├── blunux-setup
   └── blunux-ai  ← [packages.ai].agent = true 일 때만 포함
5. 바이너리 → airootfs/usr/bin/ 복사
6. AI Agent 에셋 복사 (packages.ai.agent = true 시)
   ├── install-ai-agent.sh → airootfs/usr/share/blunux/
   └── ai-agent.card.json  → airootfs/usr/share/blunux-installer/cards/
7. sudo mkarchiso → out/ 에 ISO 파일 생성
```

### 빌드 옵션

```bash
# 프로파일만 생성 (ISO 빌드 없이 — archiso 미설치 환경에서 유용)
julia build.jl --profile-only

# Rust 빌드 건너뛰기 (기존 바이너리 재사용)
julia build.jl --skip-rust
```

### 환경 변수

| 변수 | 기본값 | 설명 |
|------|--------|------|
| `BLUNUX_WORK` | `/tmp/blunux2-work` | mkarchiso 작업 디렉토리 |
| `BLUNUX_OUT` | `./out` | ISO 출력 디렉토리 |

---

## AI Agent 개발 빌드

### 빠른 시작

```bash
# 전체 워크스페이스 빌드
cargo build

# AI Agent만 빌드
cargo build --bin blunux-ai

# 릴리스 빌드 (ISO 수록용)
cargo build --release --bin blunux-ai
```

빌드 결과물: `target/release/blunux-ai`

### 테스트

```bash
# 단위 + 통합 테스트 전체 실행 (95 passed, 2 ignored)
cargo test --manifest-path crates/ai-agent/Cargo.toml

# 통합 테스트만 실행
cargo test --manifest-path crates/ai-agent/Cargo.toml --test integration_test

# Claude API 연동 테스트 (API 키 필요, 기본 무시됨)
ANTHROPIC_API_KEY=sk-ant-... \
  cargo test --manifest-path crates/ai-agent/Cargo.toml -- --ignored test_claude_api_provider

# DeepSeek API 연동 테스트
DEEPSEEK_API_KEY=sk-... \
  cargo test --manifest-path crates/ai-agent/Cargo.toml -- --ignored test_deepseek_provider
```

### AI Agent CLI 사용법

```bash
# 최초 설정 마법사 (프로바이더 선택, API 키, WhatsApp 설정)
./target/debug/blunux-ai setup

# 대화 모드 (터미널에서 직접 AI와 대화)
./target/debug/blunux-ai chat

# 상태 확인 (프로바이더, 메모리, WhatsApp, 자동화 목록)
./target/debug/blunux-ai status

# 데몬 모드 (WhatsApp 브릿지와 함께 사용, systemd 서비스로 관리)
./target/debug/blunux-ai daemon
```

### WhatsApp 브릿지 개발 실행

```bash
cd blunux-whatsapp-bridge
npm install
node src/index.js
```

처음 실행 시 터미널에 QR 코드가 출력됩니다. 휴대폰 WhatsApp으로 스캔하면 연결됩니다.

---

## config.toml 설정

`config.toml`은 ISO 빌드부터 설치, 런타임까지 모든 옵션을 제어하는 **단일 소스**입니다.

### 섹션 구조

| 섹션 | 설명 |
|------|------|
| `[blunux]` | 버전, 빌드 이름 |
| `[locale]` | 언어, 시간대, 키보드 레이아웃 |
| `[input_method]` | 한글 입력기 (kime, fcitx5, ibus) |
| `[kernel]` | 커널 선택 (linux, linux-lts, linux-zen) |
| `[install]` | 부트로더, 호스트명, 사용자, 비밀번호, 암호화, 자동 로그인 |
| `[disk]` | 스왑 공간 (none, small, suspend, file) |
| `[packages.*]` | 데스크톱, 브라우저, 오피스, 개발 도구, 멀티미디어, 게임, 가상화, 통신, 유틸리티 |
| `[packages.ai]` | **AI Agent를 ISO에 포함할지 여부** |
| `[ai_agent]` | AI Agent 런타임 설정 (프로바이더, 모드, 언어) |
| `[ai_agent.automations]` | 기본 자동화 온/오프 (헬스체크, 업데이트, 디스크) |
| `[whatsapp]` | WhatsApp 브릿지 보안 설정 |

### AI Agent 관련 설정 예시

```toml
# ISO 빌드에 포함 여부
[packages.ai]
agent = true

# 런타임 설정
[ai_agent]
enabled = true
provider = "claude"          # "claude" | "deepseek"
claude_mode = "oauth"        # "oauth" (Pro/Max 구독) | "api" (API 키 과금)
whatsapp_enabled = false
language = "auto"            # "auto" | "ko" | "en"
safe_mode = true

# 자동화 기본 활성화 여부 (세부 일정은 ~/.config/blunux-ai/automations.toml)
[ai_agent.automations]
health_check = true          # 매일 오전 9시 시스템 상태 보고
security_updates = true      # 6시간마다 보안 업데이트 확인
disk_warning = true          # 매일 자정 디스크 공간 경고

# WhatsApp 보안 설정
[whatsapp]
allowed_numbers = []         # 비어 있으면 모든 번호 허용
max_messages_per_minute = 5
require_prefix = false       # true면 "/ai " 접두사 필수 (그룹 채팅 보안용)
session_timeout = 3600       # 무활동 후 대화 초기화 (초)
```

### 자동화 스케줄 커스텀

데몬 최초 실행 시 `~/.config/blunux-ai/automations.toml`이 자동 생성됩니다.
이 파일을 직접 편집하여 일정과 동작을 변경할 수 있습니다.

```toml
# ~/.config/blunux-ai/automations.toml
# schedule 형식: 분 시 일 월 요일 (5-field cron)
#   *   = 모두 일치
#   N   = 정확한 값
#   */N = N마다

[[automation]]
name = "시스템 헬스체크"
schedule = "0 9 * * *"        # 매일 오전 9시
action = "시스템 전체 상태를 확인하고 CPU, RAM, 디스크, 업타임을 요약해줘"
notify = "whatsapp"
enabled = true

[[automation]]
name = "보안 업데이트 확인"
schedule = "0 */6 * * *"      # 6시간마다
action = "보안 업데이트가 있으면 목록과 함께 알려줘"
notify = "whatsapp"
enabled = true

[[automation]]
name = "디스크 공간 경고"
schedule = "0 0 * * *"        # 매일 자정
action = "디스크 사용률 80% 이상인 파티션이 있으면 경고해줘"
notify = "whatsapp"
enabled = true
```

---

## 시스템 아키텍처

### 라이브 세션 부팅

```
Bash (startblunux) → Rust (blunux-wizard) → exec startplasma-wayland
```

### 설치 과정

```
config.toml → blunux-toml2cal → Calamares YAML → Calamares 설치 파이프라인
```

### AI Agent 통신 구조

```
WhatsApp 서버
    │ WebSocket (whatsapp-web.js)
    ▼
blunux-whatsapp-bridge  (Node.js — systemd user service)
  • QR 인증 / 세션 관리
  • 메시지 수신 → 화이트리스트 확인 → IPC 전달
  • 15초마다 데몬에서 자동화 알림 폴링 → WhatsApp 발송
    │
    │  Unix Domain Socket  /run/user/$UID/blunux-ai.sock
    │  newline-delimited JSON  (IpcMessage)
    ▼
blunux-ai daemon  (Rust — systemd user service)
  • AI Provider 호출 (Claude API / OAuth / DeepSeek)
  • 시스템 도구 실행 (pacman, systemctl, journalctl 등)
  • 자동화 스케줄러 (cron, 매분 체크)
  • notify_queue 관리 → poll_notifications IPC 액션으로 브릿지에 전달
```

### ISO 포함 여부

| 구성 요소 | ISO 포함? | 설치 시점 |
|---|---|---|
| blunux-ai 바이너리 | ✅ 조건부 | build.jl에서 빌드 |
| install-ai-agent.sh | ✅ | App Installer 카드용 |
| ai-agent.card.json | ✅ | App Installer에 표시 |
| Node.js | ❌ | App Installer에서 설치 |
| Claude Code CLI | ❌ | App Installer에서 설치 |
| blunux-whatsapp-bridge | ❌ | App Installer에서 설치 |
| API 키 / WhatsApp 세션 | ❌ | 사용자가 직접 설정 |

---

## 설계 원칙

- **단일 설정 파일** — `config.toml`이 유일한 소스. Calamares YAML을 직접 편집할 필요 없음
- **ISO 런타임 최소화** — Rust + Bash만 포함. Julia, Python, Node.js는 ISO에 없음
- **Julia는 빌드 전용** — 개발 PC에서 오케스트레이션만 담당. ISO 크기 절감 + JIT 없음
- **API 키 분리** — `config.toml`에 API 키 없음 (ISO에 포함될 수 있으므로). 별도 `~/.config/blunux-ai/credentials/`에 저장
- **무거운 의존성은 post-install** — Node.js, npm 패키지는 App Installer 경유
- **안전장치 3단계** — 읽기(자동) / 패키지 설치(확인 필요) / 위험 명령(이중 확인 + 차단)

---

## 개발 로드맵

| 단계 | 버전 | 내용 | 상태 |
|------|------|------|------|
| Phase 1 | v0.1.0 | AI Agent 코어 MVP (CLI 대화, 시스템 도구, 메모리) | ✅ 완료 |
| Phase 2 | v0.2.0 | WhatsApp 브릿지 + 데몬 모드 + Unix IPC | ✅ 완료 |
| Phase 3 | v0.3.0 | App Installer 카드 + 설정 마법사 WhatsApp 단계 | ✅ 완료 |
| Phase 4 | v0.4.0 | 자동화 스케줄러 + WhatsApp 자동 알림 | ✅ 완료 |
| Phase 5 | v1.0.0 | AUR 패키징, ISO 통합 테스트, 보안 감사 | 예정 |

---

## 라이선스

MIT
