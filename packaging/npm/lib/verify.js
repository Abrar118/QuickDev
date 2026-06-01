'use strict';
const crypto = require('node:crypto');

// Parse `sha256sum` output: each line is "<64-hex><spaces>[*]<filename>".
// Lines that don't match are skipped. Returns Map<filename, lowercaseHex>.
function parseChecksums(text) {
  const map = new Map();
  for (const line of text.split('\n')) {
    const m = line.trim().match(/^([0-9a-fA-F]{64})\s+\*?(.+)$/);
    if (m) map.set(m[2].trim(), m[1].toLowerCase());
  }
  return map;
}

function sha256(buffer) {
  return crypto.createHash('sha256').update(buffer).digest('hex');
}

// True iff the buffer's sha256 equals expectedHex (case-insensitive).
function verify(buffer, expectedHex) {
  return sha256(buffer) === String(expectedHex).toLowerCase();
}

module.exports = { parseChecksums, sha256, verify };
