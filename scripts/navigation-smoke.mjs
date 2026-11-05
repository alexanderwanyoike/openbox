import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

const work = mkdtempSync('/tmp/openbox-navigation-');
const library = join(work, 'Collection.json');
const screenshots = resolve('.prototype/screenshots');
mkdirSync(screenshots, { recursive: true });
writeFileSync(library, JSON.stringify({ tracks: Array.from({ length: 100 }, (_, i) => ({ id: i + 1, path: join(work, `track-${i}.wav`), title: `Track ${String(i + 1).padStart(3, '0')}`, artist: 'Navigation fixture', details: { duration_seconds: 32, bpm: 120, key: '8A' } })), playlists: Array.from({ length: 100 }, (_, i) => ({ id: i + 1, name: `Playlist ${String(i + 1).padStart(3, '0')}`, track_ids: [], sort_order: i })) }));
const original = readFileSync(library);
const app = spawn(resolve(process.env.OPENBOX_BIN || 'target/debug/openbox'), ['--library', library], { env: { ...process.env, ICED_BACKEND: 'tiny-skia' }, stdio: ['ignore', 'ignore', 'pipe'] });
let logs = '';
app.stderr.on('data', data => { logs += data; });
const x = (...args) => execFileSync('xdotool', args, { encoding: 'utf8' }).trim();
let window;
async function capture(name) {
  x('mousemove', '--window', window, '700', '75');
  await delay(400);
  const path = join(screenshots, `${name}.png`);
  execFileSync('import', ['-window', window, path]);
  return path;
}
function crop(path, bounds) { return execFileSync('convert', [path, '-crop', bounds, '-depth', '8', 'rgb:-']); }
async function scroll(button, px = '100') {
  x('mousemove', '--window', window, px, '400');
  await delay(150);
  x('click', '--repeat', '14', '--delay', '35', button);
  await delay(500);
}
try {
  for (let n = 0; n < 100 && !window; n++) {
    if (app.exitCode !== null) throw new Error(logs);
    try { window = x('search', '--onlyvisible', '--pid', String(app.pid), '--name', '^Openbox$').split('\n')[0]; } catch {}
    await delay(100);
  }
  assert.ok(window, `No native window: ${logs}`);
  x('windowfocus', window);
  await delay(600);
  const before = await capture('navigation-no-scrollbar');
  const edge = crop(before, '2x500+215+120');
  for (const channel of edge) assert.equal(channel, 27, 'The collection edge must stay panel-coloured without a scrollbar');
  for (const channel of crop(before, '2x350+1426+250')) assert.ok(channel >= 27 && channel <= 30, 'The track list must not draw a scrollbar');
  await scroll('5');
  const after = await capture('navigation-scrolled');
  assert.ok(!crop(before, '175x500+25+150').equals(crop(after, '175x500+25+150')), 'Wheel scrolling must still move the playlist tree');
  await scroll('4');
  const restored = await capture('navigation-restored');
  assert.ok(crop(before, '175x500+25+150').equals(crop(restored, '175x500+25+150')), 'Scrolling up must return to the first playlists');
  await scroll('5', '400');
  const tracksAfter = await capture('navigation-tracks-scrolled');
  assert.ok(!crop(before, '350x350+320+240').equals(crop(tracksAfter, '350x350+320+240')), 'Track list wheel scrolling must still work');
  assert.ok(readFileSync(library).equals(original), 'Navigation must not change the library');
  console.log('Native navigation passed: no visible scrollbars with 100 playlists/tracks, tree scrolling down/up and track list scrolling, unchanged library.');
} finally {
  if (app.exitCode === null) { app.kill('SIGTERM'); await new Promise(resolve => app.once('exit', resolve)); }
}
