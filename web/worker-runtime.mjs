import {createScheduler} from './scheduler.mjs';

const integer = (value, min, max, name) => {
  if (!Number.isSafeInteger(value) || value < min || value > max) {
    throw new Error(`${name} must be an integer from ${min} through ${max}`);
  }
  return value;
};
const text = (value, max, name) => {
  if (typeof value !== 'string' || value.length > max) throw new Error(`invalid ${name}`);
  return value;
};
const moves = value => {
  if (!Array.isArray(value) || value.length > 2048) throw new Error('invalid move list');
  return value.map(move => text(move, 5, 'move'));
};
const decimal = value => {
  if (typeof value === 'number' && (!Number.isSafeInteger(value) || value < 0)) {
    throw new Error('use decimal strings for large integers');
  }
  if (!['string', 'number', 'bigint'].includes(typeof value)) throw new Error('invalid decimal integer');
  return text(String(value), 20, 'decimal integer');
};

export function copyReport(raw) {
  try {
    return {
      status: raw.status, finished: raw.finished, depth: raw.depth,
      selectiveDepth: raw.selective_depth, nodes: raw.nodes,
      bestMove: raw.best_move ?? null, scoreCp: raw.score_cp ?? null,
      mate: raw.mate ?? null, pv: raw.pv, variations: JSON.parse(raw.variations_json),
    };
  } finally { raw.free(); }
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
      const finished = job.last.finished;
      if (finished || job.last.depth !== job.publishedDepth || now() - job.publishedAt >= 50) {
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
          const cfg = message.options;
          if (!cfg || typeof cfg !== 'object') throw new Error('configure requires options');
          const chess960 = cfg.chess960 ?? engine.chess960;
          if (typeof chess960 !== 'boolean') throw new Error('chess960 must be boolean');
          engine.configure(text(cfg.mode ?? engine.mode, 16, 'mode'),
            integer(cfg.elo ?? engine.elo, 0, 3000, 'elo'),
            integer(cfg.hashMiB ?? engine.hash_mib, 1, 64, 'hashMiB'),
            integer(cfg.multiPv ?? engine.multi_pv, 1, 5, 'multiPv'),
            chess960, decimal(cfg.seed ?? engine.seed));
          replace();
          post({id, type: 'ack', state: state()});
          break;
        }
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
          const depth = integer(message.depth ?? 8, 1, 64, 'depth');
          const nodes = decimal(message.nodes ?? '100000');
          const quantum = integer(message.quantum ?? 256, 1, 65536, 'quantum');
          const time = message.timeMs === undefined ? null : integer(message.timeMs, 1, 86400000, 'timeMs');
          const roots = moves(message.roots ?? []);
          const initial = copyReport(engine.start_moves(depth, nodes, roots));
          replace();
          const job = {id, quantum, deadline: time === null ? null : now() + time,
            last: initial, publishedDepth: -1, publishedAt: now(), timer: null};
          current = job;
          job.timer = schedule(() => pump(job));
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
