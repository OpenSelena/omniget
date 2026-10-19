import fs from 'node:fs';

const content = fs.readFileSync('build_v100/_app/immutable/chunks/BWb5Vf6X.js', 'utf8');

// Search for route paths like "/llm", "/downloads", "/settings", "/superpowers" etc.
const matches = [...content.matchAll(/href:\s*"([^"]+)"/g)].map(m => m[1]);
console.log('href matches in root layout:');
console.log([...new Set(matches)]);

// Also search for navigation list or array in root layout
const routeRefs = [...content.matchAll(/"\/(llm|superpowers|music|omni|pet|downloads|settings|about|marketplace|tools|courses|convert|telegram|world|read|video|youtube|limits-strip)[^"]*"/g)].map(m => m[0]);
console.log('\nRoute references in root layout:');
console.log([...new Set(routeRefs)]);
