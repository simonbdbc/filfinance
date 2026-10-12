#!/usr/bin/env node
// Observation post-deploiement uniquement : aucun POST calendrier/news, ni retry.
import {pathToFileURL} from 'node:url';

const API = 'https://api.filfinance.fr';
const MEDIA = 'https://media.filfinance.fr';
const MAX_BYTES = 1024 * 1024;
const DETAILS = new Set(['covered','timeout','transport','redirect','http_status','body_too_large','invalid_encoding','invalid_feed','missing_target_day','unauthorized','forbidden','rate_limited','legal_restriction','challenge']);
export const CALENDAR_FEED_SOURCES = ['forex_factory_export','bea_json','bea_ics','fed_fomc','fed_monetary_rss','fed_speeches_rss','eurostat_ics','ecb_calendar'];
const AUDIT_KEYS = ['source','outcome','detail','httpStatus','rawSha256','rawLengthBytes'];
function requireValue(condition, label) {
  if (!condition) throw new Error(label);
}

export function observationDay(now = new Date(), formatter = new Intl.DateTimeFormat('en-CA', {timeZone:'Europe/Paris',year:'numeric',month:'2-digit',day:'2-digit'})) {
  // Les images Alpine peuvent utiliser un autre ordre de presentation faute
  // de donnees ICU completes. Les composantes restent stables, pas format().
  const parts=Object.fromEntries(formatter.formatToParts(now).map(({type,value})=>[type,value]));
  const day=`${parts.year}-${parts.month.padStart(2,'0')}-${parts.day.padStart(2,'0')}`;
  const date = new Date(`${day}T12:00:00Z`);
  if (date.getUTCDay() === 6) date.setUTCDate(date.getUTCDate()+2);
  if (date.getUTCDay() === 0) date.setUTCDate(date.getUTCDate()+1);
  return date.toISOString().slice(0,10);
}

export function inspectAudits(snapshot) {
  requireValue(Array.isArray(snapshot?.artifacts) && Array.isArray(snapshot?.items), 'admin_shape');
  const audits = snapshot.artifacts.filter(a=>a.label === 'calendar_native_source_audits');
  const report = [];
  for (const artifact of audits) {
    let body;
    try {body=JSON.parse(artifact.contentJson);} catch {throw new Error('audit_json');}
    const expected = CALENDAR_FEED_SOURCES;
    requireValue(Array.isArray(body.sources) && body.sources.length === expected.length, 'audit_source_count');
    requireValue(new Set(body.sources.map(a=>a.source)).size === expected.length, 'audit_duplicate_source');
    for (const audit of body.sources) {
      requireValue(Object.keys(audit).length === AUDIT_KEYS.length && Object.keys(audit).every(k=>AUDIT_KEYS.includes(k)), 'audit_fields');
      requireValue(expected.includes(audit.source), 'audit_source');
      requireValue(['covered','unavailable','refused'].includes(audit.outcome) && DETAILS.has(audit.detail), 'audit_redacted_outcome');
      requireValue(audit.httpStatus === null || (Number.isInteger(audit.httpStatus) && audit.httpStatus>=100 && audit.httpStatus<=599), 'audit_http_status');
      requireValue(audit.rawSha256 === null || /^[a-f0-9]{64}$/.test(audit.rawSha256), 'audit_hash');
      requireValue(audit.rawLengthBytes === null || (Number.isSafeInteger(audit.rawLengthBytes) && audit.rawLengthBytes>=0), 'audit_size');
      report.push({source:audit.source,outcome:audit.outcome,httpStatus:audit.httpStatus});
    }
  }
  return report;
}

export async function boundedRead(response, limit = MAX_BYTES) {
  requireValue(response.ok, `http_${response.status}`);
  const reader = response.body.getReader();
  const chunks = [];
  let size = 0;
  try {
    while (true) {
      const {value,done} = await reader.read();
      if (done) break;
      size += value.byteLength;
      requireValue(size <= limit, 'response_size');
      chunks.push(value);
    }
  } finally {await reader.cancel();}
  return Buffer.concat(chunks);
}

export function mediaUrl(path, base = `${MEDIA}/`) {
  const url = new URL(path, base);
  requireValue(url.origin === MEDIA && !url.username && !url.password && url.pathname.startsWith('/audio/'), 'media_origin');
  return url;
}

