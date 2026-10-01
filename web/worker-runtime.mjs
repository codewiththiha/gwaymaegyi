// Validate worker requests and schedule retained search continuations.
// Job identities suppress stale callbacks after successful replacements.

import {createScheduler} from './scheduler.mjs';

import {configure, computeOptions, moves, text} from './controls.mjs';

export function copyReport(raw) {
  try {
    return {
      status: raw.status, finished: raw.finished, depth: raw.depth,
      selectiveDepth: raw.selective_depth, nodes: raw.nodes,
      bestMove: raw.best_move ?? null, scoreCp: raw.score_cp ?? null,
      mate: raw.mate ?? null, bestMoveNodes: raw.best_move_nodes,
      tablebaseHits: raw.tablebase_hits, pv: raw.pv, variations: JSON.parse(raw.variations_json),
    };
  } finally { raw.free(); }
}

function updateAdaptiveDeadline(engine, job) {
  if (job.timeMs === null || job.last.depth <= job.adaptiveDepth) return;
  const previous = job.scoreHistory.length
    ? Math.trunc(job.scoreHistory.reduce((sum, score) => sum + score, 0) / job.scoreHistory.length)
    : 0;
  const score = job.last.scoreCp ?? 0;
  const scoreDelta = previous - score;
  if (job.last.bestMove !== null) {
    job.stability = job.previousBest === job.last.bestMove ? job.stability + 1 : 1;
    job.previousBest = job.last.bestMove;
  }
  const softMs = Number(engine.soft_time_limit_ms(
    String(job.timeMs), String(job.timeMs), job.last.bestMoveNodes,
    job.last.nodes, job.stability, scoreDelta,
  ));
  job.deadline = Math.min(job.hardDeadline, job.startedAt + softMs);
  job.adaptiveDepth = job.last.depth;
  job.scoreHistory.push(score);
  if (job.scoreHistory.length > 3) job.scoreHistory.shift();
}

