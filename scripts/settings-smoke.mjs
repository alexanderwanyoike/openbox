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
const capture = async name => { x('mousemove', '--window', window, '700', '140'); await delay(700); execFileSync('import', ['-window', window, join(screenshots, `${name}.png`)]); };
async function click(px, py) { x('mousemove', '--window', window, String(px), String(py)); await delay(120); x('click', '1'); await delay(500); }
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
  await waitFor(() => { window = x('search', '--onlyvisible', '--pid', String(app.pid), '--name', '^Openbox$').split('\n')[0]; return Boolean(window); }, 'Window did not open');
  x('windowfocus', window);
  await delay(400);
}
async function stop() { app.kill('SIGTERM'); await new Promise(resolve => app.once('exit', resolve)); }
const preferences = () => JSON.parse(readFileSync(settings));
try {
  await start();
  await click(400, 187);
  await click(430, 24);
  await click(550, 89);
  await click(1390, 89);
  await delay(1200);
  await click(195, 153);
  await click(1107, 153);
  await capture('rulers-classic');
  x('mousemove', '--window', window, '1000', '650'); await delay(120);
  x('click', '--repeat', '8', '--delay', '40', '5'); await delay(1500);
  await click(400, 600);
  await click(550, 89);
  await delay(500);
  await capture('settings-library-scrolled');
  await click(1410, 24);
  x('key', 'Escape'); await delay(800);
  await click(400, 600);
  await click(550, 89);
  await delay(500);
  await capture('settings-library-returned');
  const rows = name => execFileSync('convert', [join(screenshots, `${name}.png`), '-crop', '240x18+102+73', 'gray:-']);
  const retained = rows('settings-library-returned').equals(rows('settings-library-scrolled'));
  await click(1410, 24);
  await capture('settings-initial');
  assert.ok(retained, 'Returning from Settings changed the track at the same visible row');
  if (!process.env.CAPTURE_ONLY) {
    for (const [mode, x] of [['dark', 470], ['light', 350]]) {
      await click(x, 266);
      await waitFor(() => preferences().mode === mode, `Mode ${mode} did not apply`);
    }
    for (const [n, material] of ['native', 'aluminium', 'ceramic', 'slate', 'graphite', 'paper', 'copper', 'carbon'].entries()) {
      await click(353 + (n % 4) * 123, 379 + Math.floor(n / 4) * 96);
      await waitFor(() => preferences().material === material, `Material ${material} did not apply`);
    }
    for (const [n, accent] of ['blue', 'cobalt', 'emerald', 'vermilion', 'violet', 'amber'].entries()) {
      await click(333 + n * 82, 600);
      await waitFor(() => preferences().accent === accent, `Accent ${accent} did not apply`);
    }
    for (const [n, waveform] of ['classic', 'spectrum', 'ice', 'ember', 'acid', 'violet', 'nord', 'dracula', 'mono', 'paper'].entries()) {
      await click(924 + (n % 2) * 247, 255 + Math.floor(n / 2) * 92);
      await waitFor(() => preferences().waveform_theme === waveform, `Waveform ${waveform} did not apply`);
    }
    await click(353, 379);
    await click(333, 600);
    await capture('settings-light');
    x('key', 'Escape'); await delay(800);
    await capture('rulers-paper');
    const pixel = execFileSync('convert', [join(screenshots, 'rulers-paper.png'), '-format', '%[pixel:p{500,800}]', 'info:'], { encoding: 'utf8' });
    assert.ok(pixel.match(/\d+/g)?.map(Number).slice(0, 3).every(n => n > 200), `Library did not recolor: ${pixel}`);
    await stop();
    await start();
    assert.deepEqual(preferences(), { mode: 'light', material: 'native', accent: 'blue', waveform_theme: 'paper' });
    await click(1410, 24);
    await capture('settings-restored');
    await click(1195, 134);
    await waitFor(() => preferences().mode === 'dark' && preferences().material === 'native' && preferences().accent === 'blue' && preferences().waveform_theme === 'classic', 'Defaults did not restore');
    await capture('settings-defaults');
    assert.ok(readFileSync(music).equals(source));
    assert.ok(readFileSync(library).equals(originalLibrary));
    console.log('Native settings smoke passed: light/dark modes, eight materials, six accents, ten waveforms, retained browser row, restart persistence, defaults, unchanged library and audio.');
  }
} finally { if (app?.exitCode === null) await stop(); }
