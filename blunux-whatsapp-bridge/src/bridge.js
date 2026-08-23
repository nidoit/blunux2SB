'use strict';

const crypto = require('crypto');
const { Client, LocalAuth } = require('whatsapp-web.js');
const qrcode = require('qrcode-terminal');
const { IpcClient } = require('./ipc');
const { persistAllowedNumber } = require('./config');

/** Wrong PAIR-code attempts tolerated before pairing shuts down. */
const MAX_PAIR_ATTEMPTS = 10;

/**
 * Per-user rate limiter.
 * Tracks message timestamps within a rolling one-minute window.
 */
class RateLimiter {
    constructor(maxPerMinute) {
        this.maxPerMinute = maxPerMinute;
        /** @type {Map<string, number[]>} */
        this.history = new Map();
    }

    /** Returns true if the user is allowed to send a message now. */
    allow(phone) {
        const now = Date.now();
        const cutoff = now - 60_000;
        const times = (this.history.get(phone) || []).filter(t => t > cutoff);
        if (times.length >= this.maxPerMinute) return false;
        times.push(now);
        this.history.set(phone, times);
        return true;
    }
}

/**
 * WhatsAppBridge — connects WhatsApp Web to the Blunux AI daemon via IPC.
 */
class WhatsAppBridge {
    constructor(config) {
        this.config = config;
        this.ipc = new IpcClient(config.socketPath);
        this.rateLimiter = new RateLimiter(config.maxMessagesPerMinute);
        this.client = null;
        /** Digit-only allowed numbers, including ones paired at runtime. */
        this.allowedNumbers = new Set(
            config.allowedNumbers.map(n => n.replace(/\D/g, '')).filter(Boolean)
        );
        this.pairingCode = null;
        this.pairAttempts = 0;
    }

    async start() {
        // Connect to daemon socket first
        this.ipc.connect();
        this.ipc.on('connected', () => {
            console.log('[bridge] IPC connected to blunux-ai daemon');
        });
        this.ipc.on('disconnected', () => {
            console.warn('[bridge] IPC disconnected — will retry');
        });

        // Set up WhatsApp client
        this.client = new Client({
            authStrategy: new LocalAuth({ dataPath: '.wwebjs_auth' }),
            puppeteer: {
                headless: true,
                args: [
                    '--no-sandbox',
                    '--disable-setuid-sandbox',
                    '--disable-dev-shm-usage',
                    '--disable-accelerated-2d-canvas',
                    '--no-first-run',
                    '--no-zygote',
                    '--single-process',
                    '--disable-gpu',
                ],
            },
        });

        this.client.on('qr', (qr) => {
            console.log('\n[bridge] Scan this QR code with WhatsApp:\n');
            qrcode.generate(qr, { small: true });
        });

        this.client.on('authenticated', () => {
            console.log('[bridge] WhatsApp authenticated');
        });

        this.client.on('ready', () => {
            console.log('[bridge] WhatsApp client ready');
            if (this.allowedNumbers.size === 0) {
                this.pairingCode = crypto.randomInt(0, 1_000_000)
                    .toString()
                    .padStart(6, '0');
                console.log('');
                console.log('┌──────────────────────────────────────────────────────┐');
                console.log('│  No allowed numbers configured — pairing required.   │');
                console.log(`│  Send this from YOUR WhatsApp:   PAIR ${this.pairingCode}         │`);
                console.log('│  Until paired, every sender is denied.               │');
                console.log('└──────────────────────────────────────────────────────┘');
                console.log('');
            }
            this._startNotificationPoller();
        });

        this.client.on('disconnected', (reason) => {
            console.warn(`[bridge] WhatsApp disconnected: ${reason}`);
        });

        this.client.on('message', (msg) => this._onMessage(msg));

        await this.client.initialize();
    }

