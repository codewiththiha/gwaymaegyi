const {parentPort, workerData} = require('node:worker_threads');
const {pathToFileURL} = require('node:url');
const path = require('node:path');
(async () => {
  const wasm = require(workerData);
  const {createRuntime} = await import(pathToFileURL(path.resolve('web/worker-runtime.mjs')));
  const runtime = createRuntime(() => new wasm.Engine(), message => parentPort.postMessage(message));
  parentPort.on('message', message => runtime.handle(message));
  parentPort.postMessage({type: 'ready'});
})().catch(error => { parentPort.postMessage({type: 'init-error', error: String(error)}); });
