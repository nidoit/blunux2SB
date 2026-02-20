# Blunux AI Agent — 사용자 가이드 / User Guide

- **버전 / Version:** 0.1.0
- **날짜 / Date:** 2026-02-20
- **언어 / Language:** 한국어 + English
- **라이선스 / License:** MIT

---

> **언어 선택 / Language Selection**
> - 🇰🇷 한국어: [아래 한국어 섹션으로 이동](#한국어-가이드)
> - 🇺🇸 English: [Jump to English Guide](#english-guide)

---

# 한국어 가이드

## 목차

1. [Blunux AI Agent란?](#1-blunux-ai-agent란)
2. [시스템 요구 사항](#2-시스템-요구-사항)
3. [설치](#3-설치)
4. [최초 설정](#4-최초-설정)
5. [사용법](#5-사용법)
6. [지원 명령 예시](#6-지원-명령-예시)
7. [메모리 관리](#7-메모리-관리)
8. [상태 확인](#8-상태-확인)
9. [보안 안내](#9-보안-안내)
10. [문제 해결](#10-문제-해결)

---

## 1. Blunux AI Agent란?

Blunux AI Agent는 자연어로 Linux 시스템을 관리할 수 있게 해주는 AI 도우미입니다.

"크롬 설치해줘", "시스템 업데이트 해줘", "오늘 에러 로그 보여줘" 같은 한국어 명령으로 시스템을 관리할 수 있습니다.

### 주요 기능

| 기능 | 설명 |
|---|---|
| 자연어 시스템 관리 | 한국어/영어로 패키지 설치, 서비스 관리, 디스크 확인 등 |
| AI 모델 선택 | Claude (Anthropic) 또는 DeepSeek 사용 가능 |
| 안전한 실행 | 위험한 명령은 자동 차단, 중요 명령은 확인 후 실행 |
| 기억 기능 | 대화 내용과 시스템 정보를 로컬에 저장 |
| 한국어 지원 | `config.toml`의 언어 설정에 따라 자동 한국어 UI |

---

## 2. 시스템 요구 사항

| 항목 | 최소 사양 |
|---|---|
| 운영체제 | Blunux (Arch Linux 기반) |
| 인터넷 연결 | AI API 호출에 필요 |
| Rust | 1.75 이상 (소스 빌드 시) |
| Node.js | 20 이상 (Claude OAuth 모드 사용 시) |
| AI 계정 | Claude Pro/Max 구독 또는 Anthropic API Key 또는 DeepSeek API Key |

---

## 3. 설치

### 방법 1: Blunux App Installer (권장)

1. KDE 앱 실행기에서 **"Blunux Installer"** 실행
2. **"🤖 AI Agent"** 카드 클릭
3. **[설치]** 버튼 클릭
4. 진행 로그 확인 (약 3~5분 소요)
5. 설치 완료 후 자동으로 최초 설정 마법사 시작

### 방법 2: 수동 설치 (터미널)

```bash
# 저장소 클론
git clone https://github.com/nidoit/blunux2SB.git
cd blunux2SB

# Rust 빌드
cargo build --release -p ai-agent

# 바이너리 설치
sudo install -Dm755 target/release/blunux-ai /usr/local/bin/blunux-ai

# 최초 설정
blunux-ai setup
```

### 설치 확인

```bash
blunux-ai --version
# blunux-ai 0.1.0

blunux-ai status
# Provider: Claude (API Mode)
# Model: claude-sonnet-4-6
# Memory: ~/.config/blunux-ai/
# Language: 한국어
```

---

## 4. 최초 설정

설치 후 `blunux-ai setup` 명령을 실행하면 단계별 설정 마법사가 시작됩니다.

### 설정 단계

```
╔══════════════════════════════════════╗
║    🤖 Blunux AI Agent 설정 마법사   ║
╚══════════════════════════════════════╝

[1/5] AI 모델 선택
  > Claude (Anthropic) — 권장
    DeepSeek — 대안 모델

[2/5] 연결 방식 선택 (Claude 선택 시)
  > OAuth — Claude Pro/Max 구독 사용 (추가 비용 없음)
    API Key — 토큰 단위 과금

[3/5] 모델 선택
  > claude-sonnet-4-6 — 빠르고 균형잡힌 모델 (권장)
    claude-opus-4-6 — 더 강력하지만 느림

[4/5] 설정 저장 중...
  ✓ ~/.config/blunux-ai/config.toml 저장됨
  ✓ 메모리 디렉토리 생성됨
  ✓ 시스템 정보 수집됨

[5/5] 완료!
  blunux-ai chat 으로 시작하세요
```

### Claude OAuth 설정

OAuth 모드를 선택한 경우, Claude Code CLI가 필요합니다:

```bash
# Claude Code 설치 (자동으로 진행됩니다)
npm install -g @anthropic-ai/claude-code

# Claude 로그인 (브라우저가 열립니다)
claude login
```

Claude Pro 또는 Max 구독이 있으면 추가 비용 없이 사용할 수 있습니다.

### Claude API Key 설정

1. [https://console.anthropic.com](https://console.anthropic.com) 접속
2. API Keys 메뉴에서 새 키 생성
3. 마법사의 API Key 입력란에 붙여넣기 (화면에 표시되지 않음)
4. 자동으로 유효성 검증 후 저장

### DeepSeek API Key 설정

1. [https://platform.deepseek.com](https://platform.deepseek.com) 접속
2. API Keys에서 새 키 생성
3. 마법사의 API Key 입력란에 붙여넣기
4. 자동으로 유효성 검증 후 저장

---

## 5. 사용법

### 기본 채팅

```bash
blunux-ai chat
# 또는 인수 없이 실행
blunux-ai
```

실행 화면:

```
🤖 Blunux AI Agent v0.1.0
   Claude (claude-sonnet-4-6) | 한국어 모드
   종료: Ctrl+C

사용자: 안녕? 내 시스템 상태 알려줘

AI: 안녕하세요! 시스템 상태를 확인해드릴게요.

    📊 시스템 상태 (2026-02-20 09:15)
    • CPU: Intel Core i7-12700 @ 2.10GHz
    • RAM: 4.2GB / 16GB 사용 중 (26%)
    • 디스크: 48GB / 256GB 사용 중 (18%)
    • 업타임: 5일 2시간

사용자: 크롬 설치해줘

AI: Google Chrome을 설치하겠습니다.
    실행할 명령: yay -S google-chrome
    계속하시겠습니까? (y/n): y

    ⏳ 설치 중...
    ✅ google-chrome 설치 완료!

사용자: ^C
   Blunux AI Agent를 종료합니다. 안녕히 계세요!
```

---

## 6. 지원 명령 예시

### 패키지 관리

| 사용자 입력 | 실행 명령 |
|---|---|
| "크롬 설치해줘" | `yay -S google-chrome` |
| "vlc 설치" | `yay -S vlc` |
| "vlc 삭제해줘" | `yay -Rns vlc` |
| "설치된 패키지 목록 보여줘" | `pacman -Q` |

### 시스템 업데이트

| 사용자 입력 | 실행 명령 |
|---|---|
| "시스템 업데이트" | `sudo pacman -Syu` |
| "보안 업데이트 확인해줘" | `pacman -Qu` (분석 후 보고) |

### 서비스 관리

| 사용자 입력 | 실행 명령 |
|---|---|
| "SSH 서버 켜줘" | `sudo systemctl enable --now sshd` |
| "블루투스 재시작" | `sudo systemctl restart bluetooth` |
| "도커 상태 확인" | `systemctl status docker` |

### 디스크 & 시스템

| 사용자 입력 | 실행 명령 |
|---|---|
| "디스크 용량 확인" | `df -h` (AI 분석 포함) |
| "메모리 사용량" | `free -h` |
| "CPU 많이 쓰는 프로세스" | `ps aux --sort=-%cpu` |

### 로그 분석

| 사용자 입력 | 실행 명령 |
|---|---|
| "오늘 에러 로그 보여줘" | `journalctl --since today -p err` |
| "SSH 로그인 기록" | `journalctl -u sshd --since today` |

### 네트워크

| 사용자 입력 | 실행 명령 |
|---|---|
| "와이파이 목록 보여줘" | `nmcli device wifi list` |
| "IP 주소 확인" | `ip addr show` |

---

## 7. 메모리 관리

Blunux AI Agent는 대화 내용과 시스템 정보를 로컬 파일에 저장합니다. 모든 데이터는 `~/.config/blunux-ai/` 에 저장되며, 외부로 전송되지 않습니다.

### 메모리 보기

```bash
blunux-ai memory show
```

출력 예시:

```
=== SYSTEM.md ===
# 시스템 정보
- 배포판: Blunux 2.0 (Arch Linux 기반)
- 커널: 6.12.1-arch1-1
- 데스크톱: KDE Plasma 6.2
- CPU: Intel Core i7-12700
- RAM: 16GB
- 디스크: 256GB (18% 사용)

=== USER.md ===
# 사용자 설정
- 선호 브라우저: Firefox
- 개발 언어: Rust, Python

=== 오늘의 로그 (2026-02-20) ===
09:15 - 시스템 상태 확인
09:16 - google-chrome 설치 (확인 후 실행)
```

### 시스템 정보 새로고침

```bash
blunux-ai memory refresh
# SYSTEM.md를 현재 시스템 정보로 업데이트합니다.
```

### 메모리 초기화

```bash
blunux-ai memory clear
# 주의: 오늘의 로그와 장기 메모리(MEMORY.md)를 삭제합니다.
# 계속하시겠습니까? (y/n):
```

---

## 8. 상태 확인

```bash
blunux-ai status
```

출력 예시:

```
🤖 Blunux AI Agent 상태

프로바이더:  Claude (API Mode)
모델:        claude-sonnet-4-6
언어:        한국어
안전 모드:   활성화

메모리:
  위치: ~/.config/blunux-ai/
  SYSTEM.md: 업데이트됨 (2026-02-20)
  USER.md: 1.2 KB
  MEMORY.md: 0.8 KB
  오늘 로그: 3줄

WhatsApp:   비활성화 (Phase 2 예정)
```

---

## 9. 보안 안내

### 명령 실행 권한 모델

Blunux AI Agent는 세 단계의 권한 모델로 안전하게 명령을 실행합니다:

| 단계 | 예시 | 동작 |
|---|---|---|
| ✅ 안전 | `df -h`, `free -h`, `ps aux`, 로그 조회 | 자동 실행 |
| ⚠️ 확인 필요 | 패키지 설치/삭제, 서비스 변경, 시스템 업데이트 | 사용자 확인 후 실행 |
| 🚫 차단 | `rm -rf /`, 디스크 포맷, fork bomb 등 | 절대 실행 안 함 |

### 실행 로그

모든 실행된 명령은 `~/.config/blunux-ai/logs/commands.log`에 기록됩니다:

```
[2026-02-20T09:15:32Z] SAFE       df -h
[2026-02-20T09:16:01Z] CONFIRMED  yay -S google-chrome
[2026-02-20T09:17:45Z] BLOCKED    rm -rf /home/blu
```

### 자격증명 보안

- API 키는 `~/.config/blunux-ai/credentials/` 에 저장 (권한: 600)
- 로컬에만 저장, 외부 전송 없음
- AI는 자기 자신의 설정 파일을 수정할 수 없음

---

## 10. 문제 해결

### 문제: API 인증 오류

```
오류: Authentication failed — check credentials
```

**해결:**
```bash
# API 키 재설정
blunux-ai setup
```

### 문제: 명령을 찾을 수 없음

```
bash: blunux-ai: command not found
```

**해결:**
```bash
# PATH 확인
echo $PATH

# 바이너리 위치 확인
ls /usr/local/bin/blunux-ai

# PATH에 추가 (bash)
echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.bash_profile
source ~/.bash_profile
```

### 문제: Claude OAuth — 로그인 필요

```
오류: OAuth subprocess exited 1: not logged in
```

**해결:**
```bash
claude login
# 브라우저에서 Claude 계정으로 로그인
```

### 문제: 응답이 너무 느림

- `claude-opus-4-6` → `claude-sonnet-4-6` 모델로 변경
- `~/.config/blunux-ai/config.toml`에서 `model = "claude-sonnet-4-6"` 으로 수정

### 문제: 메모리 파일 손상

```bash
# 시스템 정보 새로고침
blunux-ai memory refresh

# 전체 초기화 (대화 기록 삭제됨)
blunux-ai memory clear
blunux-ai setup  # 재설정 없이 메모리만 초기화
```

### 로그 확인

```bash
# 명령 실행 로그
cat ~/.config/blunux-ai/logs/commands.log

# 오늘 대화 로그
cat ~/.config/blunux-ai/memory/daily/$(date +%Y-%m-%d).md
```

---

---

# English Guide

## Table of Contents

1. [What is Blunux AI Agent?](#1-what-is-blunux-ai-agent)
2. [System Requirements](#2-system-requirements)
3. [Installation](#3-installation)
4. [First-time Setup](#4-first-time-setup)
5. [Usage](#5-usage)
6. [Supported Command Examples](#6-supported-command-examples)
7. [Memory Management](#7-memory-management)
8. [Status Check](#8-status-check)
9. [Security Guide](#9-security-guide)
10. [Troubleshooting](#10-troubleshooting)

---

## 1. What is Blunux AI Agent?

Blunux AI Agent is an AI assistant that lets you manage your Linux system using natural language.

Instead of memorizing terminal commands, simply type "install Chrome", "check disk space", or "show today's error logs" in plain language.

### Key Features

| Feature | Description |
|---|---|
| Natural language system management | Install packages, manage services, check disk via plain language |
| AI model choice | Claude (Anthropic) or DeepSeek |
| Safe execution | Dangerous commands auto-blocked; important commands require confirmation |
| Memory | Conversation history and system info stored locally |
| Korean & English | UI language auto-detected from `config.toml` locale settings |

---

## 2. System Requirements

| Item | Minimum |
|---|---|
| OS | Blunux (Arch Linux-based) |
| Internet | Required for AI API calls |
| Rust | 1.75+ (for source build) |
| Node.js | 20+ (for Claude OAuth mode) |
| AI Account | Claude Pro/Max subscription OR Anthropic API Key OR DeepSeek API Key |

---

## 3. Installation

### Option 1: Blunux App Installer (Recommended)

1. Open **"Blunux Installer"** from the KDE app launcher
2. Find and click the **"🤖 AI Agent"** card
3. Click **[Install]**
4. Watch the progress log (~3–5 minutes)
5. The first-time setup wizard starts automatically

### Option 2: Manual Installation (Terminal)

```bash
# Clone the repository
git clone https://github.com/nidoit/blunux2SB.git
cd blunux2SB

# Build with Rust
cargo build --release -p ai-agent

# Install binary
sudo install -Dm755 target/release/blunux-ai /usr/local/bin/blunux-ai

# Run setup
blunux-ai setup
```

### Verify Installation

```bash
blunux-ai --version
# blunux-ai 0.1.0

blunux-ai status
# Provider: Claude (API Mode)
# Model: claude-sonnet-4-6
# Memory: ~/.config/blunux-ai/
# Language: English
```

---

## 4. First-time Setup

Run `blunux-ai setup` to start the interactive setup wizard.

### Setup Steps

```
╔══════════════════════════════════════╗
║   🤖 Blunux AI Agent Setup Wizard   ║
╚══════════════════════════════════════╝

[1/5] Choose AI Provider
  > Claude (Anthropic) — Recommended
    DeepSeek — Alternative model

[2/5] Connection Mode (if Claude selected)
  > OAuth — Use Claude Pro/Max subscription (no extra cost)
    API Key — Pay per token

[3/5] Select Model
  > claude-sonnet-4-6 — Fast & balanced (Recommended)
    claude-opus-4-6   — More capable, slower

[4/5] Saving configuration...
  ✓ ~/.config/blunux-ai/config.toml saved
  ✓ Memory directory created
  ✓ System information collected

[5/5] Done!
  Start with: blunux-ai chat
```

### Claude OAuth Setup

OAuth mode requires the Claude Code CLI:

```bash
# Claude Code is installed automatically during setup
npm install -g @anthropic-ai/claude-code

# Log in to Claude (opens browser)
claude login
```

Claude Pro or Max subscribers can use this at no additional cost.

### Claude API Key Setup

1. Visit [https://console.anthropic.com](https://console.anthropic.com)
2. Go to API Keys → Create new key
3. Paste the key when prompted (hidden input)
4. Key is automatically validated and saved

### DeepSeek API Key Setup

1. Visit [https://platform.deepseek.com](https://platform.deepseek.com)
2. Go to API Keys → Create new key
3. Paste the key when prompted
4. Key is automatically validated and saved

---

## 5. Usage

### Start a Chat

```bash
blunux-ai chat
# or simply
blunux-ai
```

Example session:

```
🤖 Blunux AI Agent v0.1.0
   Claude (claude-sonnet-4-6) | English mode
   Exit: Ctrl+C

You: Hello! What's my system status?

AI: Here's your current system status:

    📊 System Status (2026-02-20 09:15)
    • CPU: Intel Core i7-12700 @ 2.10GHz
    • RAM: 4.2GB / 16GB used (26%)
    • Disk: 48GB / 256GB used (18%)
    • Uptime: 5 days 2 hours

You: Install Chrome

AI: I'll install Google Chrome.
    Command to run: yay -S google-chrome
    Proceed? (y/n): y

    ⏳ Installing...
    ✅ google-chrome installed successfully!

You: ^C
   Goodbye! Blunux AI Agent stopped.
```

---

## 6. Supported Command Examples

### Package Management

| User Input | Command Executed |
|---|---|
| "Install Chrome" | `yay -S google-chrome` |
| "Install VLC" | `yay -S vlc` |
| "Remove VLC" | `yay -Rns vlc` |
| "Show installed packages" | `pacman -Q` |

### System Updates

| User Input | Command Executed |
|---|---|
| "Update system" | `sudo pacman -Syu` |
| "Check for security updates" | `pacman -Qu` (with AI analysis) |

### Service Management

| User Input | Command Executed |
|---|---|
| "Enable SSH server" | `sudo systemctl enable --now sshd` |
| "Restart Bluetooth" | `sudo systemctl restart bluetooth` |
| "Check Docker status" | `systemctl status docker` |

### Disk & System

| User Input | Command Executed |
|---|---|
| "Check disk space" | `df -h` (with AI analysis) |
| "Show memory usage" | `free -h` |
| "What's using the most CPU?" | `ps aux --sort=-%cpu` |

### Log Analysis

| User Input | Command Executed |
|---|---|
| "Show today's error logs" | `journalctl --since today -p err` |
| "SSH login history" | `journalctl -u sshd --since today` |

### Network

| User Input | Command Executed |
|---|---|
| "List WiFi networks" | `nmcli device wifi list` |
| "What's my IP address?" | `ip addr show` |

---

## 7. Memory Management

Blunux AI Agent stores conversation history and system information in local files under `~/.config/blunux-ai/`. All data stays on your machine — nothing is sent externally.

### View Memory

```bash
blunux-ai memory show
```

Example output:

```
=== SYSTEM.md ===
# System Information
- Distro: Blunux 2.0 (Arch Linux-based)
- Kernel: 6.12.1-arch1-1
- Desktop: KDE Plasma 6.2
- CPU: Intel Core i7-12700
- RAM: 16GB
- Disk: 256GB (18% used)

=== USER.md ===
# User Preferences
- Preferred browser: Firefox
- Development languages: Rust, Python

=== Today's Log (2026-02-20) ===
09:15 - Checked system status
09:16 - Installed google-chrome (confirmed by user)
```

### Refresh System Info

```bash
blunux-ai memory refresh
# Updates SYSTEM.md with current hardware and OS information.
```

### Clear Memory

```bash
blunux-ai memory clear
# Warning: deletes today's log and long-term memory (MEMORY.md).
# Proceed? (y/n):
```

---

## 8. Status Check

```bash
blunux-ai status
```

Example output:

```
🤖 Blunux AI Agent Status

Provider:    Claude (API Mode)
Model:       claude-sonnet-4-6
Language:    English
Safe Mode:   Enabled

Memory:
  Location: ~/.config/blunux-ai/
  SYSTEM.md: Updated (2026-02-20)
  USER.md: 1.2 KB
  MEMORY.md: 0.8 KB
  Today's log: 3 lines

WhatsApp:    Disabled (Phase 2 — coming soon)
```

---

## 9. Security Guide

### Command Permission Model

Blunux AI Agent uses a three-tier permission system:

| Tier | Examples | Behavior |
|---|---|---|
| ✅ Safe | `df -h`, `free -h`, `ps aux`, log reading | Auto-execute |
| ⚠️ Confirm | Package install/remove, service changes, system update | Ask user first |
| 🚫 Blocked | `rm -rf /`, disk format, fork bomb | Never execute |

### Command Log

Every executed command is logged to `~/.config/blunux-ai/logs/commands.log`:

```
[2026-02-20T09:15:32Z] SAFE       df -h
[2026-02-20T09:16:01Z] CONFIRMED  yay -S google-chrome
[2026-02-20T09:17:45Z] BLOCKED    rm -rf /home/blu
```

### Credential Security

- API keys are stored in `~/.config/blunux-ai/credentials/` with permissions 600
- Stored locally only — never transmitted externally
- The AI cannot modify its own configuration files

---

## 10. Troubleshooting

### Problem: API Authentication Error

```
Error: Authentication failed — check credentials
```

**Fix:**
```bash
blunux-ai setup
# Re-enter your API key
```

### Problem: Command Not Found

```
bash: blunux-ai: command not found
```

**Fix:**
```bash
# Check PATH
echo $PATH

# Check binary location
ls /usr/local/bin/blunux-ai

# Add to PATH (bash)
echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.bash_profile
source ~/.bash_profile
```

### Problem: Claude OAuth — Login Required

```
Error: OAuth subprocess exited 1: not logged in
```

**Fix:**
```bash
claude login
# Log in with your Claude account in the browser
```

### Problem: Slow Responses

- Switch from `claude-opus-4-6` to `claude-sonnet-4-6`
- Edit `~/.config/blunux-ai/config.toml`: set `model = "claude-sonnet-4-6"`

### Problem: Corrupted Memory Files

```bash
# Refresh system info
blunux-ai memory refresh

# Full reset (deletes conversation history)
blunux-ai memory clear
```

### View Logs

```bash
# Command execution log
cat ~/.config/blunux-ai/logs/commands.log

# Today's session log
cat ~/.config/blunux-ai/memory/daily/$(date +%Y-%m-%d).md
```

---

## Config File Reference

`~/.config/blunux-ai/config.toml`:

```toml
[agent]
provider = "claude"        # "claude" | "deepseek"
claude_mode = "oauth"      # "oauth" | "api"
model = "claude-sonnet-4-6"  # "claude-sonnet-4-6" | "claude-opus-4-6" | "deepseek-chat" | "deepseek-coder"
language = "auto"          # "auto" | "ko" | "en"
safe_mode = true
```

---

*Blunux AI Agent is part of the blunux2SB project. Source: https://github.com/nidoit/blunux2SB*
