'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');
const wasm = require(path.resolve(process.argv[2] || 'pkg/node/gwaymaegyi_wasm.js'));
const start = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';

assert.equal(wasm.normalize_fen(start), start);
assert.equal(wasm.legal_moves(start, false).split(' ').length, 20);
assert.equal(wasm.perft(start, 0), '1');
assert.equal(wasm.perft(start, 3), '8902');
assert.equal(wasm.play_uci(start, 'e2e4', false),
  'rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1');
assert.equal(wasm.play_uci('4k3/8/8/8/8/8/8/R4KR1 w GA - 0 1', 'f1g1', true),
  '4k3/8/8/8/8/8/8/R4RK1 b - - 1 1');
assert.throws(() => wasm.normalize_fen('bad FEN'));
assert.throws(() => wasm.play_uci(start, 'e2e5', false));
for (const depth of [-1, 0.5, 7, 256, NaN, Infinity]) {
  assert.throws(() => wasm.perft(start, depth));
}
assert.equal(wasm.perft(start, -0), '1');
console.log('WASM runtime checks passed');
