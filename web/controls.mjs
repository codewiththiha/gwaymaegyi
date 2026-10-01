// Pure worker input validation and compute presets, separate from playing strength.
// Limits are strings where JavaScript numbers would lose integer precision.

export const integer = (value, min, max, name) => {
  if (!Number.isSafeInteger(value) || value < min || value > max) {
    throw new Error(`${name} must be an integer from ${min} through ${max}`);
  }
  return value;
};
export const text = (value, max, name) => {
  if (typeof value !== 'string' || value.length > max) throw new Error(`invalid ${name}`);
  return value;
};
export const moves = value => {
  if (!Array.isArray(value) || value.length > 2048) throw new Error('invalid move list');
  return value.map(move => text(move, 5, 'move'));
};
export const decimal = value => {
  if (typeof value === 'number' && (!Number.isSafeInteger(value) || value < 0)) {
    throw new Error('use decimal strings for large integers');
  }
  if (!['string', 'number', 'bigint'].includes(typeof value)) throw new Error('invalid decimal integer');
  return text(String(value), 20, 'decimal integer');
};

const profiles = Object.freeze({
  full: {depth: 64, nodes: '18446744073709551615', quantum: 1024, reportIntervalMs: 100},
  balanced: {depth: 8, nodes: '100000', quantum: 256, reportIntervalMs: 50},
  responsive: {depth: 6, nodes: '50000', quantum: 64, reportIntervalMs: 100},
});
export const profile = name => {
  if (typeof name !== 'string' || !Object.hasOwn(profiles, name)) throw new Error('invalid performance profile');
  return profiles[name];
};
export function computeOptions(options, current = null) {
  if (!options || typeof options !== 'object' || Array.isArray(options)) throw new Error('invalid performance options');
  const preset = options.profile === undefined ? (current ?? profiles.full) : profile(options.profile);
  return {
    depth: integer(options.depth ?? preset.depth, 1, 64, 'depth'),
    nodes: decimal(options.nodes ?? preset.nodes),
    quantum: integer(options.quantum ?? preset.quantum, 1, 65536, 'quantum'),
    reportIntervalMs: integer(options.reportIntervalMs ?? preset.reportIntervalMs, 0, 5000, 'reportIntervalMs'),
    timeMs: options.timeMs === undefined ? undefined : options.timeMs === null ? null : integer(options.timeMs, 1, 86400000, 'timeMs'),
  };
}

export function configure(engine, options) {
  if (!options || typeof options !== 'object' || Array.isArray(options)) throw new Error('configure requires options');
  if (Object.hasOwn(options, 'elo') && Object.hasOwn(options, 'skillLevel')) throw new Error('choose elo or skillLevel, not both');
  const chess960 = options.chess960 ?? engine.chess960;
  if (typeof chess960 !== 'boolean') throw new Error('chess960 must be boolean');
  const args = [text(options.mode ?? engine.mode, 16, 'mode'),
    integer(options.skillLevel ?? options.elo ?? engine.elo, 0, 3000, 'strength'),
    integer(options.hashMiB ?? engine.hash_mib, 1, 64, 'hashMiB'),
    integer(options.multiPv ?? engine.multi_pv, 1, 32, 'multiPv'), chess960,
    decimal(options.seed ?? engine.seed)];
  if (Object.hasOwn(options, 'skillLevel')) engine.configure_skill(...args);
  else engine.configure(...args);
}
