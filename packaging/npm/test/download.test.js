'use strict';
const test = require('node:test');
const assert = require('node:assert');
const { Readable } = require('node:stream');
const { readCapped } = require('../lib/download');

test('readCapped returns the full body when under the cap', async () => {
  const buf = await readCapped(Readable.from([Buffer.from('ab'), Buffer.from('cd')]), 4);
  assert.strictEqual(buf.toString(), 'abcd');
});

test('readCapped rejects once the body exceeds the cap', async () => {
  const stream = Readable.from([Buffer.from('ab'), Buffer.from('cde')]);
  await assert.rejects(readCapped(stream, 4), /exceeded 4 bytes/);
});

test('readCapped rejects a stream destroyed before it ends', async () => {
  const stream = new Readable({ read() {} });
  const result = readCapped(stream, 100);
  stream.push(Buffer.from('partial'));
  stream.destroy();
  await assert.rejects(result, /closed before download completed/);
});
