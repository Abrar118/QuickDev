'use strict';
const path = require('node:path');

// Absolute path to the extracted native binary inside the package.
function binaryPath(platform, packageRoot) {
  const name = platform === 'win32' ? 'quickdev.exe' : 'quickdev';
  return path.join(packageRoot, 'binaries', name);
}

module.exports = { binaryPath };
