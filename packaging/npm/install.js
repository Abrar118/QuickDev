'use strict';
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const https = require('node:https');
const { execFileSync } = require('node:child_process');
const { platformFor } = require('./lib/platform');
const { parseChecksums, sha256 } = require('./lib/verify');

const REPO = 'Abrar118/QuickDev';

// GET a URL into a Buffer, following GitHub's redirect to the asset CDN.
function httpsGet(url) {
  return new Promise((resolve, reject) => {
    https
      .get(url, { headers: { 'User-Agent': 'quickdev-npm-installer' } }, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          res.resume();
          resolve(httpsGet(res.headers.location));
          return;
        }
        if (res.statusCode !== 200) {
          res.resume();
          reject(new Error(`GET ${url} -> HTTP ${res.statusCode}`));
          return;
        }
        const chunks = [];
        res.on('data', (c) => chunks.push(c));
        res.on('end', () => resolve(Buffer.concat(chunks)));
        res.on('error', reject);
      })
      .on('error', reject);
  });
}

// Extract just the quickdev binary from the downloaded archive into destDir.
function extract(archivePath, archive, destDir, binaryName) {
  if (archive === 'tar.gz') {
    execFileSync('tar', ['-xzf', archivePath, '-C', destDir, binaryName], { stdio: 'inherit' });
  } else {
    // Pass paths as $args rather than interpolating into the command string, so
    // a path containing a quote can't break out of the PowerShell expression.
    execFileSync(
      'powershell',
      ['-NoProfile', '-Command',
       '& { Expand-Archive -LiteralPath $args[0] -DestinationPath $args[1] -Force }',
       archivePath, destDir],
      { stdio: 'inherit' }
    );
  }
}

async function main() {
  const { version } = require('./package.json');
  const { asset, archive, binaryName } = platformFor(process.platform, process.arch);

  const base = `https://github.com/${REPO}/releases/download/v${version}`;
  console.log(`quickdev: downloading ${asset} (v${version})...`);
  const [assetBuf, checksumsBuf] = await Promise.all([
    httpsGet(`${base}/${asset}`),
    httpsGet(`${base}/checksums-sha256.txt`),
  ]);

  const expected = parseChecksums(checksumsBuf.toString('utf8')).get(asset);
  if (!expected) throw new Error(`checksums file has no entry for ${asset}`);
  const actual = sha256(assetBuf);
  if (actual !== expected) {
    throw new Error(`checksum mismatch for ${asset} (expected ${expected}, got ${actual})`);
  }

  const binariesDir = path.join(__dirname, 'binaries');
  fs.mkdirSync(binariesDir, { recursive: true });

  // Stage the archive in a private temp dir (mkdtemp), not a predictable path —
  // matters under elevated/global installs. Removed wholesale in finally.
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'quickdev-'));
  const tmp = path.join(tmpDir, asset);
  fs.writeFileSync(tmp, assetBuf);
  try {
    extract(tmp, archive, binariesDir, binaryName);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }

  const binPath = path.join(binariesDir, binaryName);
  if (!fs.existsSync(binPath)) throw new Error(`extraction did not produce ${binaryName}`);
  if (process.platform !== 'win32') fs.chmodSync(binPath, 0o755);

  console.log(`quickdev: installed ${binaryName}`);
}

main().catch((err) => {
  console.error(`quickdev: install failed: ${err.message}`);
  console.error('Download a binary manually from https://github.com/Abrar118/QuickDev/releases');
  process.exit(1);
});
