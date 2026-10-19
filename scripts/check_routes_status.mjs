import fs from 'node:fs';

const mappings = JSON.parse(fs.readFileSync('scripts/route_mappings.json', 'utf8'));

const existing = [];
const missing = [];

for (const m of mappings) {
  if (fs.existsSync(m.targetFile)) {
    existing.push(m);
  } else {
    missing.push(m);
  }
}

console.log(`Total routes: ${mappings.length}`);
console.log(`Existing in src/routes: ${existing.length}`);
console.log(`Missing/New in v1.0.0: ${missing.length}`);

console.log('\n--- Missing/New Routes ---');
for (const m of missing) {
  console.log(`Node ${m.nodeIndex} -> ${m.route} (${m.targetFile})`);
}
