import fs from 'node:fs';

const content = fs.readFileSync('x:/omniget/build_v100/_app/immutable/entry/app.BKs9v87d.js', 'utf8');

// Look for route definitions in SvelteKit app.js
// SvelteKit routes typically look like: { id: "...", ... } or pattern /.../
const matches = [...content.matchAll(/id:\s*"([^"]+)"/g)].map(m => m[1]);
console.log('Unique routes in v1.0.0 frontend:');
const unique = [...new Set(matches)];
console.log(unique);
