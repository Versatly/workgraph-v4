'use strict';

const SUPPORTED_TARGETS = {
  darwin: {
    arm64: 'aarch64-apple-darwin',
    x64: 'x86_64-apple-darwin',
  },
  linux: {
    arm64: 'aarch64-unknown-linux-gnu',
    x64: 'x86_64-unknown-linux-gnu',
  },
  win32: {
    arm64: 'aarch64-pc-windows-msvc',
    x64: 'x86_64-pc-windows-msvc',
  },
};

function getBinaryExtension(platform = process.platform) {
  return platform === 'win32' ? '.exe' : '';
}

function getInstalledBinaryName(platform = process.platform) {
  return `workgraph-native${getBinaryExtension(platform)}`;
}

function getSupportedMatrix() {
  return Object.entries(SUPPORTED_TARGETS).flatMap(([platform, arches]) =>
    Object.entries(arches).map(([arch, target]) => ({ platform, arch, target })),
  );
}

function getPlatformInfo(platform = process.platform, arch = process.arch) {
  const target = SUPPORTED_TARGETS[platform]?.[arch];

  if (!target) {
    const supported = getSupportedMatrix()
      .map(({ platform: supportedPlatform, arch: supportedArch }) => `${supportedPlatform}/${supportedArch}`)
      .join(', ');
    const error = new Error(
      `Unsupported platform/architecture: ${platform}/${arch}. Supported prebuilt targets: ${supported}.`,
    );
    error.code = 'UNSUPPORTED_PLATFORM';
    throw error;
  }

  const ext = getBinaryExtension(platform);

  return {
    platform,
    arch,
    target,
    ext,
    binaryName: `workgraph${ext}`,
    installedBinaryName: getInstalledBinaryName(platform),
  };
}

function getAssetFileName(version, info = getPlatformInfo()) {
  return `workgraph-v${version}-${info.target}${info.ext}`;
}

function getReleaseUrl(version, info = getPlatformInfo()) {
  return `https://github.com/Versatly/workgraph-v4/releases/download/v${version}/${getAssetFileName(version, info)}`;
}

module.exports = {
  getAssetFileName,
  getBinaryExtension,
  getInstalledBinaryName,
  getPlatformInfo,
  getReleaseUrl,
  getSupportedMatrix,
};
