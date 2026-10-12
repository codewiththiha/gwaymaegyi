// Test real WASM worker responsiveness, replacement, rollback, and stale-result suppression.

const assert = require('node:assert/strict');
const {Worker} = require('node:worker_threads');
const path = require('node:path');
const worker = new Worker(path.resolve('tests/worker_harness.cjs'), {workerData: path.resolve(process.argv[2])});
const messages = [];
const waiters = new Set();
worker.on('message', message => {
  messages.push(message);
  for (const waiter of [...waiters]) if (waiter.match(message)) waiter.resolve(message);
});
worker.on('error', error => { for (const waiter of [...waiters]) waiter.reject(error); });
const wait = match => {
  const prior = messages.find(match);
  if (prior) return Promise.resolve(prior);
  return new Promise((resolve, reject) => {
  const timer = setTimeout(() => { waiters.delete(waiter); reject(new Error('worker timeout: ' + JSON.stringify(messages.slice(-8)))); }, 10000);
  const waiter = {match, resolve: value => { clearTimeout(timer); waiters.delete(waiter); resolve(value); },
    reject: error => { clearTimeout(timer); waiters.delete(waiter); reject(error); }};
  waiters.add(waiter);
  });
};
const send = message => worker.postMessage(message);
(async () => {
  await wait(message => message.type === 'ready');
  send({id: 'one', type: 'start', depth: 64, nodes: '18446744073709551615', quantum: 16});
  const progress = await wait(message => message.id === 'one' && message.type === 'progress');
  assert.ok(BigInt(progress.report.nodes) > 0n);
  send({id:'tune-one',type:'performance',target:'one',options:{quantum:32,reportIntervalMs:150,nodes:'100000000',timeMs:5000}});
  const tuned = await wait(message => message.id === 'tune-one' && message.type === 'ack');
  assert.equal(tuned.report.status,'running');
  assert.ok(BigInt(tuned.report.nodes)>=BigInt(progress.report.nodes));
  assert.equal(tuned.performance.quantum,32);
  assert.equal(tuned.performance.timeMs,5000);
  assert.equal(tuned.limits.effective.nodes,'100000000');
  send({id:'bad-tune',type:'performance',target:'one',options:{quantum:0,nodes:'1'}});
  await wait(message => message.id === 'bad-tune' && message.type === 'error');
  send({id: 'stop-one', type: 'stop', target: 'one'});
  const done = await wait(message => message.id === 'one' && message.type === 'done');
  assert.equal(done.report.status, 'stopped');
  assert.ok(done.report.bestMove);

  send({id: 'two', type: 'start', depth: 64, nodes: '18446744073709551615', quantum: 16});
  await wait(message => message.id === 'two' && message.type === 'progress');
  send({id: 'reset', type: 'reset'});
  const reset = await wait(message => message.id === 'reset' && message.type === 'ack');
  assert.equal(reset.state.outcome, 'ongoing');
  const boundary = messages.length;
  send({id: 'status', type: 'status'});
  const idle = await wait(message => message.id === 'status' && message.type === 'ack');
  assert.equal(idle.report.status, 'idle');
  assert.ok(!messages.slice(boundary).some(message => message.id === 'two'));

  send({id: 'three', type: 'start', depth: 64, nodes: '18446744073709551615', quantum: 16});
  await wait(message => message.id === 'three' && message.type === 'progress');
  send({id: 'bad-config', type: 'configure', options: {elo: 499}});
  await wait(message => message.id === 'bad-config' && message.type === 'error');
  send({id: 'still-searching', type: 'status'});
  const current = await wait(message => message.id === 'still-searching' && message.type === 'ack');
  assert.equal(current.report.status, 'running');
  assert.equal(current.state.elo, 0);
  send({id: 'wrong-stop', type: 'stop', target: 'two'});
  await wait(message => message.id === 'wrong-stop' && message.type === 'error');
  send({id: 'stop-three', type: 'stop', target: 'three'});
  await wait(message => message.id === 'stop-three' && message.type === 'ack');
  send({id: 'configure', type: 'configure', options: {mode: 'human-like', elo: 1800, seed: '18446744073709551615'}});
  const configured = await wait(message => message.id === 'configure' && message.type === 'ack');
  assert.equal(configured.state.mode, 'human-like');
  assert.equal(configured.state.elo, 1800);
  assert.equal(configured.state.seed, '18446744073709551615');
  send({id:'skill',type:'configure',options:{mode:'aggressive',skillLevel:10}});
  const skill = await wait(message => message.id === 'skill' && message.type === 'ack');
  assert.equal(skill.state.mode,'aggressive');assert.equal(skill.state.elo,1800);assert.equal(skill.state.skillLevel,10);
  send({id:'conflict',type:'configure',options:{skillLevel:5,elo:1300}});
  await wait(message => message.id === 'conflict' && message.type === 'error');
  send({id:'full-config',type:'configure',options:{skillLevel:21}});
  await wait(message => message.id === 'full-config' && message.type === 'ack');
  send({id:'full-default',type:'start',quantum:16});
  await wait(message => message.id === 'full-default' && message.type === 'progress');
  send({id:'full-status',type:'status'});
  const full = await wait(message => message.id === 'full-status' && message.type === 'ack');
  assert.equal(full.state.fullStrength,true);
  assert.deepEqual(full.state.limits.requested,{depth:64,nodes:'18446744073709551615'});
  send({id:'finish-full',type:'stop',target:'full-default'});
  await wait(message => message.id === 'finish-full' && message.type === 'ack');
  send({id:'behavior-change',type:'behavior',name:'null-move',enabled:false});
  const behavior=await wait(message=>message.id==='behavior-change'&&message.type==='ack');
  assert.equal(behavior.tuning.behaviors['null-move'],false);
  send({id:'parameter-change',type:'parameter',name:'AspStartWindow',value:30});
  const parameter=await wait(message=>message.id==='parameter-change'&&message.type==='ack');
  assert.equal(parameter.tuning.parameters.AspStartWindow,30);
  send({id:'bad-parameter',type:'parameter',name:'NMPDepthDiv',value:0});
  await wait(message=>message.id==='bad-parameter'&&message.type==='error');
  send({id:'bounded-multipv',type:'configure',options:{multiPv:32}});
  const bounded = await wait(message=>message.id==='bounded-multipv'&&message.type==='ack');
  assert.equal(bounded.state.multiPv,32);
  send({id:'bad-native-multipv',type:'configure',options:{multiPv:255}});
  await wait(message=>message.id==='bad-native-multipv'&&message.type==='error');
  send({id:'bad-native-hash',type:'configure',options:{hashMiB:131072}});
  await wait(message=>message.id==='bad-native-hash'&&message.type==='error');
  send({id:'bounded-status',type:'status'});
  const boundedState = await wait(message=>message.id==='bounded-status'&&message.type==='ack');
  assert.equal(boundedState.state.multiPv,32);
  // Identical deeper-iteration reports must not repost at reportIntervalMs pace:
  // only depth advances, material decision changes, or a slow liveness heartbeat.
  const cadencePosts = [];
  send({id: 'cadence', type: 'start', depth: 64, nodes: '18446744073709551615', quantum: 64, reportIntervalMs: 100});
  const cadenceStartedAt = performance.now();
  let previous = null;
  await wait(message => {
    if (message.id !== 'cadence' || message.type !== 'progress') return false;
    const at = performance.now();
    cadencePosts.push(message);
    const report = message.report;
    if (previous) {
      const depthAdvanced = report.depth !== previous.report.depth;
      const decided = report.bestMove !== previous.report.bestMove
        || report.mate !== previous.report.mate
        || (report.scoreCp !== null && previous.report.scoreCp !== null
          && Math.abs(report.scoreCp - previous.report.scoreCp) >= 10);
      assert.ok(depthAdvanced || decided || at - previous.at >= 395,
        `silent-iteration repost under the liveness floor (gap ${(at - previous.at).toFixed(0)}ms)`);
    }
    previous = {report, at};
    return at - cadenceStartedAt >= 1100 && cadencePosts.length >= 3;
  });
  send({id: 'stop-cadence', type: 'stop', target: 'cadence'});
  const cadenceDone = await wait(message => message.id === 'cadence' && message.type === 'done');
  assert.equal(cadenceDone.report.status, 'stopped');
  send({id: 'dispose', type: 'dispose'});
  await wait(message => message.id === 'dispose' && message.type === 'ack');
  console.log('Actual WASM worker: progress, stop, reset, stale suppression, rollback, target ids, mode/strength control passed');
})().catch(error => { console.error(error); process.exitCode = 1; }).finally(() => worker.terminate());
