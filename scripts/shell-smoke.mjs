import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

const work = mkdtempSync('/tmp/openbox-shell-');
const library = join(work, 'Collection.json');
const screenshots = resolve('.prototype/screenshots');
mkdirSync(screenshots, { recursive: true });
const music = join(work, 'rhythm.wav');
execFileSync('ffmpeg', ['-v', 'error', '-f', 'lavfi', '-i', String.raw`aevalsrc=0.6*exp(-12*mod(t\,0.5))*sin(2*PI*80*t)+0.12*sin(2*PI*900*t)+0.07*sin(2*PI*6000*t):s=44100:d=32`, '-ac', '2', music]);
const tracks = Array.from({ length: 501 }, (_, n) => ({ id: n + 1, path: music, title: n ? `Track ${String(n + 1).padStart(3, '0')}` : 'Rhythm preview', artist: 'Generated audio fixture', details: { bpm: 120, key: '8A', duration_seconds: 32, analysis: { version: 'fixture', beats: Array.from({ length: 64 }, (_, n) => ({ time_seconds: n * 0.5, bpm: 120, number: n % 4 + 1 })) } } }));
writeFileSync(library, JSON.stringify({ tracks, folders: [{ id: 10, name: 'Sets', parent: null, sort_order: 0 }, { id: 11, name: 'House', parent: 10, sort_order: 0 }], playlists: [{ id: 1, name: 'Warm up', track_ids: [1, 2, 3], parent: 11 }, { id: 2, name: 'Preview', track_ids: tracks.map(t => t.id), sort_order: 1 }] }));
const source = readFileSync(music);
const app = spawn(resolve(process.env.OPENBOX_BIN || 'target/debug/openbox'), ['--library', library], { stdio: ['ignore', 'ignore', 'pipe'] });
let logs = '';
app.stderr.on('data', data => { logs += data; });
const x = (...args) => execFileSync('xdotool', args, { encoding: 'utf8' }).trim();
let window;
async function click(px, py) { x('mousemove', '--window', window, String(px), String(py)); await delay(120); x('click', '1'); await delay(450); }
async function capture(name, keepPointer = false) { if (!keepPointer) x('mousemove', '--window', window, '750', '75'); await delay(500); execFileSync('import', ['-window', window, join(screenshots, `${name}.png`)]); }
try {
  for (let n = 0; n < 100 && !window; n++) {
    if (app.exitCode !== null) throw new Error(logs);
    try { window = x('search', '--onlyvisible', '--pid', String(app.pid), '--name', '^Openbox$').split('\n')[0]; } catch {}
    await delay(100);
  }
  assert.ok(window, `No native window: ${logs}`);
  x('windowraise', window);
  x('windowfocus', window);
  await delay(700);
  await capture('shell-initial');
  const motif = execFileSync('xprop', ['-id', window, '-f', '_MOTIF_WM_HINTS', '32c', '_MOTIF_WM_HINTS'], { encoding: 'utf8' });
  assert.match(motif, /= 2, 0, 0,/, 'Custom chrome must replace the system title bar');
  await click(20, 229);
  await click(80, 258);
  await click(65, 799);
  x('type', '--clearmodifiers', 'Afterhours'); await delay(350);
  await click(155, 799);
  const created = JSON.parse(readFileSync(library)).playlists.find(p => p.name === 'Afterhours');
  assert.equal(created?.parent, 11, 'Creation must use the selected nested folder');
  await click(80, 165);
  await click(117, 24);
  await capture('shell-file-menu', true);
  const region = name => execFileSync('convert', [join(screenshots, `${name}.png`), '-crop', '220x100+105+40', 'rgba:-']);
  assert.ok(!region('shell-file-menu').equals(region('shell-initial')), 'File menu did not open');
  await click(700, 75);
  await click(192, 24);
  await capture('shell-view-menu', true);
  await click(245, 60);
  await delay(800);
  await capture('shell-players');
  await click(400, 620);
  await click(550, 134);
  await click(1390, 134);
  await delay(1200);
  await capture('shell-row-selection');
  await click(1260, 72);
  await delay(800);
  await capture('shell-dark');
  await click(1308, 24);
  await capture('shell-settings');
  await click(350, 266);
  await capture('shell-settings-light');
  x('key', 'Escape'); await delay(500);
  await capture('shell-light');
  assert.ok(readFileSync(music).equals(source));
  console.log('Native shell smoke passed: integrated chrome, File/View menus, deck toolbar, selection and Settings; source audio unchanged.');
} finally {
  if (app.exitCode === null) { app.kill('SIGTERM'); await new Promise(resolve => app.once('exit', resolve)); }
}
