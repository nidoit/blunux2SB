'use strict';

const fs = require('fs');
const os = require('os');
const path = require('path');

/**
 * Parse a minimal TOML subset — only the [whatsapp] section.
 * Handles: string arrays, integers, strings, booleans.
 */
function parseTomlSection(content, section) {
    const result = {};
    const sectionRe = new RegExp(`^\\[${section}\\]\\s*$`, 'm');
    const match = sectionRe.exec(content);
    if (!match) return result;

    const start = match.index + match[0].length;
    const nextSection = /^\[(?!\[)/m.exec(content.slice(start));
    const chunk = nextSection
        ? content.slice(start, start + nextSection.index)
        : content.slice(start);

    for (const line of chunk.split('\n')) {
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith('#')) continue;
        const eq = trimmed.indexOf('=');
        if (eq < 0) continue;

        const key = trimmed.slice(0, eq).trim();
        const rawVal = trimmed.slice(eq + 1).trim();

        if (rawVal.startsWith('[')) {
            // Array of strings: ["+821234", "+829876"]
            const inner = rawVal.slice(1, rawVal.lastIndexOf(']'));
            result[key] = inner
                .split(',')
                .map(s => s.trim().replace(/^["']|["']$/g, ''))
                .filter(Boolean);
        } else if (rawVal === 'true' || rawVal === 'false') {
            result[key] = rawVal === 'true';
        } else if (/^\d+$/.test(rawVal)) {
            result[key] = parseInt(rawVal, 10);
        } else {
            result[key] = rawVal.replace(/^["']|["']$/g, '');
        }
    }
    return result;
}

function loadConfig() {
    const configDir = process.env.BLUNUX_AI_CONFIG_DIR
        || path.join(os.homedir(), '.config', 'blunux-ai');
    const configPath = path.join(configDir, 'config.toml');

    let content = '';
    try {
        content = fs.readFileSync(configPath, 'utf8');
    } catch {
        console.error(`[config] Cannot read ${configPath} — using defaults`);
    }

    const agent = parseTomlSection(content, 'agent');
    const whatsapp = parseTomlSection(content, 'whatsapp');

    // Derive socket path: /run/user/<uid>/blunux-ai.sock
    const uid = process.getuid ? process.getuid() : 1000;
    const socketPath = process.env.BLUNUX_AI_SOCKET
        || `/run/user/${uid}/blunux-ai.sock`;

    return {
        configDir,
        socketPath,
        whatsappEnabled: agent.whatsapp_enabled === true,
        allowedNumbers: whatsapp.allowed_numbers || [],
        maxMessagesPerMinute: whatsapp.max_messages_per_minute || 5,
        requirePrefix: whatsapp.require_prefix === true,
        sessionTimeout: whatsapp.session_timeout || 3600,
    };
}

/**
 * Persist a newly paired phone number into config.toml's [whatsapp]
 * allowed_numbers so it survives bridge restarts.
 * Numbers are stored in the documented "+821012345678" format.
 */
function persistAllowedNumber(configDir, number) {
    const configPath = path.join(configDir, 'config.toml');
    let content = '';
    try {
        content = fs.readFileSync(configPath, 'utf8');
    } catch {
        // No config yet — create a minimal one holding just this setting.
    }

    if (/^allowed_numbers *=/m.test(content)) {
        content = content.replace(
            /^allowed_numbers *= *\[([^\]]*)\]/m,
            (_full, inner) => {
                const existing = inner
                    .split(',')
                    .map(s => s.trim().replace(/^["']|["']$/g, ''))
                    .filter(Boolean);
                if (!existing.includes(number)) existing.push(number);
                return `allowed_numbers = [${existing.map(n => `"${n}"`).join(', ')}]`;
            }
        );
    } else if (/^\[whatsapp\]\s*$/m.test(content)) {
        content = content.replace(
            /^\[whatsapp\]\s*$/m,
            `[whatsapp]\nallowed_numbers = ["${number}"]`
        );
    } else {
        if (content && !content.endsWith('\n')) content += '\n';
        content += `\n[whatsapp]\nallowed_numbers = ["${number}"]\n`;
    }

    fs.mkdirSync(configDir, { recursive: true });
    fs.writeFileSync(configPath, content);
}

module.exports = { loadConfig, persistAllowedNumber };
