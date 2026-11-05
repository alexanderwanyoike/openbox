import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

assert.ok(process.env.DISPLAY, 'Run in an isolated Xvfb display');
const work = mkdtempSync('/tmp/openbox-clock-smoke-');
const music = join(work, 'pulse.wav');
const library = join(work, 'library.json');
execFileSync('ffmpeg', ['-v', 'error', '-f', 'lavfi', '-i', 'sine=frequency=660:duration=4', '-ac', '2', music]);
const track = { id: 1, path: music, title: 'Clock fixture', artist: 'Generated audio', details: { bpm: 120, duration_seconds: 4, analysis: { version: 'fixture', beats: Array.from({ length: 8 }, (_, n) => ({ time_seconds: n * 0.5, bpm: 120, number: n % 4 + 1 })) } } };
writeFileSync(library, JSON.stringify({ tracks: [track, { ...track, id: 2 }], playlists: [] }));
const original = readFileSync(library);
const originalMusic = readFileSync(music);
const sink = `openbox_clock_${process.pid}`;
const moduleId = execFileSync('pactl', ['load-module', 'module-null-sink', `sink_name=${sink}`, 'rate=48000'], { encoding: 'utf8' }).trim();
const app = spawn(resolve(process.env.OPENBOX_BIN || 'target/debug/openbox'), ['--library', library], { env: { ...process.env, PULSE_SINK: sink, ICED_BACKEND: 'tiny-skia' }, stdio: ['ignore', 'ignore', 'pipe'] });
let logs = '', window;
app.stderr.on('data', data => { logs += data; });
const x = (...args) => execFileSync('xdotool', args, { encoding: 'utf8' }).trim();
async function click(px, py, wait = 400) { x('mousemove', '--window', window, String(px), String(py)); x('click', '1'); await delay(wait); }
try {
  for (let n = 0; n < 100 && !window; n++) {
    if (app.exitCode !== null) throw new Error(logs);
    try { window = x('search', '--onlyvisible', '--pid', String(app.pid), '--name', '^Openbox$').split('\n')[0]; } catch {}
    await delay(100);
  }
  assert.ok(window, logs);
  x('windowfocus', window);
  await click(400, 234);
  await click(1260, 72);
  await click(550, 134);
  await delay(700);
  await click(400, 647);
  await click(1390, 134);
  await delay(700);
  execFileSync('pactl', ['suspend-sink', sink, '1']);
  await delay(1600);
  await click(1148, 420);
  await delay(3500);
  await click(890, 420, 120);
  x('mousemove', '--window', window, '700', '40');
  const screenshot = resolve(process.env.OPENBOX_CLOCK_SCREENSHOT || join(work, 'clock-start.png'));
  execFileSync('import', ['-window', window, screenshot]);
  const row = execFileSync('convert', [screenshot, '-crop', '558x1+864+188', '-depth', '8', 'rgb:-']);
  const heads = [];
  for (let n = 0; n < 558; n++) {
    if (row[n * 3] > 185 && row[n * 3 + 1] > 185 && row[n * 3 + 2] > 185) heads.push(n);
  }
  console.log(JSON.stringify({ screenshot, heads }));
  assert.ok(heads.length > 0 && Math.max(...heads) > 5, 'Deck B did not start advancing, so the clock check is inconclusive');
  assert.ok(Math.max(...heads) < 200, 'Starting B after a paused master jumped its waveform toward the end');
  assert.ok(readFileSync(library).equals(original), 'Playback changed library bytes');
  assert.ok(readFileSync(music).equals(originalMusic), 'Playback changed source audio bytes');
} finally {
  if (app.exitCode === null) { app.kill('SIGTERM'); await new Promise(resolve => app.once('exit', resolve)); }
  execFileSync('pactl', ['unload-module', moduleId]);
}
