'use strict';
const test = require('node:test');
const assert = require('node:assert');
const { parseChecksums, sha256, verify } = require('../lib/verify');

test('parses sha256sum-style lines into a filename->hash map', () => {
  const text = [
    'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  quickdev-linux-x86_64.tar.gz',
    'a'.repeat(64) + '  quickdev-macos-aarch64.tar.gz',
  ].join('\n');
  const map = parseChecksums(text);
  assert.strictEqual(
    map.get('quickdev-linux-x86_64.tar.gz'),
    'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'
  );
  assert.strictEqual(map.get('quickdev-macos-aarch64.tar.gz'), 'a'.repeat(64));
});

test('ignores blank and malformed lines', () => {
  const map = parseChecksums('\n   \nnot a checksum line\n');
  assert.strictEqual(map.size, 0);
});

test('verify accepts a matching hash case-insensitively', () => {
  const buf = Buffer.from('hello');
  assert.ok(verify(buf, sha256(buf).toUpperCase()));
});

test('verify rejects a mismatching hash', () => {
  assert.ok(!verify(Buffer.from('hello'), 'b'.repeat(64)));
});
