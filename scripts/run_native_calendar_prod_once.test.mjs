import test from 'node:test';
import assert from 'node:assert/strict';
import {runOnce} from './run_native_calendar_prod_once.mjs';
import {CALENDAR_FEED_SOURCES} from './observe_native_calendar_prod.mjs';
const expected='calendar-day-ingestion-2026-09-07-1788764887683940812';
const env={CI_COMMIT_BRANCH:'main',CI_COMMIT_REF_PROTECTED:'true',CI_ENVIRONMENT_NAME:'production/smoke',FILFINANCE_CALENDAR_RUN_AUTHORIZED:'true',FILFINANCE_CALENDAR_RUN_DAY:'2026-09-07',FILFINANCE_CALENDAR_RUN_EXPECTED_ID:expected};
const before=()=>({localDay:'2026-09-07',latestRun:{runId:expected,status:'error'},hasActivePendingRun:false,items:[],artifacts:[{label:'calendar_native_source_audits',contentJson:JSON.stringify({sources:CALENDAR_FEED_SOURCES.map(source=>({source,outcome:'covered',detail:'covered',httpStatus:200,rawSha256:null,rawLengthBytes:null}))})}]});
const acquire=async({writeFileImpl})=>writeFileImpl('memory-only','fixture-token');
test('a retry after admission cannot POST twice',async()=>{
  let state=before(), posts=0;
  const fetchImpl=async(url,init)=>{
    assert.ok(url.startsWith('https://api.filfinance.fr/'));
    assert.equal(init.redirect,'error');
    if(init.method==='POST') {posts++;state={...state,latestRun:{runId:'calendar-day-ingestion-2026-09-07-999',status:'pending'},hasActivePendingRun:true};}
    return Response.json(state);
  };
  assert.equal((await runOnce(env,fetchImpl,acquire)).admitted,true);
  await assert.rejects(runOnce(env,fetchImpl,acquire),/latest_run_changed_no_retry/);
  assert.equal(posts,1);
});
test('DEV, missing authorization and invalid dates fail before authentication',async()=>{
  for(const override of [{CI_COMMIT_BRANCH:'dev'},{FILFINANCE_CALENDAR_RUN_AUTHORIZED:'false'},{FILFINANCE_CALENDAR_RUN_DAY:'2026-02-30'},{FILFINANCE_CALENDAR_RUN_DAY:'2026-09-06'}]) {
    await assert.rejects(runOnce({...env,...override},()=>assert.fail('network'),()=>assert.fail('auth')));
  }
});
test('quarantine and a pending run prevent admission',async()=>{
  for(const pending of [true,false]) {
    const state=before();
    if(pending) state.hasActivePendingRun=true;
    else {const audit=JSON.parse(state.artifacts[0].contentJson);audit.sources[0]={...audit.sources[0],outcome:'refused',detail:'forbidden',httpStatus:403};state.artifacts[0].contentJson=JSON.stringify(audit);}
    await assert.rejects(runOnce(env,async(_url,init)=>{assert.notEqual(init.method,'POST');return Response.json(state);},acquire));
  }
});
test('uncertain POST is never retried and remote error text is discarded',async()=>{
  let posts=0;
  await assert.rejects(runOnce(env,async(_url,init)=>{
    if(init.method==='POST') {posts++;throw new Error('private-remote-sentinel');}
    return Response.json(before());
  },acquire),error=>error.message==='admin_request_failed_no_retry');
  assert.equal(posts,1);
});
