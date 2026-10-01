// Yield search through tasks without microtask starvation or nested-timer throttling.

export function createScheduler() {
  if (typeof MessageChannel === 'undefined') {
    return {schedule: callback => setTimeout(callback, 0), cancel: clearTimeout, close() {}};
  }
  const channel = new MessageChannel();
  const callbacks = new Map();
  let sequence = 0;
  channel.port1.onmessage = ({data}) => {
    const callback = callbacks.get(data);
    callbacks.delete(data);
    callback?.();
  };
  return {
    schedule(callback) {
      sequence = (sequence + 1) >>> 0;
      callbacks.set(sequence, callback);
      channel.port2.postMessage(sequence);
      return sequence;
    },
    cancel(ticket) { callbacks.delete(ticket); },
    close() { callbacks.clear(); channel.port1.close(); channel.port2.close(); },
  };
}
