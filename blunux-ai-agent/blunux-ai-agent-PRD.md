# PRD: Blunux AI Agent

**프로젝트명:** Blunux AI Agent  
**버전:** 1.0.0
**작성일:** 2026-02-21
**작성자:** Jaewoo Joung (정재우)  
**상위 프로젝트:** [blunux2SB](https://github.com/nidoit/blunux2SB)  
**라이선스:** MIT

---

## 1. 개요

### 1.1 프로젝트 요약

Blunux AI Agent는 Blunux 리눅스 배포판의 **3대 차별화 기능** 중 하나로, AI를 통한 시스템 관리와 24/7 자동화를 제공한다. 사용자는 **WhatsApp 메시지**로 자신의 리눅스 시스템을 관리할 수 있다.

### 1.2 Blunux 3대 차별화

```
┌──────────────────────────────────────────────────────────────┐
│                 Blunux 차별화 3종 세트                         │
│                                                              │
│  1. 쉬운 한글화       kime/fcitx5 자동 설정 (구현 완료)         │
│  2. App Installer    GUI 원클릭 설치 도구 (Tauri, 구현 중)     │
│  3. AI Agent         Claude/DeepSeek + WhatsApp (본 문서)     │
│                                                              │
│  핵심 가치: "초보자도 쉽게" + "Arch Linux의 파워"               │
└──────────────────────────────────────────────────────────────┘
```

### 1.3 왜 Blunux AI Agent인가?

현재 리눅스 배포판 중 AI 에이전트를 네이티브 경험으로 제공하는 배포판은 없다. OpenClaw 같은 도구가 있지만 설치와 설정이 복잡하다. Blunux AI Agent는 **App Installer에서 카드 한 번 클릭**으로 설치되고, **WhatsApp QR 코드 한 번 스캔**으로 바로 사용할 수 있다.

**차별화 포인트:**

- **App Installer 통합** — 카드 한 번 클릭으로 전체 설치 완료
- **WhatsApp이 인터페이스** — 새로운 앱을 배울 필요 없음
- **Claude Code OAuth 지원** — 별도 API 키 없이 Claude Pro/Max 구독만으로 사용
- **DeepSeek 대안 제공** — 사용자가 원하는 AI 모델 선택
- **한국어 네이티브** — 한국 리눅스 사용자를 위한 최적화
- **blunux2SB 아키텍처 준수** — Rust 바이너리 + config.toml 단일 소스

---

## 2. 타겟 사용자

| 사용자 유형 | 주요 니즈 | 사용 방식 |
|---|---|---|
| 리눅스 초보자 | 터미널 명령어가 어려움 | WhatsApp: "크롬 설치해줘" → 자동 실행 |
| 개발자/파워유저 | 반복 작업 자동화 | WhatsApp: "서버 상태 보고해" → 자동 리포트 |
| 해외 거주 한국인 | 한국어 지원 리눅스 | 한국어로 AI에게 시스템 관리 요청 |

---

## 3. 아키텍처

### 3.1 blunux2SB 내 위치

```
blunux2SB/
├── config.toml                    # [ai_agent] 섹션 추가
├── build.jl                       # ai_agent.enabled → 조건부 빌드
├── Cargo.toml                     # members에 ai-agent 추가
├── prd.md                         # blunux2 전체 PRD
│
├── crates/
│   ├── blunux-config/             # 기존: AiAgent 필드 추가
│   ├── wizard/                    # 기존: 하드웨어 위저드
│   ├── toml2cal/                  # 기존: Calamares 변환
│   ├── setup/                     # 기존: AUR 설치
│   └── ai-agent/                  # 신규: AI Agent Core (Rust)
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs            # CLI: chat, setup, daemon, status
│           ├── providers/
│           │   ├── mod.rs         # Provider trait
│           │   ├── claude_api.rs  # Claude HTTP API (reqwest)
│           │   └── deepseek.rs    # DeepSeek HTTP API (reqwest)
│           ├── tools/
│           │   ├── mod.rs         # Tool trait
│           │   ├── system.rs      # pacman, systemctl, journalctl 등
│           │   ├── packages.rs    # 패키지 설치 (App Installer 연동)
│           │   └── safety.rs      # 권한 체크, 위험 명령 차단
│           ├── memory.rs          # 마크다운 기반 로컬 메모리
│           └── daemon.rs          # systemd 서비스 모드
│
├── blunux-whatsapp-bridge/        # 신규: WhatsApp 브릿지 (Node.js)
│   ├── package.json
│   ├── src/
│   │   ├── index.js               # whatsapp-web.js 기반 메시지 브릿지
│   │   ├── agent-client.js        # blunux-ai-agent와 IPC 통신
│   │   └── qr-setup.js            # QR 코드 스캔 설정 화면
│   └── blunux-wa-bridge.service   # systemd user service
│
├── blunux-ai-installer/           # 신규: App Installer용 설치 스크립트
│   └── install-ai-agent.sh        # App Installer 카드가 실행하는 스크립트
│
├── profile/                       # 기존 archiso 프로파일
└── scripts/                       # 기존 스크립트
```

### 3.2 전체 시스템 구조

```
┌──────────────────────────────────────────────────────────────┐
│                    Blunux Desktop (KDE)                       │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                App Installer (Tauri)                    │  │
│  │  ┌─────────┐ ┌─────────┐ ┌──────────────────────────┐ │  │
│  │  │ Chrome  │ │ VLC     │ │ 🤖 Blunux AI Agent      │ │  │
│  │  │ [설치]  │ │ [설치]  │ │ Claude + DeepSeek       │ │  │
│  │  └─────────┘ └─────────┘ │ WhatsApp으로 대화 가능   │ │  │
│  │                          │ [설치]                   │ │  │
│  │                          └──────────────────────────┘ │  │
│  └────────────────────────────────────────────────────────┘  │
│                              │                               │
│              설치 스크립트 실행 (install-ai-agent.sh)           │
│                              │                               │
│                              ▼                               │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              Blunux AI Agent Stack                      │  │
│  │                                                        │  │
│  │  ┌──────────────────┐    ┌──────────────────────────┐  │  │
│  │  │ blunux-ai-agent  │◄──│ blunux-whatsapp-bridge   │  │  │
│  │  │ (Rust 바이너리)   │    │ (Node.js)                │  │  │
│  │  │                  │    │                          │  │  │
│  │  │ • CLI 대화 모드   │    │ • whatsapp-web.js       │  │  │
│  │  │ • 시스템 도구 실행 │    │ • QR 코드 인증           │  │  │
│  │  │ • 메모리 관리     │    │ • 메시지 수신/발신        │  │  │
│  │  │ • 안전장치        │    │ • Unix socket IPC       │  │  │
│  │  └────────┬─────────┘    └──────────────────────────┘  │  │
│  │           │                                            │  │
│  │           ▼                                            │  │
│  │  ┌──────────────────────────────────────────────────┐  │  │
│  │  │              Provider Layer                       │  │  │
│  │  │  ┌──────────────────┐  ┌──────────────────────┐  │  │  │
│  │  │  │ Claude Provider  │  │ DeepSeek Provider    │  │  │  │
│  │  │  │                  │  │                      │  │  │  │
│  │  │  │ Mode A: API 직접  │  │ HTTP API 직접 호출    │  │  │  │
│  │  │  │  (reqwest HTTP)  │  │ (reqwest HTTP)       │  │  │  │
│  │  │  │                  │  │                      │  │  │  │
│  │  │  │ Mode B: Claude   │  │ deepseek-chat        │  │  │  │
│  │  │  │  Code OAuth      │  │ deepseek-coder       │  │  │  │
│  │  │  │  (subprocess)    │  │                      │  │  │  │
│  │  │  └──────────────────┘  └──────────────────────┘  │  │  │
│  │  └──────────────────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              System Tools Layer                         │  │
│  │  pacman / yay / systemctl / journalctl / nmcli / ...   │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

### 3.3 WhatsApp 브릿지 통신 구조

```
WhatsApp 서버 (E2E 암호화)
        │
        │ WebSocket (whatsapp-web.js)
        ▼
┌─────────────────────────────┐
│  blunux-whatsapp-bridge     │  Node.js 프로세스
│  (systemd user service)     │
│                             │
│  • 메시지 수신 → 파싱        │
│  • AI 응답 → WhatsApp 전송   │
│  • QR 세션 관리              │
│  • 연결 상태 모니터링         │
└─────────────┬───────────────┘
              │ Unix Domain Socket
              │ /run/user/$UID/blunux-ai.sock
              ▼
┌─────────────────────────────┐
│  blunux-ai-agent            │  Rust 프로세스
│  (systemd user service)     │
│                             │
│  • 메시지 → AI Provider      │
│  • AI 응답 → 도구 실행       │
│  • 결과 → 브릿지로 반환       │
│  • 메모리 업데이트            │
└─────────────────────────────┘
```

**IPC 프로토콜 (Unix Domain Socket, JSON):**

```json
// 브릿지 → 에이전트 (사용자 메시지)
{
  "type": "message",
  "from": "821012345678",
  "body": "시스템 업데이트 있어?",
  "timestamp": "2026-02-20T14:30:00Z"
}

// 에이전트 → 브릿지 (AI 응답)
{
  "type": "response",
  "to": "821012345678",
  "body": "보안 업데이트 3건이 있습니다:\n- linux 6.13.4\n- openssl 3.4.1\n- webkit2gtk 2.46.5\n\n지금 설치할까요?",
  "actions": ["yes_install", "skip"]
}

// 브릿지 → 에이전트 (사용자 확인)
{
  "type": "action",
  "from": "821012345678",
  "action": "yes_install"
}
```

---

## 4. 설치 흐름

### 4.1 사용자 경험 (초보자 관점)

```
1. Blunux 설치 완료, KDE 데스크탑 부팅
         │
2. 앱 런처에서 "Blunux Installer" 실행
         │
3. "🤖 AI Agent" 카드 발견
   ┌──────────────────────────────────────┐
   │  🤖 Blunux AI Agent                 │
   │                                     │
   │  Claude / DeepSeek AI로             │
   │  시스템을 관리하세요.                 │
   │  WhatsApp으로 대화할 수 있습니다.     │
   │                                     │
   │  포함: Claude Code, WhatsApp 브릿지   │
   │                                     │
   │           [ 설치 ]                   │
   └──────────────────────────────────────┘
         │
4. [설치] 클릭 → sudo 비밀번호 (App Installer가 이미 받아둠)
         │
5. 로그 패널에서 진행 확인 (약 3-5분)
   ✓ Node.js 설치 중...
   ✓ Claude Code 설치 중...
   ✓ AI Agent 설치 중...
   ✓ WhatsApp 브릿지 설치 중...
   ✓ 서비스 등록 중...
         │
6. 설정 마법사 팝업 (blunux-ai setup)
   ┌──────────────────────────────────────┐
   │  Blunux AI Agent 설정                │
   │                                     │
   │  AI 모델을 선택하세요:               │
   │  ● Claude (Pro/Max 구독 필요)        │
   │  ○ DeepSeek (API 키 필요)           │
   │                                     │
   │  Claude 연결 방식:                   │
   │  ● OAuth (Claude Pro 구독 사용)      │
   │  ○ API 키 직접 입력                  │
   │                                     │
   │              [ 다음 ]                │
   └──────────────────────────────────────┘
         │
7. Claude OAuth 인증 → 브라우저에서 로그인
         │
8. WhatsApp 연결
   ┌──────────────────────────────────────┐
   │  WhatsApp 연결                       │
   │                                     │
   │  휴대폰 WhatsApp에서                 │
   │  이 QR 코드를 스캔하세요:             │
   │                                     │
   │       ┌─────────────┐               │
   │       │ █▀▀▀█ ▀█▀▀█ │               │
   │       │ █ ▀ █ ██▀▀▄ │               │
   │       │ ▀▀▀▀▀ █▄█▄█ │               │
   │       └─────────────┘               │
   │                                     │
   │              [ 완료 ]                │
   └──────────────────────────────────────┘
         │
9. ✅ 설정 완료!
   WhatsApp에서 테스트 메시지를 보내보세요:
   "안녕? 내 시스템 상태 알려줘"
```

### 4.2 설치 스크립트 (install-ai-agent.sh)

App Installer의 AI Agent 카드가 실행하는 스크립트:

```bash
#!/bin/bash
set -e

echo "=== Blunux AI Agent 설치 ==="

# 1. 시스템 패키지 설치
echo "[1/6] 시스템 패키지 설치 중..."
sudo pacman -S --noconfirm --needed nodejs npm

# 2. Claude Code 설치 (OAuth 지원)
echo "[2/6] Claude Code 설치 중..."
npm install -g @anthropic-ai/claude-code

# 3. blunux-ai-agent 바이너리 설치 (AUR 또는 커스텀 repo)
echo "[3/6] AI Agent 설치 중..."
yay -S --noconfirm blunux-ai-agent

# 4. WhatsApp 브릿지 설치
echo "[4/6] WhatsApp 브릿지 설치 중..."
npm install -g blunux-whatsapp-bridge

# 5. systemd user 서비스 등록
echo "[5/6] 서비스 등록 중..."
mkdir -p ~/.config/systemd/user

cat > ~/.config/systemd/user/blunux-ai-agent.service << 'EOF'
[Unit]
Description=Blunux AI Agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/bin/blunux-ai-agent daemon
Restart=on-failure
RestartSec=10
Environment=BLUNUX_AI_HOME=%h/.config/blunux-ai

[Install]
WantedBy=default.target
EOF

cat > ~/.config/systemd/user/blunux-wa-bridge.service << 'EOF'
[Unit]
Description=Blunux WhatsApp Bridge
After=blunux-ai-agent.service
Requires=blunux-ai-agent.service

[Service]
Type=simple
ExecStart=/usr/bin/blunux-wa-bridge
Restart=on-failure
RestartSec=10
Environment=BLUNUX_AI_SOCK=%t/blunux-ai.sock

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
systemctl --user enable blunux-ai-agent.service
systemctl --user enable blunux-wa-bridge.service

# 6. 초기 설정 마법사 실행
echo "[6/6] 설정 마법사 실행 중..."
blunux-ai setup

echo ""
echo "✅ Blunux AI Agent 설치 완료!"
echo ""
echo "서비스 시작: systemctl --user start blunux-ai-agent blunux-wa-bridge"
echo "상태 확인:   blunux-ai status"
echo "WhatsApp에서 테스트 메시지를 보내보세요!"
```

---

## 5. 핵심 기능

### 5.1 시스템 관리 도우미

자연어로 시스템 관리 작업을 요청하면, AI Agent가 적절한 명령을 생성하고 실행한다.

**WhatsApp 대화 예시:**

```
사용자: 크롬 설치해줘
Agent:  google-chrome을 설치합니다.
        실행할 명령: yay -S google-chrome
        진행할까요? (네/아니오)
사용자: 네
Agent:  ✅ google-chrome 설치 완료!

사용자: 시스템 업데이트
Agent:  업데이트를 확인합니다...
        📦 업데이트 가능한 패키지 15개:
        - linux 6.13.3 → 6.13.4 (보안)
        - firefox 135.0 → 135.1
        - ... (13개 더)
        전체 업데이트할까요? (네/아니오)

사용자: 디스크 사용량
Agent:  💾 디스크 사용량:
        / (ext4): 45GB / 256GB (17%) ✅
        /home:    32GB / 256GB (12%) ✅
        swap:     0.2GB / 8GB
        상태 양호합니다.

사용자: 오늘 에러 로그
Agent:  📋 오늘 에러 로그 3건:
        1. bluetooth: adapter not found (09:15)
        2. docker: daemon not running (10:30)
        3. kime: engine restart (14:22)
        
        bluetooth 어댑터가 감지되지 않습니다.
        USB 블루투스 동글을 확인해 주세요.
```

**지원 작업:**

| 분류 | 자연어 예시 | 실제 실행 |
|---|---|---|
| 패키지 관리 | "크롬 설치해줘" | `yay -S google-chrome` |
| 시스템 업데이트 | "업데이트 해줘" | `sudo pacman -Syu` |
| 서비스 관리 | "SSH 서버 켜줘" | `sudo systemctl enable --now sshd` |
| 네트워크 | "와이파이 목록" | `nmcli device wifi list` |
| 디스크 | "용량 확인" | `df -h` + AI 분석 |
| 로그 분석 | "에러 로그 보여줘" | `journalctl --since today -p err` |
| 프로세스 | "메모리 많이 쓰는거" | `ps aux --sort=-%mem | head` |
| 한글 설정 | "한글 입력이 안 돼" | kime/fcitx5 상태 확인 + 재설정 |

### 5.2 자동화 에이전트 (24/7)

데몬 모드로 실행되며, 설정된 작업을 자동으로 수행하고 WhatsApp으로 알림을 보낸다.

```toml
# ~/.config/blunux-ai/automations.toml

[[automation]]
name = "시스템 헬스체크"
schedule = "0 9 * * *"            # 매일 오전 9시
action = "시스템 상태 확인하고 문제 있으면 알려줘"
notify = "whatsapp"

[[automation]]
name = "보안 업데이트 확인"
schedule = "0 */6 * * *"          # 6시간마다
action = "보안 업데이트 확인"
notify = "whatsapp"
auto_apply = false

[[automation]]
name = "디스크 공간 경고"
schedule = "0 0 * * *"            # 매일 자정
action = "디스크 80% 이상이면 알려줘"
notify = "whatsapp"
```

**WhatsApp 자동 알림 예시:**

```
🤖 Blunux AI Agent — 자동 보고

📊 일일 시스템 상태 (09:00)
• CPU: 12% | RAM: 4.2GB/16GB (26%)
• 디스크: 48GB/256GB (18%)
• 업타임: 5일 2시간
• 보류 업데이트: 3건 (보안 1건)

⚠️ 보안 업데이트 1건: openssl 3.4.0 → 3.4.1
지금 적용할까요?
```

### 5.3 메모리 시스템

로컬 마크다운 파일 기반으로 세션 간 컨텍스트를 유지한다.

```
~/.config/blunux-ai/
├── config.toml              # AI Agent 설정
├── credentials/             # API 키 (권한 600)
│   ├── claude               # Claude API 키 또는 OAuth 토큰 경로
│   └── deepseek             # DeepSeek API 키
├── automations.toml         # 자동화 설정
├── memory/
│   ├── SYSTEM.md            # 시스템 정보 (자동 감지)
│   ├── USER.md              # 사용자 선호도 (학습됨)
│   ├── MEMORY.md            # 장기 기억
│   └── daily/
│       └── 2026-02-20.md    # 일일 대화 로그
├── logs/
│   └── commands.log         # 실행된 명령 로그
└── whatsapp/
    └── session/             # WhatsApp 세션 데이터
```

---

## 6. Provider Layer

### 6.1 Claude Provider

**Mode A: HTTP API 직접 호출 (Rust reqwest)**

```rust
// crates/ai-agent/src/providers/claude_api.rs
use reqwest::Client;

pub async fn chat(messages: &[Message], api_key: &str) -> Result<String> {
    let client = Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 4096,
            "system": BLUNUX_SYSTEM_PROMPT,
            "messages": messages,
            "tools": SYSTEM_TOOLS,
        }))
        .send()
        .await?;
    // 스트리밍 응답 처리
    Ok(parse_response(response).await?)
}
```

- blunux2 원칙 준수: Rust 바이너리, 추가 런타임 불필요
- `ANTHROPIC_API_KEY` 환경변수 또는 `~/.config/blunux-ai/credentials/claude`
- 토큰당 과금

**Mode B: Claude Code OAuth (subprocess)**

```rust
// crates/ai-agent/src/providers/claude_oauth.rs
use std::process::Command;

