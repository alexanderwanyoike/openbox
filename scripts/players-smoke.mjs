import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

const work = mkdtempSync(resolve('.prototype/players-'));
const usb = join(work, 'usb');
const library = join(work, 'library.json');
const screenshots = resolve('.prototype/screenshots');
mkdirSync(usb);
mkdirSync(screenshots, { recursive: true });
const tracks = [
  { id: 1, path: join(work, 'deck-a.wav'), title: 'Preview A: 440 to 660 Hz', artist: 'Generated audio test', details: { album: 'Preview test', genre: 'Test', bpm: 120, key: '8A', duration_seconds: 90 } },
  { id: 2, path: join(work, 'deck-b.wav'), title: 'Preview B: 880 Hz', artist: 'Generated audio test', details: { album: 'Preview test', genre: 'Test', bpm: 122, key: '9A', duration_seconds: 90 } },
];
for (const track of tracks) {
  const rate = 44100;
  const frames = rate * 90;
  const wav = Buffer.alloc(44 + frames * 4);
  wav.write('RIFF'); wav.writeUInt32LE(wav.length - 8, 4); wav.write('WAVEfmt ', 8);
  wav.writeUInt32LE(16, 16); wav.writeUInt16LE(1, 20); wav.writeUInt16LE(2, 22);
  wav.writeUInt32LE(rate, 24); wav.writeUInt32LE(rate * 4, 28);
  wav.writeUInt16LE(4, 32); wav.writeUInt16LE(16, 34); wav.write('data', 36); wav.writeUInt32LE(frames * 4, 40);
  for (let frame = 0; frame < frames; frame++) {
    const seconds = frame / rate;
    const frequency = track.id === 2 ? 880 : seconds < 45 ? 440 : 660;
    const envelope = 0.7 + 0.3 * Math.cos(seconds * Math.PI * 4);
    const value = Math.round(Math.sin(seconds * frequency * Math.PI * 2) * 0.2 * envelope * 32767);
    wav.writeInt16LE(value, 44 + frame * 4); wav.writeInt16LE(value, 46 + frame * 4);
  }
  writeFileSync(track.path, wav);
}
const seed = { tracks, playlists: [{ id: 1, name: 'Preview mix', track_ids: [1, 2] }] };
writeFileSync(library, JSON.stringify(seed));
const sourceAudio = tracks.map(t => readFileSync(t.path));

const sink = `openbox_preview_${process.pid}`;
const moduleId = execFileSync('pactl', ['load-module', 'module-null-sink', `sink_name=${sink}`, 'rate=48000'], { encoding: 'utf8' }).trim();
const recording = spawn('parec', ['--raw', '--format=float32le', '--rate=48000', '--channels=2', '--latency-msec=30', `--device=${sink}.monitor`], { stdio: ['ignore', 'pipe', 'pipe'] });
let samples = Buffer.alloc(0);
let recordingLog = '';
recording.stderr.on('data', data => { recordingLog += data; });
recording.stdout.on('data', data => { samples = Buffer.concat([samples, data]).subarray(-48000 * 8 * 2); });
const app = spawn(resolve(process.env.OPENBOX_BIN || 'target/release/openbox'), ['--library', library, '--usb', usb], { env: { ...process.env, PULSE_SINK: sink }, stdio: ['ignore', 'pipe', 'pipe'] });
let logs = '';
app.stderr.on('data', data => { logs += data; });
const x = (...args) => execFileSync('xdotool', args, { encoding: 'utf8' }).trim();
let window;
const capture = name => execFileSync('import', ['-window', window, join(screenshots, `${name}.png`)]);
async function click(px, py) { x('mousemove', '--window', window, String(px), String(py)); await delay(100); x('click', '1'); await delay(500); }
async function drag(from, to, y) {
  x('mousemove', '--window', window, String(from), String(y)); await delay(100);
  x('mousedown', '1'); await delay(150);
  x('mousemove', '--window', window, String(to), String(y)); await delay(200);
  x('mouseup', '1'); await delay(500);
}
async function moveFader(px, from, to) {
  x('mousemove', '--window', window, String(px), String(from)); await delay(100);
  x('mousedown', '1'); await delay(100);
  x('mousemove', '--window', window, String(px), String(to)); await delay(150);
  x('mouseup', '1'); await delay(300);
}
async function resetFader(px) {
  x('mousemove', '--window', window, String(px), '335');
  x('keydown', 'ctrl'); x('click', '1'); x('keyup', 'ctrl'); await delay(300);
}
async function waitFor(predicate, message, timeout = 30000) {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    if (app.exitCode !== null) throw new Error(`App exited: ${logs}`);
    try { if (predicate()) return; } catch {}
    await delay(100);
  }
  throw new Error(`${message}\n${logs}\n${recordingLog}`);
}
async function signal() {
  samples = Buffer.alloc(0);
  await delay(800);
  assert.ok(samples.length > 48000 * 8 / 4, `Audio monitor returned no samples: ${recordingLog}`);
  const data = samples.subarray(-24000 * 8);
  const frames = Math.floor(data.length / 8);
  let square = 0;
  const tones = [440, 660, 880, 475.2].map(frequency => {
    let real = 0, imaginary = 0;
    for (let n = 0; n < frames; n++) {
      const sample = data.readFloatLE(n * 8);
      const phase = n * frequency * 2 * Math.PI / 48000;
      real += sample * Math.cos(phase); imaginary += sample * Math.sin(phase);
      if (frequency === 440) square += sample * sample;
    }
    return Math.hypot(real, imaginary) * 2 / frames;
  });
  return { a: tones[0], seek: tones[1], b: tones[2], pitch: tones[3], rms: Math.sqrt(square / frames) };
}

