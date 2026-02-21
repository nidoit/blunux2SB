#!/usr/bin/env julia
# Blunux2 ISO 통합 테스트 스위트
#
# ISO 빌드 시스템의 일관성과 완전성을 검증합니다.
#
# 실행 방법:
#   julia tests/iso_integration.jl
#
# 빌드 루트에서 실행해야 합니다 (blunux2SB/).

using TOML, Test

# 빌드 루트 감지 — 스크립트 위치에서 한 단계 위
const ROOT = abspath(joinpath(@__DIR__, ".."))

println("╔══════════════════════════════════════════════════════╗")
println("║      Blunux2 ISO 통합 테스트 (v1.0.0)                ║")
println("╠══════════════════════════════════════════════════════╣")
println("║  루트: $ROOT")
println("╚══════════════════════════════════════════════════════╝")
println()

# 경로 헬퍼
rp(args...) = joinpath(ROOT, args...)

@testset "Blunux2 ISO 통합 테스트" begin

    # ── 1. config.toml 구조 검증 ─────────────────────────────────────────
    @testset "config.toml 구조" begin
        @test isfile(rp("config.toml")) "config.toml이 없습니다"

        cfg = TOML.parsefile(rp("config.toml"))

        # 기본 섹션
        @test haskey(cfg, "blunux")
        @test haskey(cfg["blunux"], "version")
        @test haskey(cfg, "locale")
        @test haskey(cfg["locale"], "language")
        @test haskey(cfg, "kernel")
        @test haskey(cfg, "install")
        @test haskey(cfg, "packages")

        # AI Agent 섹션
        @test haskey(cfg["packages"], "ai")       "[packages.ai] 섹션 누락"
        @test haskey(cfg["packages"]["ai"], "agent") "[packages.ai].agent 키 누락"

        @test haskey(cfg, "ai_agent")             "[ai_agent] 섹션 누락"
        @test haskey(cfg["ai_agent"], "enabled")
        @test haskey(cfg["ai_agent"], "provider")
        @test haskey(cfg["ai_agent"], "automations") "[ai_agent.automations] 섹션 누락"

        @test haskey(cfg, "whatsapp")             "[whatsapp] 섹션 누락"
        @test haskey(cfg["whatsapp"], "allowed_numbers")
        @test haskey(cfg["whatsapp"], "require_prefix")
        @test haskey(cfg["whatsapp"], "session_timeout")
    end

    # ── 2. 빌드 시스템 파일 ───────────────────────────────────────────────
    @testset "빌드 시스템 파일" begin
        @test isfile(rp("build.jl"))   "build.jl 누락"
        @test isfile(rp("Cargo.toml")) "Cargo.toml 누락"

        # build.jl이 필수 기능을 포함하는지 확인
        content = read(rp("build.jl"), String)
        @test occursin("--profile-only", content) "build.jl에 --profile-only 옵션 없음"
        @test occursin("--skip-rust",    content) "build.jl에 --skip-rust 옵션 없음"
        @test occursin("build_rust",     content) "build.jl에 build_rust 함수 없음"
        @test occursin("mkarchiso",      content) "build.jl에 mkarchiso 호출 없음"
        @test occursin("packages.ai",    content) "build.jl에 packages.ai 분기 없음"
    end

    # ── 3. Rust 워크스페이스 ─────────────────────────────────────────────
    @testset "Rust 워크스페이스" begin
        cargo = TOML.parsefile(rp("Cargo.toml"))
        members = get(get(cargo, "workspace", Dict()), "members", String[])

        for crate in ["crates/blunux-config", "crates/toml2cal",
                      "crates/wizard", "crates/ai-agent"]
            @test crate in members "워크스페이스 멤버 누락: $crate"
        end
    end

    # ── 4. AI Agent 소스 파일 ────────────────────────────────────────────
    @testset "AI Agent 소스 파일" begin
        ai_src = rp("crates", "ai-agent", "src")

        for f in ["main.rs", "lib.rs", "agent.rs", "automations.rs",
                  "config.rs", "daemon.rs", "ipc.rs", "memory.rs",
                  "setup.rs", "strings.rs"]
            @test isfile(joinpath(ai_src, f)) "AI Agent 소스 누락: $f"
        end

        # 도구 하위 모듈
        for f in ["mod.rs", "system.rs", "packages.rs", "safety.rs"]
            @test isfile(joinpath(ai_src, "tools", f)) "tools/$f 누락"
        end

        # 프로바이더 하위 모듈
        for f in ["mod.rs", "claude_api.rs", "claude_oauth.rs", "deepseek.rs"]
            @test isfile(joinpath(ai_src, "providers", f)) "providers/$f 누락"
        end

        # 통합 테스트
        @test isfile(rp("crates", "ai-agent", "tests", "integration_test.rs"))
    end

    # ── 5. App Installer 에셋 ─────────────────────────────────────────────
    @testset "App Installer 에셋" begin
        @test isfile(rp("blunux-ai-installer", "install-ai-agent.sh"))
        @test isfile(rp("blunux-ai-installer", "ai-agent.card.json"))

        # 카드 JSON에 필수 키가 있는지 문자열로 확인
        card_json = read(rp("blunux-ai-installer", "ai-agent.card.json"), String)
        for key in ["\"id\"", "\"version\"", "\"install_script\"",
                    "\"name\"", "\"summary\""]
            @test occursin(key, card_json) "ai-agent.card.json에 $key 없음"
        end
        @test occursin("\"1.0.0\"", card_json) "ai-agent.card.json version이 1.0.0이 아님"

        # 설치 스크립트가 실행 가능한지 확인
        sh = rp("blunux-ai-installer", "install-ai-agent.sh")
        @test (filemode(sh) & 0o111) != 0 "install-ai-agent.sh가 실행 불가"
    end

    # ── 6. WhatsApp 브릿지 ───────────────────────────────────────────────
    @testset "WhatsApp 브릿지" begin
        bridge = rp("blunux-whatsapp-bridge")

        @test isfile(joinpath(bridge, "package.json"))

        # package.json version이 1.0.0인지 확인
        pkg_json = read(joinpath(bridge, "package.json"), String)
        @test occursin("\"1.0.0\"", pkg_json) "package.json version이 1.0.0이 아님"

        for f in ["index.js", "bridge.js", "ipc.js", "config.js"]
            @test isfile(joinpath(bridge, "src", f)) "bridge/src/$f 누락"
        end

        for svc in ["blunux-ai-agent.service", "blunux-wa-bridge.service"]
            @test isfile(joinpath(bridge, "systemd", svc)) "systemd/$svc 누락"
        end

        # 서비스 파일에 보안 설정이 있는지 확인
        agent_svc = read(joinpath(bridge, "systemd", "blunux-ai-agent.service"), String)
        @test occursin("NoNewPrivileges=true", agent_svc) "NoNewPrivileges 설정 누락"
        @test occursin("/usr/bin/blunux-ai",   agent_svc) "올바른 바이너리 경로 누락"
    end

    # ── 7. AUR 패키징 ────────────────────────────────────────────────────
    @testset "AUR 패키징" begin
        for pkg in ["blunux-ai-agent", "blunux-wa-bridge"]
            pkg_dir = rp("packaging", "aur", pkg)
            @test isdir(pkg_dir)            "AUR 디렉토리 누락: $pkg"
            @test isfile(joinpath(pkg_dir, "PKGBUILD"))  "PKGBUILD 누락: $pkg"
            @test isfile(joinpath(pkg_dir, ".SRCINFO"))  ".SRCINFO 누락: $pkg"

            pkgbuild = read(joinpath(pkg_dir, "PKGBUILD"), String)
            @test occursin("pkgname=$pkg",    pkgbuild) "PKGBUILD에 pkgname 없음: $pkg"
            @test occursin("pkgver=1.0.0",    pkgbuild) "PKGBUILD pkgver이 1.0.0이 아님: $pkg"
            @test occursin("package()",       pkgbuild) "PKGBUILD에 package() 없음: $pkg"
        end

        # blunux-wa-bridge가 blunux-ai-agent에 의존하는지 확인
        wa_pkgbuild = read(rp("packaging", "aur", "blunux-wa-bridge", "PKGBUILD"), String)
        @test occursin("blunux-ai-agent", wa_pkgbuild) "blunux-wa-bridge가 blunux-ai-agent에 의존하지 않음"
    end

    # ── 8. archiso 프로파일 ──────────────────────────────────────────────
    @testset "archiso 프로파일" begin
        profile = rp("profile")
        @test isdir(profile)                              "profile/ 디렉토리 누락"
        @test isfile(joinpath(profile, "profiledef.sh")) "profiledef.sh 누락"
        @test isfile(joinpath(profile, "pacman.conf"))   "pacman.conf 누락"
    end

    # ── 9. 문서 ──────────────────────────────────────────────────────────
    @testset "문서" begin
        @test isfile(rp("README.md"))     "README.md 누락"
        @test isfile(rp("prd.md"))        "prd.md 누락"
        @test isfile(rp("blunux-ai-agent", "blunux-ai-agent-PRD.md"))        "AI Agent PRD 누락"
        @test isfile(rp("blunux-ai-agent", "blunux-ai-agent-TDD.md"))        "AI Agent TDD 누락"
        @test isfile(rp("blunux-ai-agent", "blunux-ai-agent-USER-GUIDE.md")) "AI Agent User Guide 누락"

        readme = read(rp("README.md"), String)
        for section in ["빌드 방법", "AI Agent", "config.toml", "AUR"]
            @test occursin(section, readme) "README.md에 '$section' 섹션 없음"
        end
        # Phase 5가 완료 상태인지 확인 (예정이 아닌 완료)
        @test occursin("Phase 5", readme) "README.md에 Phase 5 로드맵 없음"
        @test !occursin(r"Phase 5.*예정", readme) "README.md에 Phase 5가 '예정'으로 표시됨 — '완료'로 변경 필요"
    end

end

println()
println("테스트 완료.")