pub async fn chat(message: &str) -> Result<String> {
    let output = Command::new("claude")
        .args(["-p", message, "--output-format", "json"])
        .output()?;
    Ok(parse_claude_output(&output.stdout)?)
}
```

- Claude Pro/Max 구독에 포함 (추가 비용 없음)
- Node.js + Claude Code CLI 필요 (App Installer가 설치)
- 사용량 제한 있음

### 6.2 DeepSeek Provider

```rust
// crates/ai-agent/src/providers/deepseek.rs
pub async fn chat(messages: &[Message], api_key: &str) -> Result<String> {
    let client = Client::new();
    let response = client
        .post("https://api.deepseek.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({
            "model": "deepseek-chat",
            "messages": messages,
            "tools": SYSTEM_TOOLS,
        }))
        .send()
        .await?;
    Ok(parse_response(response).await?)
}
```

- OpenAI 호환 API 형식
- Claude 대비 매우 저렴
- Claude 구독 없는 사용자를 위한 대안

### 6.3 Provider 선택 흐름

```
사용자 설정 (blunux-ai setup)
         │
         ├─ Claude OAuth → Claude Code CLI 사용
         ├─ Claude API → reqwest HTTP 직접 호출
         └─ DeepSeek → reqwest HTTP 직접 호출
         │
         ▼
