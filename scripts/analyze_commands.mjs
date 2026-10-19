import fs from 'node:fs';
import path from 'node:path';

function getFiles(dir) {
  let results = [];
  const list = fs.readdirSync(dir);
  for (const file of list) {
    const full = path.join(dir, file);
    const stat = fs.statSync(full);
    if (stat && stat.isDirectory()) {
      results = results.concat(getFiles(full));
    } else if (file.endsWith('.js')) {
      results.push(full);
    }
  }
  return results;
}

const jsFiles = getFiles('build_v100/_app/immutable');
console.log(`Scanning ${jsFiles.length} JS files...`);

const allCommands = new Map();

for (const file of jsFiles) {
  const content = fs.readFileSync(file, 'utf8');
  // Check if invoke is imported
  const m = content.match(/import\s*\{[^}]*invoke\s+as\s+([a-zA-Z0-9_$]+)[^}]*\}\s*from/);
  const invokeName = m ? m[1] : (content.includes('invoke(') ? 'invoke' : null);
  if (!invokeName) continue;

  const regex = new RegExp(invokeName + '\\s*\\(\\s*["\']([^"\']+)["\']', 'g');
  let match;
  while ((match = regex.exec(content)) !== null) {
    const cmd = match[1];
    if (!allCommands.has(cmd)) {
      allCommands.set(cmd, []);
    }
    const relPath = path.relative('build_v100', file).replace(/\\/g, '/');
    if (!allCommands.get(cmd).includes(relPath)) {
      allCommands.get(cmd).push(relPath);
    }
  }
}

console.log(`Found ${allCommands.size} unique Tauri commands invoked across frontend!`);

const sorted = Object.fromEntries(
  [...allCommands.entries()].sort(([a], [b]) => a.localeCompare(b))
);

fs.writeFileSync('scripts/all_frontend_invoked_commands.json', JSON.stringify(sorted, null, 2));
console.log('Saved to scripts/all_frontend_invoked_commands.json');

const sample = Object.keys(sorted).slice(0, 30);
console.log('Sample of 30 commands:');
console.log(sample);
