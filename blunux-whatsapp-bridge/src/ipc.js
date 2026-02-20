'use strict';

const net = require('net');
const EventEmitter = require('events');

const RECONNECT_DELAYS_MS = [2000, 4000, 8000, 16000];

/**
 * IpcClient — persistent Unix socket connection to the Blunux AI daemon.
 *
 * Sends newline-delimited JSON IpcMessage objects and resolves pending
 * promises when the matching response arrives (matched by `to` field).
 *
 * Emits:
 *   'connected'   — socket connected
 *   'disconnected' — socket closed/error
 */
class IpcClient extends EventEmitter {
    constructor(socketPath) {
        super();
        this.socketPath = socketPath;
        this.socket = null;
        this.connected = false;
        this.buffer = '';
        /** @type {Map<string, {resolve: Function, reject: Function, timer: NodeJS.Timeout}>} */
        this.pending = new Map();
        this._reconnectAttempt = 0;
        this._destroyed = false;
    }

    connect() {
        if (this._destroyed) return;
        this.socket = net.createConnection(this.socketPath);

        this.socket.on('connect', () => {
            console.log('[ipc] Connected to daemon');
            this.connected = true;
            this._reconnectAttempt = 0;
            this.emit('connected');
        });

        this.socket.on('data', (chunk) => {
            this.buffer += chunk.toString();
            const lines = this.buffer.split('\n');
            this.buffer = lines.pop(); // keep incomplete line
            for (const line of lines) {
                const trimmed = line.trim();
                if (!trimmed) continue;
                try {
                    const msg = JSON.parse(trimmed);
                    this._handleIncoming(msg);
                } catch (e) {
                    console.error('[ipc] Bad JSON from daemon:', e.message);
                }
            }
        });

        this.socket.on('close', () => {
            this.connected = false;
            this.emit('disconnected');
            this._scheduleReconnect();
        });

        this.socket.on('error', (err) => {
            console.error('[ipc] Socket error:', err.message);
            // 'close' will fire after error
        });
    }

    destroy() {
        this._destroyed = true;
        if (this.socket) this.socket.destroy();
        for (const { reject, timer } of this.pending.values()) {
            clearTimeout(timer);
            reject(new Error('IPC client destroyed'));
        }
        this.pending.clear();
    }

    /**
     * Send a message and wait for a response (matched by phone number in `to`).
     * @param {object} msg  IpcMessage object
     * @param {number} timeoutMs
     * @returns {Promise<object>} response IpcMessage
     */
    send(msg, timeoutMs = 60000) {
        return new Promise((resolve, reject) => {
            if (!this.connected) {
                return reject(new Error('IPC not connected'));
            }

            const key = msg.from || '__global__';

            const timer = setTimeout(() => {
                this.pending.delete(key);
                reject(new Error(`IPC timeout for ${key}`));
            }, timeoutMs);

            this.pending.set(key, { resolve, reject, timer });

            const line = JSON.stringify(msg) + '\n';
            this.socket.write(line, 'utf8', (err) => {
                if (err) {
                    clearTimeout(timer);
                    this.pending.delete(key);
                    reject(err);
                }
            });
        });
    }

    /**
     * Send a ping and check daemon liveness.
     * @returns {Promise<boolean>}
     */
    async ping() {
        try {
            const resp = await this.send(
                { type: 'action', action: 'ping', from: '__ping__' },
                5000
            );
            return resp.body === 'pong';
        } catch {
            return false;
        }
    }

    _handleIncoming(msg) {
        if (msg.type !== 'response') return;
        const key = msg.to || '__global__';
        const pending = this.pending.get(key);
        if (pending) {
            clearTimeout(pending.timer);
            this.pending.delete(key);
            pending.resolve(msg);
        }
    }

    _scheduleReconnect() {
        if (this._destroyed) return;
        const delay = RECONNECT_DELAYS_MS[
            Math.min(this._reconnectAttempt, RECONNECT_DELAYS_MS.length - 1)
        ];
        this._reconnectAttempt++;
        console.log(`[ipc] Reconnecting in ${delay / 1000}s...`);
        setTimeout(() => this.connect(), delay);
    }
}

module.exports = { IpcClient };