config.toml에 저장:
[ai_agent]
default_provider = "claude"
claude_mode = "oauth"
```

---

## 7. 보안

### 7.1 명령 실행 권한 모델

```
Level 0 (읽기 전용)    시스템 정보 조회, 로그 읽기        → 자동 실행
Level 1 (사용자 권한)  사용자 파일 편집, 프로그램 실행     → 자동 실행
Level 2 (관리자 권한)  패키지 설치, 서비스 관리            → WhatsApp에서 확인 후 실행
Level 3 (위험 작업)    디스크 포맷, 파티션 변경, rm -rf    → 이중 확인 + 경고
```

### 7.2 보안 원칙

- AI가 직접 `sudo` 실행 불가 — Level 2 이상은 반드시 사용자 확인
- Level 3 위험 명령은 화이트리스트 기반 차단 + 이중 확인
- API 키는 `~/.config/blunux-ai/credentials/` (권한 600)에 별도 저장
- config.toml에 API 키 절대 포함하지 않음 (ISO에 들어갈 수 있으므로)
- 모든 실행 명령은 `logs/commands.log`에 기록
- WhatsApp 브릿지는 허용된 전화번호만 응답 (화이트리스트)

### 7.3 WhatsApp 보안

```toml
# ~/.config/blunux-ai/config.toml
[whatsapp]
allowed_numbers = ["+821012345678"]   # 이 번호만 응답
require_prefix = false                 # true면 "/ai " prefix 필요
session_timeout = 3600                 # 1시간 무활동 시 재인증
```

### 7.4 WhatsApp ToS 리스크

whatsapp-web.js는 WhatsApp의 비공식 API이다. 계정 밴 위험이 이론적으로 존재한다.

**대응 방안:**
- 설정 마법사에서 리스크 안내 표시
- 별도 WhatsApp 번호 사용 권장
- 메시지 전송 빈도 제한 (분당 최대 5건)
- 향후 WhatsApp Business API 또는 Telegram Bot API 대안 제공

---

## 8. blunux2SB 통합

### 8.1 config.toml 확장

```toml
# 기존 config.toml에 추가되는 섹션

