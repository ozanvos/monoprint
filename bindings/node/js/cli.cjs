#!/usr/bin/env node
'use strict';
const { spawnSync } = require('node:child_process');
const { resolveRuntime } = require('./runtime.cjs');

try {
  const result = spawnSync(resolveRuntime('cli'), process.argv.slice(2), { stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.signal) process.kill(process.pid, result.signal);
  else process.exitCode = result.status ?? 1;
} catch (error) {
  console.error(`monoprint: ${error.message}`);
  process.exitCode = 1;
}
