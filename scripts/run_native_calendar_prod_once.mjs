import {pathToFileURL} from 'node:url';
import {acquireBetterAuthSmokeToken} from './acquire_better_auth_smoke_token.mjs';
import {boundedRead, inspectAudits} from './observe_native_calendar_prod.mjs';

const API = 'https://api.filfinance.fr';
function requireValue(condition, code) { if (!condition) throw new Error(code); }

// Le dernier run attendu constitue un verrou de relance : apres admission,
// un retry CI retrouve un autre run et s'arrete avant tout nouveau POST.
export async function runOnce(env=process.env, fetchImpl=fetch, acquire=acquireBetterAuthSmokeToken) {
  requireValue(env.CI_COMMIT_BRANCH==='main' && env.CI_COMMIT_REF_PROTECTED==='true' && env.CI_ENVIRONMENT_NAME==='production/smoke', 'prod_ci_guard');
  requireValue(env.FILFINANCE_CALENDAR_RUN_AUTHORIZED==='true', 'admission_not_authorized');
  const day=env.FILFINANCE_CALENDAR_RUN_DAY;
  const expected=env.FILFINANCE_CALENDAR_RUN_EXPECTED_ID;
  requireValue(/^\d{4}-\d{2}-\d{2}$/.test(day??''), 'invalid_day');
  const date=new Date(`${day}T12:00:00Z`);
  requireValue(!Number.isNaN(date.valueOf()) && date.toISOString().slice(0,10)===day && ![0,6].includes(date.getUTCDay()), 'invalid_day');
  requireValue(new RegExp(`^calendar-day-ingestion-${day}-[0-9]{1,25}$`).test(expected??''), 'invalid_expected_run');
  let token;
  try {
    await acquire({environment:{
      FILFINANCE_API_BASE_URL:API, FILFINANCE_PWA_URL:'https://app.filfinance.fr',
      FILFINANCE_SMOKE_EMAIL:env.FILFINANCE_SMOKE_ADMIN_EMAIL,
      FILFINANCE_SMOKE_PASSWORD:env.FILFINANCE_SMOKE_ADMIN_PASSWORD,
      FILFINANCE_SMOKE_TOKEN_FILE:'memory-only',
    }, fetchImpl, writeFileImpl:async(_path,value)=>{token=value;}, log:()=>{}});
  } catch { throw new Error('admin_auth_failed'); }
  requireValue(typeof token==='string' && token.length>0, 'admin_token_missing');
  async function request(path, init={}) {
    try {
      return JSON.parse((await boundedRead(await fetchImpl(`${API}${path}`, {
        ...init, headers:{authorization:`Bearer ${token}`, 'content-type':'application/json'},
        redirect:'error', signal:AbortSignal.timeout(10000),
      }))).toString('utf8'));
    } catch { throw new Error('admin_request_failed_no_retry'); }
  }
  const before=await request(`/api/admin/calendar-day?local_day=${day}`);
  requireValue(before.localDay===day && before.latestRun?.runId===expected, 'latest_run_changed_no_retry');
  requireValue(before.latestRun.status==='error' && before.hasActivePendingRun===false, 'run_not_terminal');
  const audits=inspectAudits(before);
  requireValue(audits.length===8 && !audits.some(a=>a.outcome==='refused'), 'source_quarantine_review_required');
  const after=await request('/api/admin/calendar-day/run', {method:'POST',body:JSON.stringify({localDay:day,force:true,reason:'ci_operator_single_admission'})});
  requireValue(after.localDay===day && after.latestRun?.runId!==expected && new RegExp(`^calendar-day-ingestion-${day}-[0-9]{1,25}$`).test(after.latestRun?.runId??''), 'admission_unconfirmed_no_retry');
  return {day,runId:after.latestRun.runId,admitted:true,postCount:1};
}

if(process.argv[1] && pathToFileURL(process.argv[1]).href===import.meta.url) {
  runOnce().then(value=>console.log(JSON.stringify(value))).catch(error=>{
    // Toutes les erreurs sont des codes locaux fermes, jamais un corps distant.
    console.error(error.message); process.exitCode=1;
  });
}
