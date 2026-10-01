// Detect SIMD128 support and choose a compatible packaged browser worker.

export function supportsSimd128() {
  const probe = new Uint8Array([0,97,115,109,1,0,0,0,1,4,1,96,0,0,3,2,1,0,10,9,1,7,0,65,0,253,15,26,11]);
  return typeof WebAssembly !== 'undefined' && WebAssembly.validate(probe);
}

export function createEngineWorker({portableUrl, simd128Url, preferSimd = true}) {
  const source = preferSimd && simd128Url && supportsSimd128() ? simd128Url : portableUrl;
  if (!source) throw new Error('a portable worker URL is required');
  return new Worker(source, {type: 'module'});
}
