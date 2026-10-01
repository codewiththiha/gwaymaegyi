// Load browser bindings, bound startup requests, and expose the worker protocol.

import init, {Engine, capabilities_json} from './gwaymaegyi_wasm.js';
import {createRuntime} from './worker-runtime.mjs';

let pending = [];
let runtime = null;
let failure = null;
self.onmessage = ({data}) => {
  if (runtime) runtime.handle(data);
  else if (failure) self.postMessage({id: data?.id ?? null, type: 'error', error: failure});
  else if (pending.length < 128) pending.push(data);
  else self.postMessage({id: data?.id ?? null, type: 'error', error: 'startup queue is full'});
};
init().then(() => {
  runtime = createRuntime(() => new Engine(), message => self.postMessage(message));
  self.postMessage({type: 'ready', capabilities: JSON.parse(capabilities_json())});
  for (const message of pending) runtime.handle(message);
  pending = [];
}).catch(error => {
  failure = String(error);
  self.postMessage({type: 'init-error', error: failure});
  for (const message of pending) self.postMessage({id: message?.id ?? null, type: 'error', error: failure});
  pending = [];
});
