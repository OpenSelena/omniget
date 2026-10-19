import fs from 'node:fs';
import path from 'node:path';

const srcDir = 'build_v100';
const destDir = 'static';

function copyDir(src, dest) {
  if (!fs.existsSync(dest)) {
    fs.mkdirSync(dest, { recursive: true });
  }
  const entries = fs.readdirSync(src, { withFileTypes: true });
  for (const entry of entries) {
    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);
    if (entry.isDirectory()) {
      copyDir(srcPath, destPath);
    } else {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}

const items = fs.readdirSync(srcDir, { withFileTypes: true });
for (const item of items) {
  if (item.name === '_app' || item.name === 'index.html') continue;
  const s = path.join(srcDir, item.name);
  const d = path.join(destDir, item.name);
  if (item.isDirectory()) {
    console.log(`Syncing directory ${item.name} -> static/${item.name}`);
    copyDir(s, d);
  } else {
    console.log(`Syncing file ${item.name} -> static/${item.name}`);
    fs.copyFileSync(s, d);
  }
}
console.log('Static asset sync complete!');
