const {createHash} = require('node:crypto');
const {readFileSync} = require('node:fs');
const {execFileSync} = require('node:child_process');

module.exports = function nativeReference(binary, {limit = 64 * 1024 * 1024} = {}) {
  const cache = new Map();
  let bytes = 0;
  const stats = {hits: 0, misses: 0};
  const run = (input, args = [], options = {}) => {
    const identity = createHash('sha256').update(readFileSync(binary)).digest('hex');
    const command = ['-', '-o', '-', ...args];
    const key = createHash('sha256').update(JSON.stringify([identity, command, options]))
      .update('\0').update(input).digest('hex');
    if (cache.has(key)) { stats.hits++; return cache.get(key); }
    stats.misses++;
    const result = execFileSync(binary, command, {input, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024, ...options});
    const size = Buffer.byteLength(result);
    if (size <= limit) {
      while (bytes + size > limit) {
        const oldest = cache.keys().next().value;
        bytes -= Buffer.byteLength(cache.get(oldest));
        cache.delete(oldest);
      }
      cache.set(key, result);
      bytes += size;
    }
    return result;
  };
  run.stats = stats;
  return run;
};
