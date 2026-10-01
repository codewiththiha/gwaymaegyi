// Measure warmed WASM searches with fresh engines; timings are deployment diagnostics.

const {performance}=require('node:perf_hooks');
const fs=require('node:fs');
const w=require(process.argv[2]);
// Three warmups precede eight fresh-engine timing samples.
const times=[];const nodes=[];
for(let trial=0;trial<11;trial++) {
 const e=new w.Engine();
 const begin=performance.now();
 e.start(6,'100000').free();
 let result;
 while(e.searching){result=e.step(1024);if(result.finished) nodes.push(Number(result.nodes));result.free();}
 const elapsed=performance.now()-begin;
 if(trial>2)times.push(elapsed);
 e.free();
}
times.sort((a,b)=>a-b);
console.log(JSON.stringify({node:process.version,medianMs:(times[3]+times[4])/2,nodes:nodes[0],minMs:times[0],maxMs:times.at(-1),wasmBytes:fs.statSync(process.argv[2].replace('.js','_bg.wasm')).size}));