async function checkWaveformMotion() {
  const geometry = x('getwindowgeometry', '--shell', window);
  const left = Number(geometry.match(/^X=(\d+)/m)[1]);
  const top = Number(geometry.match(/^Y=(\d+)/m)[1]);
  const bytes = execFileSync('ffmpeg', ['-v', 'error', '-f', 'x11grab', '-draw_mouse', '0', '-framerate', '60',
    '-video_size', '1440x900', '-i', `${process.env.DISPLAY}+${left},${top}`, '-t', '3',
    '-vf', 'crop=240:100:330:265', '-pix_fmt', 'gray', '-f', 'rawvideo', 'pipe:1'], { maxBuffer: 10 * 1024 * 1024 });
  const size = 240 * 100;
  const frames = Math.floor(bytes.length / size);
  let changed = 0;
  for (let n = 1; n < frames; n++) {
    if (!bytes.subarray(n * size, (n + 1) * size).equals(bytes.subarray((n - 1) * size, n * size))) changed++;
  }
  const fps = 60 * changed / (frames - 1);
  console.log(`Waveform motion: ${fps.toFixed(1)} distinct frames/sec (${changed}/${frames - 1} captured transitions)`);
  assert.ok(fps >= 45, `Waveform movement updates at only ${fps.toFixed(1)} frames/sec`);
}

