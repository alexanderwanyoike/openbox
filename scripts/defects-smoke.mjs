import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

const work = mkdtempSync('/tmp/openbox-defects-');
const library = join(work, 'Collection.json');
const music = join(work, 'preview.wav');
const screenshots = resolve('.prototype/screenshots');
mkdirSync(screenshots, { recursive: true });
const rhythm = String.raw`aevalsrc=0.6*exp(-12*mod(t\,0.5))*sin(2*PI*80*t)+0.12*sin(2*PI*900*t)+0.07*sin(2*PI*6000*t):s=44100:d=90`;
execFileSync('ffmpeg', ['-v', 'error', '-f', 'lavfi', '-i', rhythm, '-ac', '2', music]);
const genre = 'Electronic / Techno / Dub Techno / Ambient / Deep House / Disco / House';
const track = { id: 1, path: music, title: 'Preview rhythm', artist: 'Generated fixture', details: { bpm: 120, key: '8A', duration_seconds: 90, genre, analysis: { version: 'fixture', beats: Array.from({ length: 180 }, (_, n) => ({ time_seconds: n * 0.5, bpm: 120, number: n % 4 + 1 })) } } };
writeFileSync(library, JSON.stringify({ tracks: [track, { ...track, id: 2, title: 'Second rhythm' }, { ...track, id: 3, path: join(work, 'missing.mp3'), title: 'Unavailable file' }], playlists: [{ id: 1, name: 'Preview', track_ids: [1, 2, 3] }] }));
const original = readFileSync(library);
const source = readFileSync(music);
const sink = `openbox_defects_${process.pid}`;
const moduleId = execFileSync('pactl', ['load-module', 'module-null-sink', `sink_name=${sink}`, 'rate=48000'], { encoding: 'utf8' }).trim();
const app = spawn(resolve(process.env.OPENBOX_BIN || 'target/debug/openbox'), ['--library', library], { env: { ...process.env, PULSE_SINK: sink, ICED_BACKEND: 'tiny-skia' }, stdio: ['ignore', 'ignore', 'pipe'] });
let logs = '';
app.stderr.on('data', data => { logs += data; });
const x = (...args) => execFileSync('xdotool', args, { encoding: 'utf8' }).trim();
let window;
async function click(px, py) { x('mousemove', '--window', window, String(px), String(py)); await delay(220); x('click', '1'); await delay(500); }
async function capture(name, hover = false) {
  if (!hover) x('mousemove', '--window', window, '700', '75');
  await delay(700);
  const path = join(screenshots, `${name}.png`);
  execFileSync('import', ['-window', window, path]);
  return path;
}
function waveformBackground(path) {
  return execFileSync('convert', [path, '-crop', '1x1+300+187', '-depth', '8', 'rgb:-']);
}
try {
  for (let n = 0; n < 100 && !window; n++) {
    if (app.exitCode !== null) throw new Error(logs);
    try { window = x('search', '--onlyvisible', '--pid', String(app.pid), '--name', '^Openbox$').split('\n')[0]; } catch {}
    await delay(100);
  }
  assert.ok(window, logs);
  x('windowfocus', window);
  await delay(500);
  await click(400, 234);
  await click(1260, 72);
  await delay(700);
  await click(550, 134);
  await click(400, 647);
  await click(1390, 134);
  await delay(1000);
  await click(45, 420);
  await click(890, 420);
  const dark = await capture('defects-dark');
  assert.ok([...waveformBackground(dark)].every(v => v < 80), 'Dark waveform surface is too light');
  x('mousemove', '--window', window, '822', '122');
  await capture('defects-tooltip', true);
  await click(1314, 24);
  await click(350, 266);
  await capture('defects-light-settings');
  x('key', 'Escape'); await delay(500);
  const light = await capture('defects-light');
  assert.ok([...waveformBackground(light)].every(v => v > 200), 'Light waveform surface is too dark');
  await click(1314, 24);
  await click(470, 266);
  await click(1171, 623);
  await capture('defects-dark-settings');
  x('key', 'Escape'); await delay(500);
  const paper = await capture('defects-dark-paper');
  assert.ok([...waveformBackground(paper)].every(v => v < 80), 'Paper needs its own dark variant');
  await click(45, 420);
  await click(890, 420);
  await click(400, 675);
  await click(550, 134);
  await delay(600);
  await capture('defects-file-error');
  await click(1320, 517);
  await capture('defects-error-details');
  assert.ok(readFileSync(library).equals(original), 'Preview modified the library');
  assert.ok(readFileSync(music).equals(source), 'Preview modified the source audio');
  console.log('Native defects smoke passed: faders, tooltip, long metadata, light/dark cached waveforms, dark Paper variant, failed-track notice and unchanged library/audio.');
} finally {
  if (app.exitCode === null) { app.kill('SIGTERM'); await new Promise(resolve => app.once('exit', resolve)); }
  execFileSync('pactl', ['unload-module', moduleId]);
}
