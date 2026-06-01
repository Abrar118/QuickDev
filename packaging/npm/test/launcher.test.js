'use strict';
const test = require('node:test');
const assert = require('node:assert');
const path = require('node:path');
const { binaryPath } = require('../lib/launcher');

test('resolves the unix binary path under binaries/', () => {
  assert.strictEqual(binaryPath('linux', '/pkg'), path.join('/pkg', 'binaries', 'quickdev'));
});

test('resolves the darwin binary path under binaries/', () => {
  assert.strictEqual(binaryPath('darwin', '/pkg'), path.join('/pkg', 'binaries', 'quickdev'));
});

test('resolves the windows binary path with an .exe suffix', () => {
  assert.strictEqual(binaryPath('win32', '/pkg'), path.join('/pkg', 'binaries', 'quickdev.exe'));
});
