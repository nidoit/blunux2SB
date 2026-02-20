use crate::config::Language;

pub fn welcome(lang: &Language) -> &'static str {
    match lang {
        Language::Korean => "Blunux AI Agent에 오신 것을 환영합니다!",
        Language::English => "Welcome to Blunux AI Agent!",
    }
}

pub fn prompt(lang: &Language) -> &'static str {
    match lang {
        Language::Korean => "사용자",
        Language::English => "You",
    }
}

pub fn thinking(lang: &Language) -> &'static str {
    match lang {
        Language::Korean => "생각 중...",
        Language::English => "Thinking...",
    }
}

pub fn confirm_action(lang: &Language) -> &'static str {
    match lang {
        Language::Korean => "계속하시겠습니까? (y/n): ",
        Language::English => "Proceed? (y/n): ",
    }
}

pub fn cancelled(lang: &Language) -> &'static str {
    match lang {
        Language::Korean => "취소되었습니다.",
        Language::English => "Cancelled.",
    }
}

pub fn blocked(lang: &Language) -> &'static str {
    match lang {
        Language::Korean => "안전 정책에 의해 차단되었습니다.",
        Language::English => "Blocked by safety policy.",
    }
}

pub fn goodbye(lang: &Language) -> &'static str {
    match lang {
        Language::Korean => "Blunux AI Agent를 종료합니다. 안녕히 계세요!",
        Language::English => "Goodbye! Blunux AI Agent stopped.",
    }
}

pub fn error_prefix(lang: &Language) -> &'static str {
    match lang {
        Language::Korean => "오류",
        Language::English => "Error",
    }
}

pub fn exit_hint(lang: &Language) -> &'static str {
    match lang {
        Language::Korean => "종료: Ctrl+C",
        Language::English => "Exit: Ctrl+C",
    }
}

pub fn confirm_install(lang: &Language, package: &str) -> String {
    match lang {
        Language::Korean => format!("{package} 패키지를 설치합니다."),
        Language::English => format!("Installing package: {package}"),
    }
}

pub fn confirm_remove(lang: &Language, package: &str) -> String {
    match lang {
        Language::Korean => format!("{package} 패키지를 삭제합니다."),
        Language::English => format!("Removing package: {package}"),
    }
}

pub fn confirm_service(lang: &Language, action: &str, service: &str) -> String {
    match lang {
        Language::Korean => format!("{service} 서비스를 {action}합니다."),
        Language::English => format!("{action} service: {service}"),
    }
}

pub fn confirm_update(lang: &Language) -> &'static str {
    match lang {
        Language::Korean => "시스템 전체 업데이트를 실행합니다.",
        Language::English => "Running full system update.",
    }
}

pub fn confirm_command(lang: &Language, command: &str) -> String {
    match lang {
        Language::Korean => format!("실행할 명령: {command}"),
        Language::English => format!("Command to run: {command}"),
    }
}

pub fn tool_executing(lang: &Language, tool_name: &str) -> String {
    match lang {
        Language::Korean => format!("실행 중: {tool_name}"),
        Language::English => format!("Executing: {tool_name}"),
    }
}

// ── Setup wizard strings ─────────────────────────────────────────────────────

pub fn setup_welcome(lang: &Language) -> &'static str {
    match lang {
        Language::Korean => "Blunux AI Agent 설정 마법사",
        Language::English => "Blunux AI Agent Setup Wizard",
    }
}

pub fn setup_provider_prompt(lang: &Language) -> &'static str {
    match lang {
        Language::Korean => "AI 프로바이더를 선택하세요",
        Language::English => "Select AI provider",
    }
}

pub fn setup_claude_mode_prompt(lang: &Language) -> &'static str {
    match lang {
        Language::Korean => "Claude 연결 방식을 선택하세요",
        Language::English => "Select Claude connection mode",
    }
}

pub fn setup_model_prompt(lang: &Language) -> &'static str {
    match lang {
        Language::Korean => "모델을 선택하세요",
        Language::English => "Select model",
    }
}

pub fn setup_api_key_prompt(lang: &Language) -> &'static str {
    match lang {
        Language::Korean => "API 키를 입력하세요",
        Language::English => "Enter your API key",
    }
}

pub fn setup_done(lang: &Language) -> &'static str {
    match lang {
        Language::Korean => "설정 완료! 'blunux-ai chat'으로 시작하세요.",
        Language::English => "Setup complete! Start with 'blunux-ai chat'.",
    }
}

pub fn setup_whatsapp_coming_soon(lang: &Language) -> &'static str {
    match lang {
        Language::Korean => "WhatsApp 브리지: 추후 지원 예정 (Phase 2)",
        Language::English => "WhatsApp bridge: Coming soon (Phase 2)",
    }
}
