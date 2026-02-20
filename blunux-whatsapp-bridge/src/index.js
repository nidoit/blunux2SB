'use strict';

const { loadConfig } = require('./config');
const { WhatsAppBridge } = require('./bridge');

async function main() {
    const config = loadConfig();

    if (!config.whatsappEnabled) {
        console.error(
            '[blunux-wa-bridge] WhatsApp is disabled in config.toml.\n' +
            'Set whatsapp_enabled = true under [agent] and restart.'
        );
        process.exit(1);
    }

    console.log('[blunux-wa-bridge] Starting...');
    console.log(`  Socket : ${config.socketPath}`);
    console.log(`  Allowed: ${config.allowedNumbers.length ? config.allowedNumbers.join(', ') : '(all)'}`);
    console.log(`  Rate   : ${config.maxMessagesPerMinute} msg/min per user`);

    const bridge = new WhatsAppBridge(config);

    process.on('SIGINT', async () => {
        console.log('\n[blunux-wa-bridge] Shutting down...');
        await bridge.stop();
        process.exit(0);
    });

    process.on('SIGTERM', async () => {
        await bridge.stop();
        process.exit(0);
    });

    await bridge.start();
}

main().catch(err => {
    console.error('[blunux-wa-bridge] Fatal error:', err);
    process.exit(1);
});
