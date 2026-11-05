import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { mkdtempSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
const app = spawn(resolve(process.env.OPENBOX_BIN || 'target/debug/openbox'), ['--library', join(mkdtempSync('/tmp/openbox-identity-'), 'library.json')], { env: { ...process.env, ICED_BACKEND: 'tiny-skia' }, stdio: 'ignore' });
try {
  let window;
  for (let n = 0; n < 100 && !window; n++) {
    try { window = execFileSync('xdotool', ['search', '--onlyvisible', '--pid', String(app.pid), '--name', '^Openbox$'], { encoding: 'utf8' }).trim().split('\n')[0]; } catch {}
    await delay(100);
  }
  assert.ok(window, 'No native window');
  const identity = execFileSync('xprop', ['-id', window, 'WM_CLASS'], { encoding: 'utf8' });
  assert.match(identity, /"io\.github\.alexanderwanyoike\.openbox"/, 'Window identity must match the Openbox desktop entry');
  const icon = execFileSync('xprop', ['-id', window, '-f', '_NET_WM_ICON', '32c', '_NET_WM_ICON'], { encoding: 'utf8', maxBuffer: 2000000 });
  assert.match(icon, /= 256, 256,/, 'The native window must supply its own icon');
  console.log('Native identity passed: Openbox WM_CLASS and bundled 256-pixel window icon.');
} finally { app.kill('SIGTERM'); await new Promise(resolve => app.once('exit', resolve)); }
