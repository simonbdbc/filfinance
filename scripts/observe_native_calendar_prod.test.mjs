import test from 'node:test';
import assert from 'node:assert/strict';
import {CALENDAR_FEED_SOURCES,observationDay,inspectAudits,boundedRead,mediaUrl,observe,adminSnapshot} from './observe_native_calendar_prod.mjs';

const source=name=>({source:name,outcome:'refused',detail:'forbidden',httpStatus:403,rawSha256:null,rawLengthBytes:null});
const snapshot=sources=>({items:[],artifacts:[{label:'calendar_native_source_audits',contentJson:JSON.stringify({sources})}]});
test('all eight feed audits are required without admitting a legacy endpoint in a new run',()=>{
  const feeds=CALENDAR_FEED_SOURCES.map(source);
  assert.equal(inspectAudits(snapshot(feeds)).length,8);
  assert.throws(()=>inspectAudits(snapshot(feeds.slice(1))));
  assert.throws(()=>inspectAudits(snapshot([source('investing'),...feeds.slice(1)])));
  assert.equal(inspectAudits(snapshot(feeds.map(a=>({...a,outcome:'unavailable',detail:'invalid_feed'})))).length,8);
});
test('weekend observation reads Monday without triggering a run',()=>{
  assert.equal(observationDay(new Date('2026-09-06T08:00:00Z')),'2026-09-07');
  assert.equal(observationDay(new Date('2026-09-04T23:00:00Z')),'2026-09-07');
  assert.equal(observationDay(new Date('2026-09-08T08:00:00Z')),'2026-09-08');
});
test('ICU display order and separators cannot change the observation day',()=>{
  const formatter={formatToParts:()=>[{type:'month',value:'9'},{type:'literal',value:'/'},{type:'day',value:'6'},{type:'literal',value:'/'},{type:'year',value:'2026'}]};
  assert.equal(observationDay(new Date('2026-09-06T08:00:00Z'),formatter),'2026-09-07');
});
test('historical sources are rejected even when both were refused',()=>{
  assert.throws(()=>inspectAudits(snapshot([source('investing'),source('forex_factory')])));
});
test('raw content, unknown fields, duplicate or missing sources fail closed',()=>{
  for(const sources of [
    [source('investing')],
    [source('investing'),source('investing')],
    [{...source('investing'),html:'private'},source('forex_factory')],
    [{...source('investing'),detail:'private response text'},source('forex_factory')],
    [{...source('investing'),rawSha256:'not-a-hash'},source('forex_factory')],
  ]) assert.throws(()=>inspectAudits(snapshot(sources)));
});
test('response limit is enforced while streaming and body is cancelled',async()=>{
  let cancelled=false;
  const response=new Response(new ReadableStream({pull(controller){controller.enqueue(new Uint8Array(8));},cancel(){cancelled=true;}}));
  await assert.rejects(boundedRead(response,10),/response_size/);
  assert.equal(cancelled,true);
  assert.equal((await boundedRead(new Response('ok'),10)).toString(),'ok');
});
test('media tickets cannot be sent to another origin or non-audio path',()=>{
  assert.equal(mediaUrl('/audio/replay/today/playlist.m3u8').origin,'https://media.filfinance.fr');
  for(const url of ['https://other.invalid/audio/a.ts','http://media.filfinance.fr/audio/a.ts','https://user:pass@media.filfinance.fr/audio/a.ts','/api/admin']) assert.throws(()=>mediaUrl(url));
});
test('DEV and missing credentials fail before any network action',async()=>{
  await assert.rejects(observe({FILFINANCE_API_BASE_URL:'https://api.dev.filfinance.fr'}),/prod_api_guard/);
  await assert.rejects(observe({FILFINANCE_API_BASE_URL:'https://api.filfinance.fr'}),/missing_tokens/);
});
test('admin websocket authenticates, validates hello and closes after snapshot',async()=>{
  let socket;
  class FakeSocket extends EventTarget {
    constructor(url) {super();this.url=url;socket=this;queueMicrotask(()=>this.dispatchEvent(new Event('open')));}
    send(value) {
      assert.deepEqual(JSON.parse(value),{type:'auth',access_token:'fixture-only'});
      this.dispatchEvent(new MessageEvent('message',{data:JSON.stringify({type:'hello'})}));
      this.dispatchEvent(new MessageEvent('message',{data:JSON.stringify({type:'snapshot',payload:{localDay:'2026-09-07'}})}));
    }
    close() {this.closed=true;}
  }
  assert.deepEqual(await adminSnapshot('2026-09-07','fixture-only',FakeSocket),{localDay:'2026-09-07'});
  assert.equal(socket.closed,true);
  assert.equal(socket.url.includes('fixture-only'),false);
});
