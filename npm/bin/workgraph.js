#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');
const { spawn } = require('child_process');

const { getInstalledBinaryName } = require('../platform');

const binaryPath = path.resolve(__dirname, '..', 'vendor', getInstalledBinaryName());

if (!fs.existsSync(binaryPath)) {
  console.error(
    '[workgraph] Native binary is missing. Reinstall `@versatly/workgraph`, or run `node npm/install.js` from the package directory.',
  );
  process.exit(1);
}

const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
});

child.on('error', (error) => {
  console.error(`[workgraph] Failed to launch ${binaryPath}: ${error.message}`);
  process.exit(1);
});

child.on('close', (code, signal) => {
  if (signal) {
    console.error(`[workgraph] Native binary exited from signal ${signal}.`);
    process.exit(1);
  }

  process.exit(code ?? 0);
});
