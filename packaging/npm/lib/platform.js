'use strict';

// (process.platform : process.arch) -> release asset descriptor.
const SUPPORTED = {
  'darwin:x64':   { asset: 'quickdev-macos-x86_64.tar.gz',  archive: 'tar.gz', binaryName: 'quickdev' },
  'darwin:arm64': { asset: 'quickdev-macos-aarch64.tar.gz', archive: 'tar.gz', binaryName: 'quickdev' },
  'linux:x64':    { asset: 'quickdev-linux-x86_64.tar.gz',  archive: 'tar.gz', binaryName: 'quickdev' },
  'linux:arm64':  { asset: 'quickdev-linux-aarch64.tar.gz', archive: 'tar.gz', binaryName: 'quickdev' },
  'win32:x64':    { asset: 'quickdev-windows-x86_64.zip',   archive: 'zip',    binaryName: 'quickdev.exe' },
};

// Resolve the prebuilt asset for a platform/arch pair, or throw a clear error
// listing the supported combinations. Returns a fresh object each call.
function platformFor(platform, arch) {
  const found = SUPPORTED[`${platform}:${arch}`];
  if (!found) {
    const supported = Object.keys(SUPPORTED).join(', ');
    throw new Error(
      `quickdev: unsupported platform ${platform}/${arch}. Supported: ${supported}. ` +
      'Download a binary from https://github.com/Abrar118/QuickDev/releases instead.'
    );
  }
  return { ...found };
}

module.exports = { platformFor, SUPPORTED };
