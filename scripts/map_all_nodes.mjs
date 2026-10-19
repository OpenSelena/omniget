import fs from 'node:fs';
import path from 'node:path';

const content = fs.readFileSync('build_v100/_app/immutable/entry/app.BKs9v87d.js', 'utf8');

// 1. Extract all node files
const nodeFiles = fs.readdirSync('build_v100/_app/immutable/nodes');
const nodeMap = {};
for (const file of nodeFiles) {
  const m = file.match(/^(\d+)\.([^.]+)\.js$/);
  if (m) {
    nodeMap[parseInt(m[1], 10)] = file;
  }
}

// 2. Load dictionary
const dict = JSON.parse(fs.readFileSync('scripts/routes_dictionary.json', 'utf8'));

// Special layout nodes:
// 0: Root layout
// 1: Root error
// 2: /about layout
// 3: /llm layout
// 4: /llm/catalog layout
// 5: /llm/company/[id] layout

const routeMapping = [];

routeMapping.push({
  nodeIndex: 0,
  nodeFile: nodeMap[0],
  targetFile: 'src/routes/+layout.svelte',
  type: 'layout',
  route: '/'
});

routeMapping.push({
  nodeIndex: 1,
  nodeFile: nodeMap[1],
  targetFile: 'src/routes/+error.svelte',
  type: 'error',
  route: '/'
});

routeMapping.push({
  nodeIndex: 2,
  nodeFile: nodeMap[2],
  targetFile: 'src/routes/about/+layout.svelte',
  type: 'layout',
  route: '/about'
});

routeMapping.push({
  nodeIndex: 3,
  nodeFile: nodeMap[3],
  targetFile: 'src/routes/llm/+layout.svelte',
  type: 'layout',
  route: '/llm'
});

routeMapping.push({
  nodeIndex: 4,
  nodeFile: nodeMap[4],
  targetFile: 'src/routes/llm/catalog/+layout.svelte',
  type: 'layout',
  route: '/llm/catalog'
});

routeMapping.push({
  nodeIndex: 5,
  nodeFile: nodeMap[5],
  targetFile: 'src/routes/llm/company/[id]/+layout.svelte',
  type: 'layout',
  route: '/llm/company/[id]'
});

for (const [route, config] of Object.entries(dict)) {
  const pageNode = Array.isArray(config) ? config[0] : config;
  const targetDir = route === '/' ? 'src/routes' : path.posix.join('src/routes', route);
  const targetFile = path.posix.join(targetDir, '+page.svelte');
  routeMapping.push({
    nodeIndex: pageNode,
    nodeFile: nodeMap[pageNode],
    targetFile,
    type: 'page',
    route
  });
}

// Sort by nodeIndex
routeMapping.sort((a, b) => a.nodeIndex - b.nodeIndex);

console.log(`Total mapped nodes: ${routeMapping.length}`);
fs.writeFileSync('scripts/route_mappings.json', JSON.stringify(routeMapping, null, 2));

for (const r of routeMapping.slice(0, 15)) {
  console.log(`Node ${r.nodeIndex} (${r.nodeFile}) -> ${r.targetFile}`);
}
