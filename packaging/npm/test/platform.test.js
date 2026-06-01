'use strict';
const test = require('node:test');
const assert = require('node:assert');
const { platformFor } = require('../lib/platform');

test('maps darwin arm64 to the aarch64 tarball', () => {
  assert.deepStrictEqual(platformFor('darwin', 'arm64'), {
    asset: 'quickdev-macos-aarch64.tar.gz', archive: 'tar.gz', binaryName: 'quickdev',
  });
});

test('maps darwin x64 to the x86_64 tarball', () => {
  assert.deepStrictEqual(platformFor('darwin', 'x64'), {
    asset: 'quickdev-macos-x86_64.tar.gz', archive: 'tar.gz', binaryName: 'quickdev',
  });
});

test('maps linux x64 to the x86_64 tarball', () => {
  assert.deepStrictEqual(platformFor('linux', 'x64'), {
    asset: 'quickdev-linux-x86_64.tar.gz', archive: 'tar.gz', binaryName: 'quickdev',
  });
});

test('maps linux arm64 to the aarch64 tarball', () => {
  assert.deepStrictEqual(platformFor('linux', 'arm64'), {
    asset: 'quickdev-linux-aarch64.tar.gz', archive: 'tar.gz', binaryName: 'quickdev',
  });
});

test('maps win32 x64 to the zip with an .exe binary', () => {
  assert.deepStrictEqual(platformFor('win32', 'x64'), {
    asset: 'quickdev-windows-x86_64.zip', archive: 'zip', binaryName: 'quickdev.exe',
  });
});

test('throws on an unsupported platform', () => {
  assert.throws(() => platformFor('freebsd', 'x64'), /unsupported platform freebsd\/x64/);
});

test('throws on an unsupported arch', () => {
  assert.throws(() => platformFor('linux', 'ia32'), /unsupported platform linux\/ia32/);
});
