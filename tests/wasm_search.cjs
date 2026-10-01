// Compare actual WASM evaluation/search with native fixtures and transaction guarantees.

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const wasm = require(path.resolve(process.argv[2]));
const {pathToFileURL} = require('node:url');

(async () => {
  const {copyReport} = await import(pathToFileURL(path.resolve('web/worker-runtime.mjs')));
  assert.equal(typeof wasm.Engine, "function");
  assert.equal(typeof wasm.EngineReport, "function");
  const capabilities = JSON.parse(wasm.capabilities_json());
  assert.deepEqual(capabilities.modes, ['balanced', 'aggressive', 'human-like', 'analysis']);
  assert.equal(capabilities.eloCalibrated, false);
  if (process.argv[3]) assert.equal(capabilities.simd128, process.argv[3] === 'simd128');
  const nominal = [500,800,1000,1200,1300,1400,1500,1600,1700,1800,1900,2000,2100,2200,2300,2400,2500,2650,2800,3000];
  assert.deepEqual(capabilities.skillLevels, nominal.map((elo,index) => ({level:index+1,elo})).concat({level:21,elo:0}));
  assert.equal(capabilities.liveLimits, true);
  const controls=JSON.parse(wasm.search_controls_json());
  assert.deepEqual(controls.behaviors, ['aspiration','null-move','reverse-futility','quiet-pruning','late-reductions','razoring','internal-reductions','exchange-pruning','history-pruning','probcut','singular-extensions']);
  assert.equal(controls.parameters.length,31);
    assert.ok(controls.parameters.some((item) => item.name === 'SEDepth'));
    assert.ok(controls.parameters.some((item) => item.name === 'LMRBase'));
  let checks = 0;
  for (const line of fs.readFileSync('crates/eval/tests/scores.txt', 'utf8').trim().split('\n')) {
    const fields = line.trim().split(/\s+/);
    const fen = fields.slice(0, 6).join(' ');
    ['balanced', 'endgame', 'aggressive'].forEach((model, index) => {
      ['white', 'black'].forEach((side, color) => {
        assert.equal(wasm.raw_evaluate(fen, model, side), Number(fields[6 + index * 2 + color]));
        checks++;
      });
    });
  }
  const finish = (engine, quantum) => {
    let report;
    for (let count = 0; count < 100000; count++) {
      report = copyReport(engine.step(quantum));
      if (report.finished) return report;
    }
    throw new Error('nonterminating search');
  };
  const cases = JSON.parse(fs.readFileSync('fixtures/search.json', 'utf8'));
  for (const scenario of cases) {
    const engine = new wasm.Engine();
    try {
      engine.configure(scenario.mode, scenario.elo, 8, 1, scenario.chess960, '19');
      engine.set_position(scenario.fen, []);
      engine.start_moves(scenario.depth, scenario.nodes, scenario.roots).free();
      const actual = finish(engine, 128);
      assert.deepEqual(actual, scenario.report, JSON.stringify(scenario));
      let fen = scenario.fen;
      for (const move of actual.pv) fen = wasm.play_uci(fen, move, scenario.chess960);
      assert.ok(fen);
    } finally { engine.free(); }
  }
  const first = new wasm.Engine();
  const second = new wasm.Engine();
  try {
    first.start(3, '20000').free(); second.start(3, '20000').free();
    assert.deepEqual(finish(first, 1), finish(second, 1024));
    first.reset(); second.reset();
    first.start(2, '20000').free(); second.start(3, '20000').free();
    first.step(25).free(); second.step(25).free();
    const progress = copyReport(first.report());
    assert.throws(() => first.set_limits(0, '20'));
    assert.deepEqual(copyReport(first.report()), progress);
    assert.deepEqual(copyReport(first.set_limits(3, '20000')), progress);
    assert.deepEqual(finish(first, 128), finish(second, 128));
    for (let index=0;index<nominal.length;index++) {
      first.set_skill_level(index+1);
      assert.equal(first.elo, nominal[index]); assert.equal(first.skill_level,index+1);
    }
    first.set_skill_level(21); assert.equal(first.elo,0);
    first.set_elo(1350); assert.equal(first.skill_level,undefined);
    first.set_skill_level(1); first.start_full().free();
    assert.deepEqual(JSON.parse(first.limits_json()), {
      requested:{depth:64,nodes:'18446744073709551615'},effective:{depth:1,nodes:'256'}
    });
    first.stop().free(); first.set_skill_level(21);
    first.set_behavior('null-move',false);
    first.set_parameter('AspStartWindow',30);
    assert.equal(JSON.parse(first.tuning_json()).parameters.AspStartWindow,30);
    assert.equal(JSON.parse(first.tuning_json()).behaviors['null-move'],false);
    assert.throws(()=>first.set_parameter('NMPDepthDiv',0));
    assert.throws(()=>first.set_behavior('unknown',false));
    first.set_parameter('AspStartWindow',20); first.set_behavior('null-move',true);
    first.reset(); first.start(64, '1000000').free(); first.step(17).free();
    const before = copyReport(first.report());
    for (const bad of [0, -1, 0.5, NaN, Infinity, 65537, 4294967296]) assert.throws(() => first.step(bad));
    for (const bad of [0, 22, -1, 1.5, NaN, Infinity, 256]) assert.throws(() => first.set_skill_level(bad));
    for (const bad of [499, 3001, -1, 500.5, NaN, Infinity, 65536]) assert.throws(() => first.set_elo(bad));
    assert.throws(() => first.start(0, '20'));
    assert.throws(() => first.start(2, '18446744073709551616'));
    assert.throws(() => first.set_position('bad FEN', []));
    assert.deepEqual(copyReport(first.report()), before);
    const stopped = copyReport(first.stop());
    assert.equal(stopped.status, 'stopped');
    assert.ok(first.legal_moves.includes(stopped.bestMove));
    first.reset(); assert.equal(first.searching, false);
    assert.equal(copyReport(first.report()).status, 'idle');
    first.set_position('7k/6Q1/6K1/8/8/8/8/8 b - - 150 1', []);
    const terminal = copyReport(first.start(4, '1000'));
    assert.equal(terminal.bestMove, null); assert.equal(terminal.scoreCp, -30000);
    assert.equal(first.outcome, 'checkmate:white');
    assert.throws(() => first.play_uci('h8h7'), /game is already over/);
  } finally { first.free(); second.free(); }
  console.log(`WASM search: ${checks} raw scores, ${cases.length} native snapshots, quantum invariance, rollback, stop, terminal checks passed (${capabilities.simd128 ? 'simd128' : 'portable'})`);
})().catch(error => { console.error(error); process.exitCode = 1; });
