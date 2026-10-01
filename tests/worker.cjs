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
  send({id: 'dispose', type: 'dispose'});
  await wait(message => message.id === 'dispose' && message.type === 'ack');
  console.log('Actual WASM worker: progress, stop, reset, stale suppression, rollback, target ids, mode/strength control passed');
})().catch(error => { console.error(error); process.exitCode = 1; }).finally(() => worker.terminate());
