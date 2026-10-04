import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

const work = mkdtempSync(resolve('.prototype/analysis-'));
const library = join(work, 'library.json');
const usb = join(work, 'usb');
const music = join(work, 'tagged.mp3');
const cover = join(work, 'cover.ppm');
const screenshots = resolve('.prototype/screenshots');
mkdirSync(usb);
mkdirSync(screenshots, { recursive: true });
writeFileSync(cover, Buffer.concat([Buffer.from('P6\n2 2\n255\n'), Buffer.from([0, 160, 230, 0, 160, 230, 0, 160, 230, 0, 160, 230])]));
const jpeg = join(work, 'cover.jpg');
execFileSync('ffmpeg', ['-v', 'error', '-i', cover, jpeg]);
const rhythm = String.raw`aevalsrc=0.25*sin(2*PI*130.8128*t)+0.15*sin(2*PI*164.8138*t)+0.15*sin(2*PI*195.9977*t)+0.12*sin(2*PI*261.6256*t)+0.7*exp(-60*mod(t\,0.5))*sin(2*PI*70*t):s=22050:d=32`;
execFileSync('ffmpeg', ['-v', 'error', '-f', 'lavfi', '-i', rhythm, '-i', jpeg, '-map', '0:a', '-map', '1:v', '-c:a', 'libmp3lame', '-c:v', 'copy', '-disposition:v', 'attached_pic', '-id3v2_version', '3', '-metadata', 'title=Analyzed track', '-metadata', 'artist=Tagged artist', '-metadata', 'album=Artwork test', '-metadata', 'genre=House', music]);
const original = readFileSync(music);
writeFileSync(library, JSON.stringify({ tracks: [{ id: 1, path: music, title: 'Before analysis', artist: '', details: { album: '', genre: '', bpm: null, key: '', duration_seconds: 32 } }], playlists: [{ id: 1, name: 'Analyzed set', track_ids: [1] }] }));
const app = spawn(resolve(process.env.OPENBOX_BIN || 'target/release/openbox'), ['--library', library, '--usb', usb], { stdio: ['ignore', 'ignore', 'pipe'] });
let logs = '';
app.stderr.on('data', data => { logs += data; });
const x = (...args) => execFileSync('xdotool', args, { encoding: 'utf8' }).trim();
let window;
const capture = name => execFileSync('import', ['-window', window, join(screenshots, `${name}.png`)]);
const read = () => JSON.parse(readFileSync(library));
async function click(px, py) { x('mousemove', '--window', window, String(px), String(py)); await delay(100); x('click', '1'); await delay(300); }
async function waitFor(predicate, message) {
  const start = Date.now();
  while (Date.now() - start < 60000) {
    if (app.exitCode !== null) throw new Error(`App exited: ${logs}`);
    try { if (predicate()) return; } catch {}
    await delay(100);
  }
  throw new Error(`${message}\n${logs}`);
}

try {
  await waitFor(() => { window = x('search', '--onlyvisible', '--name', '^Openbox$').split('\n')[0]; return Boolean(window); }, 'Window did not open');
  x('windowfocus', window);
  await delay(500);
  await click(547, 24);
  capture('analysis-progress');
  await waitFor(() => read().tracks[0].details.analysis?.beats.length > 40, 'Library analysis did not finish');
  const local = read();
  assert.equal(local.tracks[0].title, 'Analyzed track');
  assert.equal(local.tracks[0].artist, 'Tagged artist');
  assert.equal(local.tracks[0].details.key, 'C');
  assert.ok(Math.abs(local.tracks[0].details.bpm - 120) < 1);
  assert.ok(existsSync(local.tracks[0].details.artwork));
  assert.deepEqual(local.playlists[0].track_ids, [1]);
  await click(400, 187);
  await click(430, 24);
  await click(550, 81);
  await delay(1500);
  capture('analysis-players');
  const pixel = execFileSync('convert', [join(screenshots, 'analysis-players.png'), '-format', '%[pixel:p{70,80}]', 'info:'], { encoding: 'utf8' });
  const channels = pixel.match(/\d+/g)?.map(Number);
  assert.ok(channels && channels[1] > 100 && channels[2] > 150, `Album cover did not appear: ${pixel}`);
  await click(74, 544);
  await click(1370, 24);
  capture('analysis-export');
  const manifest = join(usb, '.openbox/device.json');
  await waitFor(() => existsSync(manifest), 'Analyzed playlist did not export');
  const device = JSON.parse(readFileSync(manifest)).library;
  const exported = device.tracks[0];
  assert.ok(exported.details.analysis.beats.length > 40);
  assert.equal(exported.details.key, 'C');
  assert.ok(existsSync(exported.details.artwork));
  assert.ok(exported.details.artwork.startsWith(usb));
  assert.deepEqual(readFileSync(music), original);
  assert.deepEqual(read(), local);
  await delay(300);
  capture('analysis-complete');
  console.log('Native analysis smoke passed: bulk reanalysis, ID3 metadata, visible cover art, beat grid, USB analysis/artwork export, preserved membership and source files.');
} finally {
  app.kill('SIGTERM');
}
