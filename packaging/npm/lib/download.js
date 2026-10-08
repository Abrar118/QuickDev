'use strict';

// Collect a readable stream into a Buffer, failing once it exceeds maxBytes.
// Without a cap a misbehaving server could stream until the install runs out
// of memory. Also fails if the stream closes before 'end' (connection dropped
// or destroyed by a deadline), so a truncated body never reaches the checksum.
function readCapped(stream, maxBytes) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let total = 0;
    let ended = false;
    stream.on('data', (c) => {
      total += c.length;
      if (total > maxBytes) {
        stream.destroy(new Error(`download exceeded ${maxBytes} bytes`));
        return;
      }
      chunks.push(c);
    });
    stream.on('end', () => {
      ended = true;
      resolve(Buffer.concat(chunks));
    });
    stream.on('error', reject);
    stream.on('close', () => {
      if (!ended) reject(new Error('connection closed before download completed'));
    });
  });
}

module.exports = { readCapped };
