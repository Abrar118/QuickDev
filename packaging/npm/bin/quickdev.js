#!/usr/bin/env node
'use strict';
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { binaryPath } = require('../lib/launcher');

const binPath = binaryPath(process.platform, path.join(__dirname, '..'));

if (!fs.existsSync(binPath)) {
  console.error(
    'quickdev: native binary not found. Reinstall with `npm install -g quickdev` ' +
    '(postinstall scripts must be enabled), or grab a binary from ' +
    'https://github.com/Abrar118/QuickDev/releases'
  );
  process.exit(1);
}

const result = spawnSync(binPath, process.argv.slice(2), { stdio: 'inherit' });
if (result.error) {
  console.error(`quickdev: failed to run binary: ${result.error.message}`);
  process.exit(1);
}
process.exit(result.status ?? 1);
