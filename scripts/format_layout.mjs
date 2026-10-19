import fs from 'node:fs';
import prettier from 'prettier';

async function formatFile() {
  const code = fs.readFileSync('build_v100/_app/immutable/chunks/BWb5Vf6X.js', 'utf8');
  console.log('Formatting BWb5Vf6X.js...');
  const formatted = await prettier.format(code, { parser: 'babel' });
  if (!fs.existsSync('scripts/decompiled')) {
    fs.mkdirSync('scripts/decompiled', { recursive: true });
  }
  fs.writeFileSync('scripts/decompiled/root_layout.js', formatted);
  console.log('Saved formatted root layout to scripts/decompiled/root_layout.js');
}

formatFile();
