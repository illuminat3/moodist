// Builds moodistd and stages it where Tauri's `externalBin` expects it
// (src-tauri/binaries/moodistd-<target triple>), so bundles ship it next to the UI.
import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const run = (cmd, args) => execFileSync(cmd, args, { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'] });

const triple = run('rustc', ['-vV']).match(/^host: (\S+)/m)[1];
const exe = process.platform === 'win32' ? '.exe' : '';

execFileSync('cargo', ['build', '--release', '--locked', '-p', 'moodistd'], { cwd: root, stdio: 'inherit' });

const targetDir = process.env.CARGO_TARGET_DIR ?? join(root, 'target');
const dest = join(root, 'src-tauri', 'binaries', `moodistd-${triple}${exe}`);
mkdirSync(dirname(dest), { recursive: true });
copyFileSync(join(targetDir, 'release', `moodistd${exe}`), dest);
console.log(`staged ${dest}`);
