import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { copyFileSync, existsSync, linkSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

const work = mkdtempSync(resolve('.prototype/smoke-'));
const library = join(work, 'library.json');
const usb = join(work, 'usb');
const screenshots = resolve('.prototype/screenshots');
mkdirSync(usb);
mkdirSync(screenshots, { recursive: true });
const seed = JSON.parse(readFileSync(resolve('.prototype/demo-seed.json')));
mkdirSync(join(work, 'music'));
for (let n = 0; n < 500; n++) {
  const path = join(work, 'music', `Crate track ${String(n + 1).padStart(3, '0')}.wav`);
  linkSync(seed.tracks[0].path, path);
  seed.tracks.push({ ...seed.tracks[0], id: n + 11, path, title: `Crate track ${String(n + 1).padStart(3, '0')}`, artist: 'Bulk selection test' });
}
seed.playlists.push({ id: 4, name: 'Bulk set', track_ids: [] });
writeFileSync(library, JSON.stringify(seed));
const database = join(usb, 'PIONEER/rekordbox/export.pdb');
mkdirSync(join(usb, 'PIONEER/rekordbox'), { recursive: true });
copyFileSync(resolve('tests/fixtures/rekordbox-demo.pdb'), database);
const originalDatabase = readFileSync(database);
const originalAudio = [];
for (const name of ['NOISE', 'SINEWAVE']) {
  const path = join(usb, `Contents/UnknownArtist/UnknownAlbum/${name}.wav`);
  mkdirSync(join(usb, 'Contents/UnknownArtist/UnknownAlbum'), { recursive: true });
  copyFileSync(resolve('.prototype/music/Glasshouse - After hours.wav'), path);
  originalAudio.push([path, readFileSync(path)]);
}
const originalAnalysis = join(usb, 'PIONEER/USBANLZ/original.DAT');
mkdirSync(join(usb, 'PIONEER/USBANLZ'), { recursive: true });
writeFileSync(originalAnalysis, 'Original cue data');

const readLibrary = () => JSON.parse(readFileSync(library));
const manifest = join(usb, '.openbox/device.json');
const readUsb = () => JSON.parse(readFileSync(manifest)).library;
const app = spawn(resolve(process.env.OPENBOX_BIN || 'target/release/openbox'), ['--library', library, '--usb', usb], { stdio: ['ignore', 'pipe', 'pipe'] });
let logs = '';
app.stderr.on('data', data => { logs += data; });

function x(...args) { return execFileSync('xdotool', args, { encoding: 'utf8' }).trim(); }
let window;
async function click(px, py) { x('mousemove', '--window', window, String(px), String(py)); await delay(100); x('click', '1'); await delay(400); }
async function type(value) { x('key', '--delay', '80', 'ctrl+a'); x('type', '--clearmodifiers', '--delay', '40', value); await delay(500); }
async function waitFor(predicate, message, timeout = 30000) {
  const started = Date.now();
  while (Date.now() - started < timeout) {
    if (app.exitCode !== null) throw new Error(`App exited: ${logs}`);
    try { if (predicate()) return; } catch {}
    await delay(100);
  }
  throw new Error(`${message}\n${logs}`);
}

try {
  await waitFor(() => {
    window = x('search', '--onlyvisible', '--name', '^Openbox$').split('\n')[0];
    return Boolean(window);
  }, 'Native Iced window did not open');
  x('windowfocus', window);
  await delay(500);
  const capture = name => execFileSync('import', ['-window', window, join(screenshots, `${name}.png`)]);
  capture('smoke-library');

  await click(500, 113);
  await type('Bulk selection test');
  await click(240, 188);
  x('key', '--delay', '80', 'ctrl+a');
  await delay(1000);
  capture('smoke-bulk-selected');
  await click(470, 846);
  capture('smoke-bulk-menu');
  await click(450, 822);
  await waitFor(() => readLibrary().playlists.find(p => p.name === 'Bulk set').track_ids.length === 500, 'Ctrl+A did not add all 500 filtered tracks to the playlist');
  assert.equal(readLibrary().playlists.find(p => p.name === 'Bulk set').track_ids[499], 510);
  x('key', 'Escape');
  await click(400, 187);
  x('keydown', 'Control_L');
  await click(400, 243);
  await click(400, 187);
  await click(400, 299);
  x('keyup', 'Control_L');
  capture('smoke-control-selection');
  await click(470, 846);
  await click(450, 793);
  await waitFor(() => readLibrary().playlists.find(p => p.name === 'Last one').track_ids.length === 4, 'Ctrl toggle selection did not add both non-adjacent tracks');
  assert.deepEqual(readLibrary().playlists.find(p => p.name === 'Last one').track_ids.slice(-2), [13, 15]);
  await click(75, 261);
  capture('smoke-bulk-playlist');
  await click(1435, 806);
  capture('smoke-bulk-scrolled');
  await click(1435, 218);
  await delay(400);
  await click(400, 219);
  x('keydown', 'Shift_L');
  await click(400, 275);
  x('keyup', 'Shift_L');
  await click(680, 846);
  await waitFor(() => readLibrary().playlists.find(p => p.name === 'Bulk set').track_ids.length === 497, 'Shift range removal did not remove three selected tracks');
  await click(75, 167);
  await click(360, 121);
  await type('Smoke opening');
  x('key', 'Return');
  await waitFor(() => readLibrary().playlists[0].name === 'Smoke opening', 'Library rename was not saved');
  await click(1370, 24);
  await waitFor(() => existsSync(manifest), 'Export did not publish a device library');
  await delay(500);
  assert.equal(readUsb().tracks.length, 6);
  await click(75, 394);
  capture('smoke-usb-workspace');
  await click(450, 156);
  await type('Phase Garden');
  await click(370, 202);
  await click(400, 225);
  x('key', '--delay', '80', 'ctrl+a');
  await delay(400);
  capture('smoke-usb-bulk-source');
  await click(735, 846);
  await waitFor(() => readUsb().playlists[0].track_ids.length === 6, 'Batch library transfer did not add both selected tracks');
  assert.equal(readUsb().tracks.length, 8);
  const playlistTitles = () => readUsb().playlists[0].track_ids.map(id => readUsb().tracks.find(t => t.id === id).title);
  assert.deepEqual(playlistTitles().slice(-2), ['Driftwood', 'Open air']);
  await click(450, 156);
  await type('Night windows');
  await click(400, 225);
  const replaced = readUsb().playlists[0].track_ids[0];
  await click(1000, 266);
  await click(640, 846);
  await waitFor(() => readUsb().playlists[0].track_ids[0] !== replaced, 'USB track replacement was not saved');
  assert.equal(playlistTitles()[0], 'Night windows');
  assert.equal(readUsb().tracks.length, 9);
  assert.equal(readLibrary().tracks.length, 510);
  await click(1000, 266);
  x('keydown', 'Shift_L');
  await click(1000, 322);
  x('keyup', 'Shift_L');
  await click(1290, 846);
  await waitFor(() => readUsb().playlists[0].track_ids.length === 3, 'Bulk USB removal did not remove the selected range');
  assert.equal(readUsb().tracks.length, 9);
  await click(185, 845);
  await waitFor(() => readUsb().playlists[0].track_ids.length === 6, 'Undo did not restore the bulk USB removal');
  await click(75, 395);
  await click(1170, 156);
  await waitFor(() => readUsb().playlists.length === 0, 'USB playlist deletion was not saved');
  assert.equal(readUsb().tracks.length, 9);
  await click(185, 845);
  await waitFor(() => readUsb().playlists.length === 1, 'Undo did not restore the USB playlist');
  await click(75, 395);
  await click(1370, 24);
  await waitFor(() => readLibrary().playlists.length === 5, 'USB playlist was not imported into the local library');
  const imported = readLibrary().playlists.find(p => p.name === 'Smoke opening (USB 1)');
  assert.equal(imported.track_ids.length, 6);
  for (const id of imported.track_ids) {
    const track = readLibrary().tracks.find(t => t.id === id);
    assert.ok(track.path.startsWith(work) && !track.path.startsWith(usb));
    assert.ok(existsSync(track.path));
  }
  await click(75, 363);
  await click(75, 768);
  await type('Sets');
  await click(165, 805);
  await waitFor(() => readUsb().folders.some(f => f.name === 'Sets'), 'USB folder creation was not saved');
  const sets = readUsb().folders.find(f => f.name === 'Sets');
  await click(75, 768);
  await type('House');
  await click(165, 805);
  await waitFor(() => readUsb().folders.some(f => f.name === 'House' && f.parent === sets.id), 'Nested USB folder was not saved');
  const house = readUsb().folders.find(f => f.name === 'House');
  await click(75, 768);
  await type('Nested opening');
  x('key', 'Return');
  await waitFor(() => readUsb().playlists.some(p => p.name === 'Nested opening' && p.parent === house.id), 'Playlist was not created in the selected USB folder');
  await delay(400);
  capture('smoke-usb-tree');
  await click(10, 459);
  capture('smoke-usb-tree-collapsed');
  await click(75, 459);
  capture('smoke-usb-folder');
  await click(75, 521);
  await click(1370, 24);
  await waitFor(() => readLibrary().playlists.some(p => p.name === 'Nested opening'), 'Nested USB playlist was not imported');
  const localSets = readLibrary().folders.find(f => f.name === 'Sets');
  const localHouse = readLibrary().folders.find(f => f.name === 'House' && f.parent === localSets.id);
  assert.equal(readLibrary().playlists.find(p => p.name === 'Nested opening').parent, localHouse.id);
  await delay(400);
  capture('smoke-imported-tree');

  await click(75, 104);
  await click(500, 113);
  await type('Open air');
  await click(400, 187);
  x('key', '--delay', '80', 'ctrl+a');
  await delay(400);
  await click(625, 846);
  await waitFor(() => readLibrary().playlists.some(p => p.name === 'New playlist' && p.track_ids.length === 2), 'Selection was not saved as a new populated playlist');
  x('type', '--clearmodifiers', '--delay', '40', 'Selected tracks');
  x('key', 'Return');
  await waitFor(() => readLibrary().playlists.some(p => p.name === 'Selected tracks' && p.track_ids.length === 2), 'New playlist name was not focused and selected for immediate renaming');
  capture('smoke-created-from-selection');

  assert.deepEqual(readFileSync(database).subarray(4096, originalDatabase.length), originalDatabase.subarray(4096));
  for (const [path, bytes] of originalAudio) assert.deepEqual(readFileSync(path), bytes);
  assert.equal(readFileSync(originalAnalysis, 'utf8'), 'Original cue data');
  writeFileSync(resolve('.prototype/smoke-last.json'), JSON.stringify({ work, library, usb, screenshots }, null, 2));
  console.log('Native Iced smoke passed: 510-track library, Ctrl+A bulk add, Shift range remove, sorted batch USB transfer, replacement, deletion and Undo, nested tree import, playlist from selection, and original byte preservation.');
  console.log(`Fixture: ${usb}`);

} finally {
  app.kill('SIGTERM');
}
