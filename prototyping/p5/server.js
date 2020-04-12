#!/bin/env node

const express = require('express');
const expressWs = require('express-ws');
const fs = require('fs');
const fsp = fs.promises;

const app = express();
expressWs(app);

app.ws('/watch/:sketchName', (ws, req) => {
    const fireMessage = () => {
        ws.send('asdf');
    };
    let currentTimeout = null;
    const watcher = fs.watch(req.params.sketchName, {}, (event, filename) => {
        if (!filename || event !== 'change') return;

        if (currentTimeout) {
            clearTimeout(currentTimeout);
        }
        currentTimeout = setTimeout(fireMessage, 100);
    });
    ws.on('close', () => watcher.close());
});

const refreshWatcher = sketchName => `
    const connection = new WebSocket('ws://localhost:3030/watch/${sketchName}');
    connection.onmessage = () => {
        location.reload();
    };
`;

app.get('/:sketchName', async (req, res) => {
    try {
        const script = await fsp.readFile(req.params.sketchName);
        const html = `
            <html>
                <head>
                    <script>${refreshWatcher(req.params.sketchName)}</script>
                    <script src="https://cdn.jsdelivr.net/npm/p5@1.0.0/lib/p5.js"></script>
                    <script>${script}</script>
                </head>
                <body>
                </body>
            </html>
        `;
        res.status(200).send(html);
    } catch (_) {
        res.status(404).send(`Could not find a file named "${req.params.sketchName}".`);
    }
});

console.log('ready!');
app.listen(3030);
