import fs from 'node:fs';

const content = fs.readFileSync('build_v100/_app/immutable/entry/app.BKs9v87d.js', 'utf8');

const dictIdx = content.indexOf('"/youtube":[67]');
if (dictIdx !== -1) {
  const start = content.lastIndexOf('{', dictIdx);
  const end = content.indexOf('}', dictIdx);
  const dictStr = content.substring(start, end + 1);
  console.log('=== ROUTE DICTIONARY ===');
  console.log(dictStr);
  const dict = JSON.parse(dictStr);
  fs.writeFileSync('scripts/routes_dictionary.json', JSON.stringify(dict, null, 2));
}

// Find nodes array
// In app.js, nodes are defined as: _r=[()=>import("..."), ...]
const expIdx = content.indexOf('as nodes');
console.log('=== NODES EXPORT CONTEXT ===');
console.log(content.substring(expIdx - 500, expIdx + 50));