export async function adminSnapshot(day, token, WebSocketImpl=WebSocket) {
  return new Promise((resolve,reject)=>{
    const socket = new WebSocketImpl(`wss://api.filfinance.fr/ws/admin/calendar-day?local_day=${day}`);
    let finished=false, hello=false, messages=0;
    const finish=(error,value)=>{
      if(finished) return;
      finished=true; clearTimeout(timer); socket.close();
      if(error) reject(new Error(error)); else resolve(value);
    };
    const timer=setTimeout(()=>finish('admin_ws_timeout'),10000);
    socket.addEventListener('open',()=>socket.send(JSON.stringify({type:'auth',access_token:token})));
    socket.addEventListener('message',event=>{
      try {
        requireValue(typeof event.data==='string' && Buffer.byteLength(event.data)<=MAX_BYTES && ++messages<=64,'admin_ws_size');
        const value=JSON.parse(event.data);
        if(value.type==='hello') hello=true;
        if(value.type==='snapshot') {
          requireValue(hello && value.payload?.localDay===day,'admin_ws_snapshot');
          finish(null,value.payload);
        }
      } catch {finish('admin_ws_response');}
    });
    socket.addEventListener('error',()=>finish('admin_ws_transport'));
    socket.addEventListener('close',()=>finish('admin_ws_closed'));
  });
}

export async function observe(environment=process.env) {
  requireValue(environment.FILFINANCE_API_BASE_URL===API,'prod_api_guard');
  const listener=environment.FILFINANCE_LISTENER_ACCESS_TOKEN;
  const admin=environment.FILFINANCE_ADMIN_ACCESS_TOKEN;
  requireValue(listener?.trim() && admin?.trim(),'missing_tokens');
  const get=async(path,token)=>{
    const response=await fetch(`${API}${path}`, {headers:{authorization:`Bearer ${token}`},redirect:'error',signal:AbortSignal.timeout(10000)});
    try {return JSON.parse((await boundedRead(response)).toString('utf8'));} catch {throw new Error('api_response');}
  };
  const day=observationDay();
  const calendar=await get(`/api/calendar-day?local_day=${day}`,listener);
  requireValue(calendar.localDay===day && Array.isArray(calendar.items),'calendar_response');
  const summary=await get(`/api/calendar-day/summary?local_day=${day}`,listener);
  requireValue(summary.localDay===day && typeof summary.status==='string','summary_response');
  const snapshot=await get(`/api/admin/calendar-day?local_day=${day}`,admin);
  requireValue(snapshot.localDay===day,'admin_day');
  const audits=inspectAudits(snapshot);
  const ws=await adminSnapshot(day,admin);
  const wsAudits=inspectAudits(ws);
  requireValue(wsAudits.length>=audits.length,'admin_ws_audit_missing');
  requireValue(snapshot.artifacts.filter(a=>a.label==='calendar_native_source_audits').every(a=>ws.artifacts.some(b=>b.id===a.id && b.contentJson===a.contentJson)),'admin_ws_audit_mismatch');
  console.log(JSON.stringify({check:'calendar_admin',day,items:calendar.items.length,summaryStatus:summary.status,audits,websocketAudits:wsAudits.length,collectionTriggered:false}));

  const replay=await get('/api/replay/today',listener);
  requireValue(replay.enabled===true,'hls_disabled');
  if (!replay.available) {
    console.log(JSON.stringify({check:'bunny_hls',status:'no_existing_replay',generated:false}));
    return;
  }
  const ticket=await get('/api/hls-ticket',listener);
  const playlist=mediaUrl(replay.playlist_url);
  if(ticket.enabled) {
    requireValue(typeof ticket.ticket==='string' && ticket.ticket.length>0,'hls_ticket');
    playlist.searchParams.set('hls_ticket',ticket.ticket);
  }
  const readMedia=async url=>boundedRead(await fetch(url,{redirect:'error',signal:AbortSignal.timeout(10000)}));
  const text=(await readMedia(playlist)).toString('utf8');
  requireValue(text.startsWith('#EXTM3U'),'hls_manifest');
  const segment=text.split(/\r?\n/).find(line=>line.trim() && !line.startsWith('#'));
  requireValue(segment,'hls_segment_missing');
  const segmentUrl=mediaUrl(segment,playlist);
  if(ticket.enabled) segmentUrl.searchParams.set('hls_ticket',ticket.ticket);
  const bytes=await readMedia(segmentUrl);
  requireValue(bytes.length>0,'hls_empty_segment');
  console.log(JSON.stringify({check:'bunny_hls',status:'passed',manifest:true,segmentBytes:bytes.length,generated:false}));
}

if (process.argv[1] && pathToFileURL(process.argv[1]).href===import.meta.url) {
  observe().catch(()=>{console.error('Native calendar PROD observation failed; response bodies and credentials withheld.');process.exitCode=1;});
}
