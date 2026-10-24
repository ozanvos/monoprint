'use strict';
const { resolveRuntime } = require('./runtime.cjs');

exports.loadBinding = function loadBinding() {
  return require(resolveRuntime('binding'));
};