export function createRuntime(factory, post, hooks = {}) {
  const engine = factory();
  const scheduler = hooks.schedule ? {schedule: hooks.schedule, cancel: hooks.cancel ?? (() => {}), close() {}} : createScheduler();
  const {schedule, cancel} = scheduler;
  const now = hooks.now ?? (() => performance.now());
  let current = null;
  let disposed = false;

  const state = () => ({
    fen: engine.fen, outcome: engine.outcome, claims: engine.claims, legalMoves: engine.legal_moves,
    mode: engine.mode, elo: engine.elo, hashMiB: engine.hash_mib,
    multiPv: engine.multi_pv, chess960: engine.chess960, seed: engine.seed,
    skillLevel: engine.skill_level ?? null, limits: JSON.parse(engine.limits_json()),
    fullStrength: engine.elo === 0 || engine.mode === 'analysis',
  });
  const replace = () => {
    if (current) {
      cancel(current.timer);
      post({id: current.id, type: 'done', reason: 'replaced', report: current.last});
      current = null;
    }
  };
  const pump = job => {
    if (disposed || current !== job) return;
    try {
      if (job.deadline !== null && now() >= job.deadline) job.last = copyReport(engine.stop());
      else job.last = copyReport(engine.step(job.quantum));
      updateAdaptiveDeadline(engine, job);
      const finished = job.last.finished;
      if (finished || job.last.depth !== job.publishedDepth || now() - job.publishedAt >= job.reportIntervalMs) {
        post({id: job.id, type: finished ? 'done' : 'progress', report: job.last});
        job.publishedDepth = job.last.depth;
        job.publishedAt = now();
      }
      if (finished) current = null;
      else job.timer = schedule(() => pump(job));
    } catch (error) {
      try { engine.stop().free(); } catch { /* A trapped module may reject cleanup. */ }
      current = null;
      post({id: job.id, type: 'error', error: String(error)});
    }
  };

  const handle = message => {
    let id = null;
    try {
      if (!message || typeof message !== 'object') throw new Error('request must be an object');
      id = message.id;
      if (!(typeof id === 'string' && id.length > 0 && id.length <= 128)
          && !Number.isSafeInteger(id)) throw new Error('request requires a string or safe integer id');
      if (disposed) throw new Error('worker is disposed');
      switch (message.type) {
        case 'configure': {
          configure(engine, message.options);
          replace();
          post({id, type: 'ack', state: state()});
          break;
        }
        case 'behavior':
          if (typeof message.enabled !== 'boolean') throw new Error('behavior enabled must be boolean');
          engine.set_behavior(text(message.name,64,'behavior'),message.enabled);
          replace();post({id,type:'ack',tuning:JSON.parse(engine.tuning_json())});break;
        case 'parameter':
          if (!Number.isSafeInteger(message.value)) throw new Error('parameter value must be an integer');
          engine.set_parameter(text(message.name,64,'parameter'),message.value);
          replace();post({id,type:'ack',tuning:JSON.parse(engine.tuning_json())});break;
        case 'position':
          engine.set_position(text(message.fen, 256, 'FEN'), moves(message.moves ?? []));
          replace(); post({id, type: 'ack', state: state()}); break;
        case 'play':
          engine.play_uci(text(message.move, 5, 'move'));
          replace(); post({id, type: 'ack', state: state()}); break;
        case 'reset':
          engine.reset(); replace(); post({id, type: 'ack', state: state()}); break;
        case 'start': {
          if (current && current.id === id) throw new Error('active search ids must be unique');
          const compute = computeOptions(message);
          const roots = moves(message.roots ?? []);
          const initial = copyReport(engine.start_moves(compute.depth, compute.nodes, roots));
          replace();
          const startedAt = now();
          const timeMs = compute.timeMs ?? null;
          const hardDeadline = timeMs === null ? null : startedAt + timeMs;
          const job = {id, ...compute, timeMs, startedAt, hardDeadline, deadline: hardDeadline,
            last: initial, adaptiveDepth: initial.depth, previousBest: null,
            stability: 1, scoreHistory: [], publishedDepth: -1, publishedAt: now(), timer: null};
          current = job;
          job.timer = schedule(() => pump(job));
          break;
        }
        case 'performance': {
          if (!current) throw new Error('a running search is required to adjust performance');
          if (message.target !== undefined && current.id !== message.target) throw new Error('no matching active search');
          const job = current;
          const compute = computeOptions(message.options, job);
          const timeChanged = compute.timeMs !== undefined;
          const previousTimeMs = job.timeMs;
          const report = copyReport(engine.set_limits(compute.depth, compute.nodes));
          Object.assign(job, compute, {last: report});
          if (!timeChanged) {
            job.timeMs = previousTimeMs;
          } else {
            job.timeMs = compute.timeMs;
            job.startedAt = now();
            job.hardDeadline = job.timeMs === null ? null : job.startedAt + job.timeMs;
            job.deadline = job.hardDeadline;
            job.adaptiveDepth = report.depth;
            job.previousBest = report.bestMove;
            job.stability = 1;
            job.scoreHistory = [];
          }
          if (report.finished) {
            cancel(job.timer); current = null;
            post({id: job.id, type: 'done', report});
          }
          post({id, type: 'ack', report, performance: {
            depth: job.depth, nodes: job.nodes, quantum: job.quantum,
            reportIntervalMs: job.reportIntervalMs, timeMs: job.timeMs,
          }, limits: JSON.parse(engine.limits_json())});
          break;
        }
        case 'stop':
          if (message.target !== undefined && current?.id !== message.target) throw new Error('no matching active search');
          if (current) {
            const job = current; cancel(job.timer); current = null;
            job.last = copyReport(engine.stop());
            post({id: job.id, type: 'done', report: job.last});
          }
          post({id, type: 'ack', report: copyReport(engine.report())}); break;
        case 'status': post({id, type: 'ack', state: state(), report: copyReport(engine.report())}); break;
        case 'dispose':
          replace(); engine.free(); disposed = true; scheduler.close(); post({id, type: 'ack'}); break;
        default: throw new Error('unknown request type');
      }
    } catch (error) { post({id, type: 'error', error: String(error)}); }
  };
  return {handle, dispose() { if (!disposed) { replace(); engine.free(); disposed = true; scheduler.close(); } }};
}
