'use strict';

const { Client, LocalAuth } = require('whatsapp-web.js');
const qrcode = require('qrcode-terminal');
const { IpcClient } = require('./ipc');

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

        // Whitelist check
        if (this.config.allowedNumbers.length > 0) {
            const allowed = this.config.allowedNumbers.some(n =>
                normalised.includes(n.replace(/\D/g, ''))
            );
            if (!allowed) {
                console.log(`[bridge] Ignored unauthorised number: ${normalised}`);
                return;
            }
        }

        // Rate limit check
        if (!this.rateLimiter.allow(normalised)) {
            await msg.reply(
                'Too many messages. Please wait a minute before sending more.'
            );
            return;
        }

        console.log(`[bridge] Message from ${normalised}: ${body.slice(0, 80)}`);

        // Forward to daemon
        try {
            if (!this.ipc.connected) {
                await msg.reply('AI agent is not running. Please try again later.');
                return;
            }

            const response = await this.ipc.send({
                type: 'message',
                from: normalised,
                body,
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

    /** Strip WhatsApp suffix and non-digits for whitelist comparison. */
    _normalisePhone(from) {
        return from.replace('@c.us', '').replace(/\D/g, '');
    }

    async stop() {
        this.ipc.destroy();
        if (this.client) await this.client.destroy();
    }
}

module.exports = { WhatsAppBridge };
