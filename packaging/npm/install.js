'use strict';
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const https = require('node:https');
const { execFileSync } = require('node:child_process');
const { platformFor } = require('./lib/platform');
const { parseChecksums, sha256 } = require('./lib/verify');
const { readCapped } = require('./lib/download');

const REPO = 'Abrar118/QuickDev';
// Release archives are under 1 MB; the caps leave ample headroom while still
// bounding memory if a server keeps sending.
const MAX_ASSET_BYTES = 32 * 1024 * 1024;
const MAX_CHECKSUMS_BYTES = 64 * 1024;
// Whole-install budget. The 30s socket timeout only catches a stalled
// connection; this also stops a server that trickles bytes forever.
const DEADLINE_MS = 5 * 60 * 1000;

// GET a URL into a Buffer, following GitHub's redirect to the asset CDN.
// Caps redirect depth, body size, and total time (`deadline`, an absolute
// Date.now() value shared across redirects) so a redirect loop, an oversized
// body, or a stalled connection fails cleanly instead of hanging the install.
function httpsGet(url, maxBytes, deadline, depth = 0) {
  if (depth > 5) return Promise.reject(new Error(`too many redirects fetching ${url}`));
  const remaining = deadline - Date.now();
  if (remaining <= 0) return Promise.reject(new Error(`download timed out: ${url}`));
  return new Promise((resolve, reject) => {
    const req = https.get(
      url,
      { headers: { 'User-Agent': 'quickdev-npm-installer' }, timeout: 30000 },
      (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          res.resume();
          clearTimeout(timer);
          resolve(httpsGet(res.headers.location, maxBytes, deadline, depth + 1));
          return;
        }
        if (res.statusCode !== 200) {
          res.resume();
          reject(new Error(`GET ${url} -> HTTP ${res.statusCode}`));
          return;
        }
        if (Number(res.headers['content-length']) > maxBytes) {
          res.destroy();
          reject(new Error(`${url} is larger than ${maxBytes} bytes`));
          return;
        }
        readCapped(res, maxBytes).then(resolve, reject);
      }
    );
    const timer = setTimeout(
      () => req.destroy(new Error(`download timed out: ${url}`)),
      remaining
    );
    req.on('close', () => clearTimeout(timer));
    req.on('timeout', () => req.destroy(new Error(`download timed out: ${url}`)));
    req.on('error', reject);
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
  const deadline = Date.now() + DEADLINE_MS;
  const [assetBuf, checksumsBuf] = await Promise.all([
    httpsGet(`${base}/${asset}`, MAX_ASSET_BYTES, deadline),
    httpsGet(`${base}/checksums-sha256.txt`, MAX_CHECKSUMS_BYTES, deadline),
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
