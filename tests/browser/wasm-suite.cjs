const suite = process.env.WASM_CI_SUITE || 'full';
if (!['pr', 'full'].includes(suite)) throw new Error(`Unknown WASM_CI_SUITE: ${suite}`);
module.exports = {full: suite === 'full'};
