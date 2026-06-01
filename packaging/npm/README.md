# quickdev (npm)

Installs the [QuickDev](https://github.com/Abrar118/QuickDev) CLI as a prebuilt
binary.

```bash
npm install -g @panda-orion/quickdev
quickdev --help
```

On install, a postinstall script downloads the binary matching your platform
from the matching GitHub Release and verifies its SHA-256 checksum. This
requires network access and that npm postinstall scripts are enabled (i.e. not
`--ignore-scripts`).

Supported platforms: macOS (x64, arm64), Linux (x64, arm64), Windows (x64).
On any other platform, install fails with a clear message — download a binary
directly from the [Releases page](https://github.com/Abrar118/QuickDev/releases).

The package version tracks the QuickDev release it installs (e.g.
`@panda-orion/quickdev@0.2.0` installs release `v0.2.0`).