    async _onMessage(msg) {
        // Only handle individual chats (not groups)
        if (msg.isGroupMsg) return;

        const phone = msg.from; // e.g. "821012345678@c.us"
        const normalised = this._normalisePhone(phone);
        const body = (msg.body || '').trim();

        if (!body) return;

        // Whitelist check. An empty list means NOBODY is allowed — the owner
        // registers their number by sending the PAIR code printed at startup.
        if (this.allowedNumbers.size === 0) {
            await this._handlePairing(msg, normalised, body);
            return;
        }
        const configured = [...this.allowedNumbers];
        const allowed = configured.some(n => normalised.includes(n));
        if (!allowed) {
            console.log(
                `[bridge] Ignored unauthorised number: ${normalised} ` +
                `(add it to allowed_numbers in config.toml to permit it)`
            );
            return;
        }
        // Substring matching is deprecated: a future release will require the
        // full number in allowed_numbers. Warn now so nobody gets locked out.
        const exact = configured.some(n => normalised === n);
        if (!exact) {
            console.warn(
                `[bridge] DEPRECATION: sender ${normalised} was accepted by partial match only. ` +
                `Add the full number (with country code) to allowed_numbers — ` +
                `exact matching will become mandatory in a future release.`
            );
        }

        // Command prefix gate ("/ai ...") — enforced when require_prefix = true.
        let command = body;
        if (this.config.requirePrefix) {
            if (!/^\/ai(\s|$)/i.test(body)) return;
            command = body.replace(/^\/ai\s*/i, '').trim();
            if (!command) return;
        }

        // Rate limit check
        if (!this.rateLimiter.allow(normalised)) {
            await msg.reply(
                'Too many messages. Please wait a minute before sending more.'
            );
            return;
        }

        console.log(`[bridge] Message from ${normalised}: ${command.slice(0, 80)}`);

        // Forward to daemon
        try {
            if (!this.ipc.connected) {
                await msg.reply('AI agent is not running. Please try again later.');
                return;
            }

            const response = await this.ipc.send({
                type: 'message',
                from: normalised,
                body: command,
                timestamp: new Date().toISOString(),
            }, 120_000); // 2-minute timeout for long commands

            const reply = response.body || '(no response)';
            // WhatsApp limits messages to ~65535 chars; truncate if needed
            const truncated = reply.length > 4000
                ? reply.slice(0, 3997) + '...'
                : reply;
            await msg.reply(truncated);
        } catch (err) {
            console.error('[bridge] Error forwarding message:', err.message);
            await msg.reply(`Error: ${err.message}`);
        }
    }

    /**
     * Owner pairing: with no allowed numbers configured, the only accepted
     * message is "PAIR <code>" matching the code printed at startup. The
     * sender becomes the registered owner (persisted to config.toml).
     */
    async _handlePairing(msg, normalised, body) {
        const m = body.match(/^pair\s+(\d{6})$/i);
        if (!m) {
            console.log(
                `[bridge] Denied ${normalised} — no numbers registered. ` +
                `Send "PAIR <code>" (see startup log) to register.`
            );
            return;
        }
        if (!this.pairingCode) {
            console.log(`[bridge] Pairing attempt from ${normalised} but pairing is not active.`);
            return;
        }
        if (m[1] !== this.pairingCode) {
            this.pairAttempts += 1;
            console.warn(
                `[bridge] Wrong PAIR code from ${normalised} ` +
                `(attempt ${this.pairAttempts}/${MAX_PAIR_ATTEMPTS})`
            );
            if (this.pairAttempts >= MAX_PAIR_ATTEMPTS) {
                this.pairingCode = null;
                console.error(
                    '[bridge] Too many wrong PAIR codes — pairing disabled. ' +
                    'Restart the bridge to generate a new code.'
                );
            }
            return;
        }

        this.allowedNumbers.add(normalised);
        this.pairingCode = null;
        try {
            persistAllowedNumber(this.config.configDir, `+${normalised}`);
            console.log(`[bridge] Paired owner ${normalised} (saved to config.toml)`);
        } catch (err) {
            console.error(
                `[bridge] Paired ${normalised} for this session, but could not save config: ${err.message}`
            );
        }
        await msg.reply(
            '✅ Paired! This number can now control the Blunux AI agent.\n' +
            '등록 완료! 이 번호로 Blunux AI 에이전트를 사용할 수 있습니다.'
        );
    }

    /**
     * Start polling the daemon every 15 seconds for automation notifications.
     * When the daemon has pending notifications, they are sent via WhatsApp.
     */
    _startNotificationPoller() {
        const POLL_INTERVAL_MS = 15_000;

        const poll = async () => {
            if (!this.ipc.connected) return;
            try {
                const items = await this.ipc.pollNotifications();
                for (const item of items) {
                    const phone = item.to;
                    const body  = item.body || '';
                    if (!phone || !body) continue;

                    // Format as WhatsApp JID: strip leading + and append @c.us
                    const jid = phone.replace(/^\+/, '') + '@c.us';
                    try {
                        await this.client.sendMessage(jid, body);
                        console.log(`[bridge] Notification sent to ${phone}: ${body.slice(0, 60)}...`);
                    } catch (err) {
                        console.error(`[bridge] Failed to send notification to ${phone}:`, err.message);
                    }
                }
            } catch (err) {
                console.error('[bridge] Notification poll error:', err.message);
            }
        };

        this._pollTimer = setInterval(poll, POLL_INTERVAL_MS);
    }

    /** Strip WhatsApp suffix and non-digits for whitelist comparison. */
    _normalisePhone(from) {
        return from.replace('@c.us', '').replace(/\D/g, '');
    }

    async stop() {
        if (this._pollTimer) clearInterval(this._pollTimer);
        this.ipc.destroy();
        if (this.client) await this.client.destroy();
    }
}

module.exports = { WhatsAppBridge };
