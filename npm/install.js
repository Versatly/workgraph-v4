#!/usr/bin/env node
'use strict';

const fs = require('fs');
const https = require('https');
const path = require('path');
const { spawnSync } = require('child_process');
const { pipeline } = require('stream/promises');

const packageJson = require('../package.json');
const {
  getBinaryExtension,
  getInstalledBinaryName,
  getPlatformInfo,
  getReleaseUrl,
} = require('./platform');

const repoRoot = path.resolve(__dirname, '..');
const vendorDir = path.join(__dirname, 'vendor');

function log(message) {
  console.log(`[workgraph] ${message}`);
}

function warn(message) {
  console.warn(`[workgraph] ${message}`);
}

function removeIfExists(filePath) {
  try {
    fs.rmSync(filePath, { force: true });
  } catch {
    // ignore cleanup failures
  }
}

function ensureVendorDir() {
  fs.mkdirSync(vendorDir, { recursive: true });
}

function hasCargo() {
  const result = spawnSync('cargo', ['--version'], {
    cwd: repoRoot,
    stdio: 'ignore',
  });

  return result.status === 0;
}

async function downloadToFile(url, destinationPath, redirectCount = 0) {
  if (redirectCount > 5) {
    throw new Error(`Too many redirects while downloading ${url}.`);
  }

  await new Promise((resolve, reject) => {
    const request = https.get(
      url,
      {
        headers: {
          'User-Agent': `@versatly/workgraph/${packageJson.version}`,
        },
      },
      (response) => {
        const statusCode = response.statusCode ?? 0;

        if ([301, 302, 307, 308].includes(statusCode) && response.headers.location) {
          response.resume();
          downloadToFile(response.headers.location, destinationPath, redirectCount + 1)
            .then(resolve)
            .catch(reject);
          return;
        }

        if (statusCode !== 200) {
          response.resume();
          reject(new Error(`HTTP ${statusCode} while downloading ${url}.`));
          return;
        }

        const fileStream = fs.createWriteStream(destinationPath, { mode: 0o755 });
        pipeline(response, fileStream).then(resolve).catch(reject);
      },
    );

    request.on('error', reject);
  });
}

function buildWithCargo(nativeBinaryPath) {
  log('Falling back to a local cargo build for the native binary.');

  const build = spawnSync(
    'cargo',
    ['build', '--release', '-p', 'workgraph', '--bin', 'workgraph', '--locked'],
    {
      cwd: repoRoot,
      stdio: 'inherit',
    },
  );

  if (build.status !== 0) {
    throw new Error('Local cargo build failed.');
  }

  const builtBinaryPath = path.join(repoRoot, 'target', 'release', `workgraph${getBinaryExtension()}`);

  if (!fs.existsSync(builtBinaryPath)) {
    throw new Error(`Cargo build completed, but ${builtBinaryPath} was not produced.`);
  }

  fs.copyFileSync(builtBinaryPath, nativeBinaryPath);

  if (process.platform !== 'win32') {
    fs.chmodSync(nativeBinaryPath, 0o755);
  }
}

async function install() {
  ensureVendorDir();

  const nativeBinaryPath = path.join(vendorDir, getInstalledBinaryName());
  let downloadError = null;

  if (process.env.WORKGRAPH_INSTALL_SKIP_DOWNLOAD !== '1') {
    try {
      const platformInfo = getPlatformInfo();
      const releaseUrl = getReleaseUrl(packageJson.version, platformInfo);
      log(`Downloading prebuilt binary for ${platformInfo.target} from ${releaseUrl}`);
      removeIfExists(nativeBinaryPath);
      await downloadToFile(releaseUrl, nativeBinaryPath);
      if (process.platform !== 'win32') {
        fs.chmodSync(nativeBinaryPath, 0o755);
      }
      log(`Installed prebuilt binary to ${nativeBinaryPath}`);
      return;
    } catch (error) {
      downloadError = error;
      warn(`Prebuilt binary download failed: ${error.message}`);
      removeIfExists(nativeBinaryPath);
    }
  }

  if (process.env.WORKGRAPH_INSTALL_SKIP_BUILD === '1') {
    throw new Error(
      `Unable to install WorkGraph without a prebuilt binary. ${downloadError ? downloadError.message : 'Cargo fallback was disabled.'}`,
    );
  }

  if (!hasCargo()) {
    throw new Error(
      `No compatible prebuilt binary was available and cargo is not installed. Install Rust and rerun npm install, or publish a release asset for version v${packageJson.version}.`,
    );
  }

  buildWithCargo(nativeBinaryPath);
  log(`Installed native binary to ${nativeBinaryPath}`);
}

install().catch((error) => {
  console.error(`[workgraph] ${error.message}`);
  process.exit(1);
});
