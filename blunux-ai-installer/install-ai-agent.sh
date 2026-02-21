#!/usr/bin/env bash
# install-ai-agent.sh — Install Blunux AI Agent (Phase 1 + 2)
#
# Usage:
#   ./install-ai-agent.sh              # install everything
#   ./install-ai-agent.sh --no-bridge  # skip WhatsApp bridge
#   ./install-ai-agent.sh --uninstall  # remove everything
#
# Requires: cargo, node (>=18), npm

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_BIN="/usr/local/bin"
BRIDGE_LIB="/usr/local/lib/blunux-wa-bridge"
SYSTEMD_USER_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
info()    { echo -e "${GREEN}[install]${NC} $*"; }
warn()    { echo -e "${YELLOW}[install]${NC} $*"; }
error()   { echo -e "${RED}[install]${NC} $*" >&2; }

# ── Argument parsing ──────────────────────────────────────────────────────────
INSTALL_BRIDGE=true
UNINSTALL=false
for arg in "$@"; do
    case "$arg" in
        --no-bridge)   INSTALL_BRIDGE=false ;;
        --uninstall)   UNINSTALL=true ;;
        *) error "Unknown argument: $arg"; exit 1 ;;
    esac
done

# ── Uninstall ─────────────────────────────────────────────────────────────────
if $UNINSTALL; then
    info "Uninstalling Blunux AI Agent..."

    # Stop and disable services
    for svc in blunux-ai-agent blunux-wa-bridge; do
        if systemctl --user is-active --quiet "$svc" 2>/dev/null; then
            systemctl --user stop "$svc"
        fi
        if systemctl --user is-enabled --quiet "$svc" 2>/dev/null; then
            systemctl --user disable "$svc"
        fi
        rm -f "${SYSTEMD_USER_DIR}/${svc}.service"
    done
    systemctl --user daemon-reload 2>/dev/null || true

    # Remove binaries and bridge
    rm -f "${INSTALL_BIN}/blunux-ai"
    rm -rf "$BRIDGE_LIB"

    info "Uninstall complete."
    info "Config and credentials at ~/.config/blunux-ai/ were NOT removed."
    exit 0
fi

# ── Preflight checks ──────────────────────────────────────────────────────────
check_command() {
    if ! command -v "$1" &>/dev/null; then
        error "Required command not found: $1"
        error "Please install it and re-run this script."
        exit 1
    fi
}

check_command cargo
check_command node

node_version=$(node --version | sed 's/v//' | cut -d. -f1)
if [[ "$node_version" -lt 18 ]]; then
    error "Node.js >= 18 required (found $(node --version))."
    exit 1
fi

if $INSTALL_BRIDGE; then
    check_command npm
fi

# ── Build Rust agent ──────────────────────────────────────────────────────────
info "Building blunux-ai (Rust)..."
(cd "$REPO_ROOT" && cargo build --release --bin blunux-ai 2>&1)

BIN="$REPO_ROOT/target/release/blunux-ai"
if [[ ! -f "$BIN" ]]; then
    error "Build failed: $BIN not found"
    exit 1
fi

info "Installing blunux-ai to ${INSTALL_BIN}/"
install -Dm755 "$BIN" "${INSTALL_BIN}/blunux-ai"

# ── Install WhatsApp bridge ───────────────────────────────────────────────────
if $INSTALL_BRIDGE; then
    BRIDGE_SRC="$REPO_ROOT/blunux-whatsapp-bridge"
    if [[ ! -d "$BRIDGE_SRC" ]]; then
        warn "Bridge source not found at $BRIDGE_SRC — skipping bridge install"
        INSTALL_BRIDGE=false
    else
        info "Installing WhatsApp bridge to ${BRIDGE_LIB}/"
        rm -rf "$BRIDGE_LIB"
        mkdir -p "$BRIDGE_LIB"
        cp -r "$BRIDGE_SRC/src" "$BRIDGE_LIB/"
        cp "$BRIDGE_SRC/package.json" "$BRIDGE_LIB/"

        info "Installing Node.js dependencies..."
        (cd "$BRIDGE_LIB" && npm install --omit=dev 2>&1)
    fi
fi

# ── Install systemd user services ─────────────────────────────────────────────
mkdir -p "$SYSTEMD_USER_DIR"

SYSTEMD_SRC="$REPO_ROOT/blunux-whatsapp-bridge/systemd"

install -Dm644 "${SYSTEMD_SRC}/blunux-ai-agent.service" \
    "${SYSTEMD_USER_DIR}/blunux-ai-agent.service"

if $INSTALL_BRIDGE; then
    install -Dm644 "${SYSTEMD_SRC}/blunux-wa-bridge.service" \
        "${SYSTEMD_USER_DIR}/blunux-wa-bridge.service"
fi

systemctl --user daemon-reload

# ── Enable and start services ─────────────────────────────────────────────────
info "Enabling blunux-ai-agent service..."
systemctl --user enable blunux-ai-agent.service

if $INSTALL_BRIDGE; then
    info "Enabling blunux-wa-bridge service..."
    systemctl --user enable blunux-wa-bridge.service
fi

# ── First-time setup prompt ───────────────────────────────────────────────────
if [[ ! -f "$HOME/.config/blunux-ai/config.toml" ]]; then
    echo ""
    info "No config found. Running first-time setup..."
    echo ""
    blunux-ai setup
fi

# ── Start services ────────────────────────────────────────────────────────────
echo ""
info "Starting blunux-ai-agent daemon..."
systemctl --user start blunux-ai-agent.service

if $INSTALL_BRIDGE; then
    info "Starting blunux-wa-bridge..."
    systemctl --user start blunux-wa-bridge.service
fi

# ── Summary ───────────────────────────────────────────────────────────────────
echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Blunux AI Agent installed!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "  Binary : ${INSTALL_BIN}/blunux-ai"
[[ $INSTALL_BRIDGE == true ]] && echo "  Bridge : ${BRIDGE_LIB}/"
echo ""
echo "  Usage  : blunux-ai chat"
echo "  Status : blunux-ai status"
echo "  Daemon : systemctl --user status blunux-ai-agent"
if $INSTALL_BRIDGE; then
    echo "  Bridge : systemctl --user status blunux-wa-bridge"
    echo ""
    warn "WhatsApp: scan QR code on first start"
    echo "         journalctl --user -u blunux-wa-bridge -f"
fi
echo ""
