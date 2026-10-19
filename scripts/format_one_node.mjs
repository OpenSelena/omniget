import fs from 'node:fs';
import { execSync } from 'node:child_process';

if (!fs.existsSync('scripts/decompiled')) {
  fs.mkdirSync('scripts/decompiled', { recursive: true });
}

fs.copyFileSync('build_v100/_app/immutable/nodes/10.B1GI-gBr.js', 'scripts/decompiled/node_10_raw.js');
console.log('Running prettier on node_10_raw.js...');
execSync('npx prettier --write scripts/decompiled/node_10_raw.js', { stdio: 'inherit' });
console.log('Formatted node 10!');