[packages.ai]
agent = true                      # AI Agent를 App Installer에 포함할지

[ai_agent]
enabled = true
default_provider = "claude"       # "claude" | "deepseek"
claude_mode = "oauth"             # "oauth" | "api"
whatsapp_bridge = true            # WhatsApp 연동

[ai_agent.automations]
health_check = true               # 일일 시스템 상태 보고
security_updates = true           # 보안 업데이트 알림
disk_warning = true               # 디스크 공간 경고
```

### 8.2 blunux-config crate 확장

```rust
// crates/blunux-config/src/lib.rs 에 추가

pub struct BlunuxConfig {
    // ... 기존 필드들 ...
    #[serde(default)]
    pub ai_agent: Option<AiAgent>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct AiAgent {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_provider")]
    pub default_provider: String,
    #[serde(default = "default_claude_mode")]
    pub claude_mode: String,
    #[serde(default)]
    pub whatsapp_bridge: bool,
}
```

`#[serde(default)]`로 기존 config.toml과 완전히 하위호환된다. `[ai_agent]` 섹션이 없어도 파싱 에러 없음.

### 8.3 Cargo.toml 확장

```toml
[workspace]
members = [
    "crates/blunux-config",     # 기존
    "crates/toml2cal",          # 기존
    "crates/wizard",            # 기존
    "crates/setup",             # 기존
    "crates/ai-agent",          # 신규
]
resolver = "2"
```

### 8.4 build.jl 수정

```julia
# build.jl 에 추가되는 로직

if config["packages"]["ai"]["agent"]
    # AI Agent 바이너리 빌드
    run(`cargo build --release -p ai-agent`)
    cp("target/release/blunux-ai-agent", "$airootfs/usr/bin/blunux-ai-agent")
    
    # 설치 스크립트 복사
    cp("blunux-ai-installer/install-ai-agent.sh",
       "$airootfs/usr/share/blunux/install-ai-agent.sh")
    
    # App Installer 카드 정의 복사
    cp("blunux-ai-installer/ai-agent.card.json",
       "$airootfs/usr/share/blunux-installer/cards/ai-agent.card.json")
end
```

### 8.5 ISO에 포함되는 것 vs 포함되지 않는 것

| 구성 요소 | ISO에 포함? | 설치 시점 |
|---|---|---|
| blunux-ai-agent (Rust 바이너리) | ✅ 조건부 | build.jl에서 빌드 |
| install-ai-agent.sh | ✅ | App Installer 카드용 |
| App Installer AI 카드 정의 | ✅ | App Installer에 표시 |
| Node.js | ❌ | App Installer에서 설치 |
| Claude Code CLI | ❌ | App Installer에서 설치 |
| blunux-whatsapp-bridge | ❌ | App Installer에서 설치 |
| WhatsApp 세션 데이터 | ❌ | 사용자 QR 스캔 시 생성 |
| API 키 | ❌ | 사용자가 직접 입력 |

**원칙 준수:**
- ✅ ISO 런타임: Rust + Bash만 (Node.js, Python 없음)
- ✅ config.toml 단일 소스
- ✅ Julia는 빌드 전용 (ISO에 미포함)
- ✅ API 키는 config.toml에 미포함
- ✅ 무거운 의존성은 post-install (App Installer 경유)

---

## 9. 기술 스택

| 구성 요소 | 기술 | 선택 이유 |
|---|---|---|
| AI Agent Core | Rust (reqwest, tokio, serde) | blunux2 원칙 준수, 성능 |
| WhatsApp 브릿지 | Node.js (whatsapp-web.js) | 유일한 현실적 WhatsApp 라이브러리 |
| IPC | Unix Domain Socket (JSON) | 가볍고 안전 |
| 메모리 | 마크다운 파일 (로컬) | 단순, 이식성, 사용자 편집 가능 |
| 스케줄러 | systemd timer | 리눅스 네이티브 |
| 설정 | TOML | blunux2 기존 형식과 통일 |
| 설치 | App Installer (Tauri) | blunux2 기존 도구 활용 |
| API 키 저장 | 파일 (권한 600) | 단순, keyring 대안 가능 |

---

## 10. 개발 로드맵

### Phase 1: Core MVP (v0.1.0) ✅ 완료

- [x] `crates/ai-agent/` Rust crate 생성
- [x] Claude API Provider 구현 (reqwest HTTP)
- [x] DeepSeek Provider 구현 (reqwest HTTP)
- [x] CLI 대화 모드 (`blunux-ai chat`)
- [x] 시스템 도구: pacman, systemctl, journalctl 래퍼
- [x] 안전장치: 권한 모델, 위험 명령 차단
- [x] 메모리 시스템 기본 구현
- [x] `blunux-config` crate에 `AiAgent` 필드 추가

### Phase 2: WhatsApp 브릿지 (v0.2.0) ✅ 완료

- [x] `blunux-whatsapp-bridge/` Node.js 프로젝트 생성
- [x] whatsapp-web.js 기반 메시지 수신/발신
- [x] QR 코드 스캔 설정 화면
- [x] Unix Socket IPC (에이전트 ↔ 브릿지)
- [x] WhatsApp 번호 화이트리스트
- [x] systemd user service 파일

### Phase 3: Claude Code OAuth + App Installer 통합 (v0.3.0) ✅ 완료

- [x] Claude Code OAuth Provider 구현 (subprocess)
- [x] `install-ai-agent.sh` 설치 스크립트
- [x] App Installer 카드 정의 (ai-agent.card.json)
- [x] `blunux-ai setup` 설정 마법사 (TUI)
- [x] `build.jl` 수정 (조건부 빌드)
- [x] config.toml 통합 테스트

### Phase 4: 자동화 에이전트 (v0.4.0) ✅ 완료

- [x] `blunux-ai daemon` 데몬 모드
- [x] `automations.toml` 파서 및 스케줄러
- [x] WhatsApp 자동 알림 (헬스체크, 업데이트, 디스크)
- [x] 일일 리포트 생성
- [x] `blunux-ai status` 상태 확인 명령

### Phase 5: 안정화 & 배포 (v1.0.0) ✅ 완료

- [x] AUR 패키지 생성 (blunux-ai-agent, blunux-wa-bridge)
- [x] Blunux ISO 통합 테스트
- [x] 한국어/영어 다국어 지원
- [x] 문서화 (설치 가이드, 사용법)
- [x] 보안 감사 (API 키 관리, 명령 실행 안전장치)

### 향후 계획

- [ ] Telegram Bot API 대안 (WhatsApp ToS 리스크 대비)
- [ ] KDE 시스템 트레이 위젯
- [ ] 로컬 웹 UI (localhost)
- [ ] 사용자 정의 도구(Skills) 시스템
- [ ] MCP (Model Context Protocol) 지원
- [ ] 로컬 AI 모델 지원 (Ollama)

---

## 11. 경쟁 분석

| 특성 | Blunux AI Agent | OpenClaw | Claude Code |
|---|---|---|---|
| OS 내장 | ✅ App Installer 원클릭 | ❌ 복잡한 수동 설치 | ❌ 별도 설치 |
| WhatsApp 지원 | ✅ | ✅ | ❌ |
| 시스템 관리 | ✅ pacman/systemd 네이티브 | ⚠️ 범용 | ❌ 코딩 전용 |
| 한국어 | ✅ 네이티브 | ⚠️ 제한적 | ⚠️ 제한적 |
| Claude OAuth | ✅ | ✅ | ✅ (본인) |
| DeepSeek | ✅ | ⚠️ 가능 | ❌ |
| 초보자 설치 난이도 | 카드 1번 클릭 | 터미널 여러 단계 | 터미널 |
| 24/7 자동화 | ✅ | ✅ | ❌ |
| GUI 앱 설치 | ✅ App Installer 통합 | ❌ | ❌ |
| 비용 | 무료 (AI 비용 별도) | 무료 (AI 비용 별도) | $20/월 |

---

## 12. 리스크 & 대응

| 리스크 | 영향 | 대응 방안 |
|---|---|---|
| WhatsApp ToS 위반 → 계정 밴 | 높음 | 별도 번호 권장, Telegram 대안 준비 |
| Claude Code OAuth 정책 변경 | 중간 | API 모드 fallback, DeepSeek 대안 |
| whatsapp-web.js 라이브러리 비호환 | 중간 | 버전 고정, 커뮤니티 모니터링 |
| AI가 위험한 시스템 명령 실행 | 높음 | 3단계 권한 모델, 명령 화이트리스트 |
| Node.js 의존성으로 ISO 원칙 위배 | 낮음 | post-install 설치로 원칙 유지 |
| WhatsApp 세션 만료 | 낮음 | 자동 재연결 + 사용자 알림 |

---

## 13. 참고

- [blunux2SB PRD](https://github.com/nidoit/blunux2SB/blob/main/prd.md) — 상위 프로젝트 PRD
- [Blunux App Installer](https://github.com/nidoit/appinstaller) — GUI 앱 설치 도구
- [OpenClaw](https://openclaw.ai/) — WhatsApp AI 에이전트 선행 사례
- [whatsapp-web.js](https://github.com/pedroslopez/whatsapp-web.js) — WhatsApp Web API
- [Claude API Docs](https://docs.anthropic.com/) — Claude API 문서
- [DeepSeek API](https://platform.deepseek.com/) — DeepSeek API 문서
