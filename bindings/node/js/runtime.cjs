'use strict';
const { existsSync } = require('node:fs');
const { dirname, join } = require('node:path');
const pkg = require('../package.json');

exports.resolveRuntime = function resolveRuntime(kind) {
  let target = `${process.platform}-${process.arch}`;
  if (process.platform === 'linux') {
    const glibc = process.report?.getReport()?.header?.glibcVersionRuntime;
    if (!glibc) throw new Error('Monoprint currently requires glibc on Linux; musl/Alpine is not in the release matrix.');
    target += '-gnu';
  } else if (process.platform === 'win32') target += '-msvc';
  const supported = ['darwin-arm64', 'darwin-x64', 'linux-x64-gnu', 'linux-arm64-gnu', 'win32-x64-msvc'];
  if (!supported.includes(target)) throw new Error(`Unsupported Monoprint target: ${target}`);
  const file = kind === 'cli'
    ? `monoprint.${target}${process.platform === 'win32' ? '.exe' : ''}`
    : `monoprint.${target}.node`;
  // Local builds only. Published root packages contain no platform-specific binary.
  const local = join(__dirname, '..', 'native', file);
  if (existsSync(local)) return local;
  const name = `${pkg.name}-${target}`;
  let manifest;
  try {
    manifest = require.resolve(`${name}/package.json`);
  } catch (cause) {
    throw new Error(`Missing Monoprint native package ${name}@${pkg.version}. Reinstall ${pkg.name} with optional dependencies enabled (npm install --include=optional ${pkg.name}).`, { cause });
  }
  const installed = require(manifest);
  if (installed.version !== pkg.version) throw new Error(`Monoprint native package version mismatch: ${name}@${installed.version}, expected ${pkg.version}`);
  return join(dirname(manifest), file);
};
