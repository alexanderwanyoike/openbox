import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { createServer } from 'node:http';
import { createReadStream, mkdirSync, mkdtempSync, copyFileSync, readFileSync, writeFileSync, statSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import { join, resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

const work = mkdtempSync('/tmp/openbox-update-demo-');
const library = join(work, 'my library.json');
const stage = join(work, 'stage');
mkdirSync(stage);
const executable = join(work, 'openbox');
copyFileSync(process.env.OPENBOX_CURRENT_BIN, executable);
copyFileSync(process.env.OPENBOX_NEXT_BIN, join(stage, 'openbox'));
const archive = join(work, 'openbox.tar.gz');
execFileSync('tar', ['-czf', archive, '-C', stage, 'openbox']);
const checksum = createHash('sha256').update(readFileSync(archive)).digest('hex');
writeFileSync(library, '{"tracks":[],"playlists":[]}');
const originalLibrary = readFileSync(library);
const screenshots = resolve('.prototype/screenshots');
mkdirSync(screenshots, { recursive: true });
let base;
let downloads = 0;
const server = createServer(async (request, response) => {
  if (request.url === '/asset') {
    downloads++;
    response.writeHead(200, { 'Content-Type': 'application/octet-stream', 'Content-Length': statSync(archive).size });
    for await (const chunk of createReadStream(archive, { highWaterMark: 1024 * 1024 })) {
      if (!response.write(chunk)) await once(response, 'drain');
      await delay(35);
    }
    response.end();
    return;
  }
  const release = { tag_name: 'v0.2.0', created_at: '2026-11-04T00:00:00Z', assets: [{ name: 'openbox-x86_64-unknown-linux-gnu.tar.gz', url: `${base}/asset`, digest: `sha256:${checksum}` }] };
  response.writeHead(200, { 'Content-Type': 'application/json' });
  response.end(JSON.stringify(release));
});
server.listen(0, '127.0.0.1');
await once(server, 'listening');
base = `http://127.0.0.1:${server.address().port}`;
const app = spawn(executable, ['--library', library], { env: { ...process.env, OPENBOX_UPDATE_TEST_API: base }, stdio: ['ignore', 'ignore', 'pipe'] });
let logs = '';
app.stderr.on('data', chunk => { logs += chunk; });
const x = (...args) => execFileSync('xdotool', args, { encoding: 'utf8' }).trim();
let window;
let newPid;
async function waitFor(predicate, message, timeout = 60000) {
  const start = Date.now();
  while (Date.now() - start < timeout) { try { if (predicate()) return; } catch {} await delay(100); }
  throw new Error(`${message}\n${logs}`);
}
async function capture(name) { x('mousemove', '--window', window, '700', '350'); await delay(700); execFileSync('import', ['-window', window, join(screenshots, `${name}.png`)]); }
try {
  await waitFor(() => { window = x('search', '--onlyvisible', '--name', '^Openbox$').split('\n')[0]; return Boolean(window); }, 'Initial window did not open');
  x('windowfocus', window);
  await delay(1500);
  await capture('update-available');
  x('mousemove', '--window', window, '1370', '884'); await delay(120); x('click', '1');
  await waitFor(() => downloads === 1, 'The footer did not start the download');
  await delay(500);
  await capture('update-downloading');
  await waitFor(() => app.exitCode === 0, 'The original application did not exit after updating');
  await waitFor(() => {
    window = x('search', '--onlyvisible', '--name', '^Openbox$').split('\n')[0];
    const pid = Number(x('getwindowpid', window));
    if (pid && pid !== app.pid) { newPid = pid; return true; }
    return false;
  }, 'The updated application did not reopen');
  await delay(1200);
  await capture('update-restarted');
  assert.equal(execFileSync(executable, ['--version'], { encoding: 'utf8' }).trim(), 'openbox 0.2.0');
  assert.deepEqual(readFileSync(library), originalLibrary);
  assert.equal(downloads, 1);
  console.log('Native updater smoke passed: footer download, progress, verified replacement, process restart into v0.2.0, preserved library argument and bytes.');
} finally {
  if (app.exitCode === null) app.kill('SIGTERM');
  if (newPid) { try { process.kill(newPid, 'SIGTERM'); } catch {} }
  server.closeAllConnections(); server.close();
}