try {
  await waitFor(() => { window = x('search', '--onlyvisible', '--pid', String(app.pid), '--name', '^Openbox$').split('\n')[0]; return Boolean(window); }, 'Iced window did not open');
  x('windowfocus', window);
  await delay(500);
  await click(400, 234);
  await click(1260, 72);
  await click(550, 134);
  await delay(1200);
  capture('players-loaded-a');
  await click(400, 647);
  await click(1390, 134);
  await delay(1200);
  capture('players-loaded-b');
  await click(45, 436);
  await click(890, 436);
  let audio = await signal();
  assert.ok(audio.a > 0.02 && audio.b > 0.02, `Both decks were not audible: ${JSON.stringify(audio)}\n${logs}`);
  if (process.env.OPENBOX_CHECK_MOTION) await checkWaveformMotion();
  await moveFader(650, 303, 398);
  capture('players-channel-a-muted');
  audio = await signal();
  assert.ok(audio.a < 0.004 && audio.b > 0.02, `Channel A fader did not isolate A: ${JSON.stringify(audio)}`);
  await resetFader(650);
  audio = await signal();
  assert.ok(audio.a > 0.02 && audio.b > 0.02, `Channel A fader reset failed: ${JSON.stringify(audio)}`);
  await moveFader(773, 303, 398);
  audio = await signal();
  assert.ok(audio.b < 0.004 && audio.a > 0.02, `Channel B fader did not isolate B: ${JSON.stringify(audio)}`);
  await resetFader(773);
  await drag(621, 605, 459);
  audio = await signal();
  assert.ok(audio.a > 0.04 && audio.b < 0.004, `Crossfader A failed: ${JSON.stringify(audio)}`);
  await drag(819, 835, 459);
  audio = await signal();
  assert.ok(audio.b > 0.04 && audio.a < 0.004, `Crossfader B failed: ${JSON.stringify(audio)}`);
  await click(720, 459);
  await drag(646, 630, 429);
  capture('players-master-muted');
  audio = await signal();
  assert.ok(audio.rms < 0.0005, `Master mute failed: ${JSON.stringify(audio)}`);
  await click(822, 122);
  x('mousemove', '--window', window, '649', '166'); await delay(100);
  x('mousedown', '1'); await delay(150);
  x('mousemove', '--window', window, '649', '256'); await delay(150);
  x('mouseup', '1'); await delay(300);
  audio = await signal();
  capture('players-trim-muted');
  assert.ok(audio.a < 0.004 && audio.b > 0.02, `Channel A trim failed: ${JSON.stringify(audio)}`);
  x('mousemove', '--window', window, '649', '166'); await delay(100);
  x('click', '3'); await delay(300);
  audio = await signal();
  assert.ok(audio.a > 0.02 && audio.b > 0.02, `Trim knob reset failed: ${JSON.stringify(audio)}`);
  await click(822, 122);
  await drag(621, 605, 459);
  await drag(265, 510, 465);
  audio = await signal();
  assert.ok(audio.pitch > 0.04 && audio.a < 0.004, `Pitch did not change playback speed: ${JSON.stringify(audio)}`);
  await click(563, 463);
  await click(438, 198);
  audio = await signal();
  assert.ok(audio.seek > 0.04 && audio.a < 0.004, `Overview seeking did not reach the second part of the song: ${JSON.stringify(audio)}`);
  await drag(300, 230, 267);
  capture('players-waveform-seek');
  await click(45, 436);
  audio = await signal();
  assert.ok(audio.rms < 0.0005, `Pause did not silence deck A: ${JSON.stringify(audio)}`);
  await click(160, 198);
  await click(165, 436);
  await click(438, 198);
  await click(112, 436);
  await click(45, 436);
  audio = await signal();
  assert.ok(audio.a > 0.04 && audio.seek < 0.004, `Cue did not return to the saved position: ${JSON.stringify(audio)}`);
  await click(720, 459);
  await click(1260, 72);
  audio = await signal();
  assert.ok(audio.a > 0.02 && audio.b > 0.02, `Hiding the players interrupted playback: ${JSON.stringify(audio)}`);
  await click(1260, 72);
  capture('players-mixing');
  await click(45, 436);
  await click(890, 436);
  audio = await signal();
  assert.ok(audio.rms < 0.0005, `Paused decks still emitted audio: ${JSON.stringify(audio)}`);
  await click(75, 612);
  await click(1378, 72);
  const database = join(usb, 'PIONEER/rekordbox/export.pdb');
  for (let n = 0; n < 10 && !existsSync(database); n++) {
    capture(`players-export-${n}`);
    await delay(120);
  }
  await waitFor(() => existsSync(join(usb, '.openbox/device.json')), 'Export did not finish');
  await delay(400);
  capture('players-export-complete');
  assert.deepEqual(JSON.parse(readFileSync(library)), seed);
  tracks.forEach((track, i) => assert.deepEqual(readFileSync(track.path), sourceAudio[i]));
  writeFileSync(resolve('.prototype/players-last.json'), JSON.stringify({ work, library, usb, screenshots }, null, 2));
  console.log('Native player smoke passed: both decks audible, crossfader endpoints, independent channel faders, fader reset, trim/master volume, pitch, waveform seeking, cue, pause, hide/show persistence, export and unchanged source files.');
} finally {
  app.kill('SIGTERM');
  recording.kill('SIGTERM');
  execFileSync('pactl', ['unload-module', moduleId]);
}
