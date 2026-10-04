import { mkdirSync, existsSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';

const folder = resolve('.prototype/music');
mkdirSync(folder, { recursive: true });
mkdirSync(resolve('.prototype/demo-usb'), { recursive: true });

const songs = [
  ['Glasshouse', 'After hours', 118, '8A', 'Deep house'],
  ['North Circuit', 'Low orbit', 120, '9A', 'Deep house'],
  ['Glasshouse', 'Soft focus', 121, '8A', 'House'],
  ['Phase Garden', 'Open air', 124, '10A', 'House'],
  ['Soft Signal', 'Night windows', 125, '11A', 'House'],
  ['North Circuit', 'Side street', 126, '9A', 'Tech house'],
  ['Phase Garden', 'Driftwood', 127, '10A', 'Tech house'],
  ['Soft Signal', 'Second light', 128, '11A', 'Techno'],
  ['Glasshouse', 'Daybreak', 122, '8A', 'House'],
  ['North Circuit', 'Last train', 119, '9A', 'Deep house'],
];

const tracks = songs.map(([artist, title, bpm, key, genre], index) => {
  const path = join(folder, `${artist} - ${title}.wav`);
  if (!existsSync(path)) writeFileSync(path, groove(bpm, index));
  return { id: index + 1, path, title, artist, details: { album: 'Openbox demo audio', genre, key, bpm, duration_seconds: 16 } };
});

const library = resolve('.prototype/demo-library.json');
const seed = { tracks, playlists: [
  { id: 1, name: 'Warm up', track_ids: [1, 2, 3, 10] },
  { id: 2, name: 'After dark', track_ids: [4, 5, 6, 7, 8] },
  { id: 3, name: 'Last one', track_ids: [9, 10] },
] };
writeFileSync(resolve('.prototype/demo-seed.json'), JSON.stringify(seed, null, 2));
if (!existsSync(library)) {
  writeFileSync(library, JSON.stringify(seed, null, 2));
}
console.log('Demo audio and library ready. Tracks are generated test sounds, not commercial recordings.');

function groove(bpm, variation) {
  const rate = 44100;
  const frames = rate * 16;
  const dataSize = frames * 4;
  const buffer = Buffer.alloc(44 + dataSize);
  buffer.write('RIFF', 0);
  buffer.writeUInt32LE(36 + dataSize, 4);
  buffer.write('WAVEfmt ', 8);
  buffer.writeUInt32LE(16, 16);
  buffer.writeUInt16LE(1, 20);
  buffer.writeUInt16LE(2, 22);
  buffer.writeUInt32LE(rate, 24);
  buffer.writeUInt32LE(rate * 4, 28);
  buffer.writeUInt16LE(4, 32);
  buffer.writeUInt16LE(16, 34);
  buffer.write('data', 36);
  buffer.writeUInt32LE(dataSize, 40);
  const beat = 60 / bpm;
  for (let frame = 0; frame < frames; frame++) {
    const time = frame / rate;
    const phase = time % beat;
    const kick = Math.sin(2 * Math.PI * (48 * phase + 0.7 * (1 - Math.exp(-30 * phase)))) * Math.exp(-16 * phase);
    const tone = Math.sin(2 * Math.PI * (110 + variation * 11) * time) * 0.12;
    const fade = Math.min(time * 8, (16 - time) * 8, 1);
    const sample = Math.round((kick * 0.35 + tone) * fade * 32767);
    buffer.writeInt16LE(sample, 44 + frame * 4);
    buffer.writeInt16LE(sample, 46 + frame * 4);
  }
  return buffer;
}
