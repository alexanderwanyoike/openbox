import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

const integrated = process.env.OPENBOX_LAYOUT === 'integrated';
const transportY = integrated ? 420 : 381;
const work = mkdtempSync('/tmp/openbox-sync-');
const library = join(work, 'library.json');
const screenshots = resolve('.prototype/screenshots');
mkdirSync(screenshots, { recursive: true });
const tracks = [[120, 660, 0.1], [126, 990, 0.23]].map(([bpm, frequency, offset], n) => {
  const rate = 44100;
  const frames = rate * 90;
  const pcm = Buffer.alloc(44 + frames * 4);
  pcm.write('RIFF'); pcm.writeUInt32LE(pcm.length - 8, 4); pcm.write('WAVEfmt ', 8);
  pcm.writeUInt32LE(16, 16); pcm.writeUInt16LE(1, 20); pcm.writeUInt16LE(2, 22);
  pcm.writeUInt32LE(rate, 24); pcm.writeUInt32LE(rate * 4, 28);
  pcm.writeUInt16LE(4, 32); pcm.writeUInt16LE(16, 34); pcm.write('data', 36); pcm.writeUInt32LE(frames * 4, 40);
  for (let frame = 0; frame < frames; frame++) {
    const time = frame / rate;
    const phase = ((time - offset) % (60 / bpm) + 60 / bpm) % (60 / bpm);
    const envelope = Math.exp(-phase * 20);
    const sample = Math.round(Math.sin(time * frequency * Math.PI * 2) * envelope * 14000);
    pcm.writeInt16LE(sample, 44 + frame * 4); pcm.writeInt16LE(sample, 46 + frame * 4);
  }
  const path = join(work, `deck-${n}.wav`);
  writeFileSync(path, pcm);
  return { id: n + 1, path, title: `Sync ${String.fromCharCode(65 + n)}: ${bpm} BPM`, artist: 'Generated audio fixture', details: { bpm: 120, key: '8A', duration_seconds: 90, analysis: { version: 'fixture', beats: Array.from({ length: 180 }, (_, i) => ({ time_seconds: offset + i * 60 / bpm, bpm, number: i % 4 + 1 })) } } };
});
writeFileSync(library, JSON.stringify({ tracks, playlists: [] }));
const original = readFileSync(library);
const sources = tracks.map(track => readFileSync(track.path));
const sink = `openbox_sync_${process.pid}`;
const moduleId = execFileSync('pactl', ['load-module', 'module-null-sink', `sink_name=${sink}`, 'rate=48000'], { encoding: 'utf8' }).trim();
const recorder = spawn('parec', ['--raw', '--format=float32le', '--rate=48000', '--channels=2', `--device=${sink}.monitor`]);
let samples = Buffer.alloc(0);
recorder.stdout.on('data', data => { samples = Buffer.concat([samples, data]).subarray(-48000 * 8 * 10); });
const app = spawn(resolve(process.env.OPENBOX_BIN || 'target/debug/openbox'), ['--library', library], { env: { ...process.env, PULSE_SINK: sink }, stdio: ['ignore', 'ignore', 'pipe'] });
let logs = '';
app.stderr.on('data', data => { logs += data; });
const x = (...args) => execFileSync('xdotool', args, { encoding: 'utf8' }).trim();
let window;
async function click(px, py) { x('mousemove', '--window', window, String(px), String(py)); await delay(120); x('click', '1'); await delay(500); }
async function capture(name) { x('mousemove', '--window', window, '700', '40'); await delay(500); execFileSync('import', ['-window', window, join(screenshots, `${name}.png`)]); }
function envelopes(frequency, pcm) {
  const size = 960;
  const result = [];
  for (let start = 0; start + size * 8 <= pcm.length; start += size * 8) {
    let real = 0, imaginary = 0;
    for (let frame = 0; frame < size; frame++) {
      const value = pcm.readFloatLE(start + frame * 8);
      const angle = 2 * Math.PI * frequency * frame / 48000;
      real += value * Math.cos(angle); imaginary += value * Math.sin(angle);
    }
    result.push(Math.hypot(real, imaginary) / size);
  }
  return result;
}
try {
  for (let n = 0; n < 100 && !window; n++) {
    if (app.exitCode !== null) throw new Error(logs);
    try { window = x('search', '--onlyvisible', '--pid', String(app.pid), '--name', '^Openbox$').split('\n')[0]; } catch {}
    await delay(100);
  }
  assert.ok(window, `No native window: ${logs}`);
  x('windowfocus', window);
  await click(400, integrated ? 234 : 187);
  await click(integrated ? 1260 : 430, integrated ? 72 : 24);
  await click(550, integrated ? 134 : 81);
  await delay(1000);
  await click(400, integrated ? 647 : 592);
  await click(1390, integrated ? 134 : 81);
  await delay(1000);
  await click(890, transportY);
  await delay(700);
  await click(45, transportY);
  await capture('sync-before');
  await click(integrated ? 302 : 290, transportY);
  await delay(2000);
  await capture('sync-active');
  const bpmGlyphs = left => execFileSync('convert', [join(screenshots, 'sync-active.png'),
    '-crop', `85x24+${left}+${integrated ? 115 : 60}`, '-channel', 'RGB', '-separate', '+channel',
    '-evaluate-sequence', 'max', '-threshold', '45%', '-trim', '+repage', '-depth', '8', 'gray:-']);
  const digitsA = bpmGlyphs(435), digitsB = bpmGlyphs(integrated ? 1280 : 1283);
  const difference = [...digitsA].filter((value, n) => value !== digitsB[n]).length / digitsA.length;
  assert.ok(digitsA.length === digitsB.length && difference < 0.04,
    `Synced decks show different BPM digits (${(difference * 100).toFixed(1)}% glyph mismatch)`);
  samples = Buffer.alloc(0);
  await delay(6000);
  const a = envelopes(660 * 126 / 120, samples);
  const b = envelopes(990, samples);
  assert.ok(a.length > 200, 'No recorded output');
  const mean = values => values.reduce((sum, v) => sum + v, 0) / values.length;
  const ma = mean(a), mb = mean(b);
  let covariance = 0, va = 0, vb = 0;
  for (let n = 0; n < a.length; n++) { covariance += (a[n] - ma) * (b[n] - mb); va += (a[n] - ma) ** 2; vb += (b[n] - mb) ** 2; }
  const correlation = covariance / Math.sqrt(va * vb);
  console.log(`Recorded beat-envelope correlation: ${correlation.toFixed(3)}`);
  assert.ok(ma > 0.002 && mb > 0.002, 'Sync A must match the already-playing B deck, including its audible pitch');
  assert.ok(correlation > 0.9, 'Recorded beat pulses are not aligned');
  await click(450, integrated ? 452 : 412);
  await capture('sync-manual-pitch');
  assert.ok(readFileSync(library).equals(original), 'Sync changed library bytes');
  tracks.forEach((track, n) => assert.ok(readFileSync(track.path).equals(sources[n]), 'Sync changed audio bytes'));
  console.log('Native sync smoke passed: two loaded analyzed tracks, automatic master from the first playing deck, aligned recorded beat envelopes and manual pitch, unchanged library and audio.');
} finally {
  app.kill('SIGTERM'); recorder.kill('SIGTERM');
  execFileSync('pactl', ['unload-module', moduleId]);
}
