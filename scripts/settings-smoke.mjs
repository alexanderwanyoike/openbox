import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

const work = mkdtempSync(resolve('.prototype/settings-'));
const library = join(work, 'library.json');
const settings = join(work, 'library.settings.json');
const music = join(work, 'rhythm.wav');
const screenshots = resolve('.prototype/screenshots');
mkdirSync(screenshots, { recursive: true });
const rhythm = String.raw`aevalsrc=0.6*exp(-12*mod(t\,0.5))*sin(2*PI*80*t)+0.12*sin(2*PI*900*t)+0.07*sin(2*PI*6000*t):s=44100:d=32`;
execFileSync('ffmpeg', ['-v', 'error', '-f', 'lavfi', '-i', rhythm, '-ac', '2', music]);
const seed = { tracks: [{ id: 1, path: music, title: 'Rhythm preview', artist: 'Generated audio fixture', details: { bpm: 120, key: '8A', duration_seconds: 32, analysis: { version: 'fixture', beats: Array.from({ length: 64 }, (_, n) => ({ time_seconds: n * 0.5, bpm: 120, number: n % 4 + 1 })) } } }], playlists: [{ id: 1, name: 'Preview', track_ids: [1] }] };
for (let n = 2; n <= 501; n++) seed.tracks.push({ ...seed.tracks[0], id: n, title: `Browser row ${String(n).padStart(4, '0')}` });
writeFileSync(library, JSON.stringify(seed));
const source = readFileSync(music);
const originalLibrary = readFileSync(library);
const binary = resolve(process.env.OPENBOX_BIN || 'target/debug/openbox');
let app;
let logs = '';
let window;
const x = (...args) => execFileSync('xdotool', args, { encoding: 'utf8' }).trim();
const capture = name => { x('mousemove', '--window', window, '700', '140'); execFileSync('import', ['-window', window, join(screenshots, `${name}.png`)]); };
async function click(px, py) { x('mousemove', '--window', window, String(px), String(py)); x('click', '1'); await delay(200); }
async function waitFor(predicate, message) {
  for (let n = 0; n < 100; n++) {
    if (app.exitCode !== null) throw new Error(`App exited: ${logs}`);
    try { if (predicate()) return; } catch {}
    await delay(100);
  }
  throw new Error(`${message}\n${logs}`);
}
async function start() {
  window = undefined;
  app = spawn(binary, ['--library', library], { stdio: ['ignore', 'ignore', 'pipe'] });
  app.stderr.on('data', data => { logs += data; });
  await waitFor(() => { window = x('search', '--onlyvisible', '--name', '^Openbox$').split('\n')[0]; return Boolean(window); }, 'Window did not open');
  x('windowfocus', window);
  await delay(400);
}
async function stop() { app.kill('SIGTERM'); await new Promise(resolve => app.once('exit', resolve)); }
const preferences = () => JSON.parse(readFileSync(settings));
try {
  await start();
  await click(400, 187);
  await click(430, 24);
  await click(550, 81);
  await click(1390, 81);
  await delay(1200);
  await click(195, 145);
  await click(1107, 145);
  capture('rulers-classic');
  x('mousemove', '--window', window, '1000', '650');
  x('click', '--repeat', '8', '--delay', '40', '5'); await delay(300);
  capture('settings-library-scrolled');
  await click(1410, 24);
  x('key', 'Escape'); await delay(300);
  capture('settings-library-returned');
  const rows = name => execFileSync('convert', [join(screenshots, `${name}.png`), '-crop', '900x180+260+570', 'gray:-']);
  assert.ok(rows('settings-library-returned').equals(rows('settings-library-scrolled')), 'Returning from Settings lost the scrolled library viewport');
  await click(1410, 24);
  capture('settings-initial');
  if (!process.env.CAPTURE_ONLY) {
    const uiThemes = ['openbox', 'obsidian', 'berlin', 'acid', 'vaporwave', 'nord', 'dracula', 'gruvbox', 'tokyo_night', 'daylight'];
    const waveThemes = ['classic', 'spectrum', 'ice', 'ember', 'acid', 'violet', 'nord', 'dracula', 'mono', 'paper'];
    for (let n = 0; n < uiThemes.length; n++) {
      await click(350, 273 + 53 * n);
      await waitFor(() => existsSync(settings) && preferences().ui_theme === uiThemes[n], `UI theme ${uiThemes[n]} did not apply`);
    }
    for (let n = 0; n < waveThemes.length; n++) {
      await click(1100, 276 + 59 * n);
      await waitFor(() => preferences().waveform_theme === waveThemes[n], `Waveform theme ${waveThemes[n]} did not apply`);
    }
    capture('settings-daylight');
    x('key', 'Escape'); await delay(300);
    capture('rulers-paper');
    const pixel = execFileSync('convert', [join(screenshots, 'rulers-paper.png'), '-format', '%[pixel:p{500,800}]', 'info:'], { encoding: 'utf8' });
    assert.ok(pixel.match(/\d+/g)?.map(Number).slice(0, 3).every(n => n > 225), `Library did not recolor: ${pixel}`);
    await stop();
    await start();
    assert.deepEqual(preferences(), { ui_theme: 'daylight', waveform_theme: 'paper' });
    await click(1410, 24);
    capture('settings-restored');
    await click(1310, 105);
    await waitFor(() => preferences().ui_theme === 'openbox' && preferences().waveform_theme === 'classic', 'Defaults did not restore');
    capture('settings-defaults');
    assert.deepEqual(readFileSync(music), source);
    assert.deepEqual(readFileSync(library), originalLibrary);
    console.log('Native settings smoke passed: all ten UI and waveform choices, visible light theme, restart persistence, defaults, unchanged library and audio.');
  }
} finally { if (app?.exitCode === null) await stop(); }
