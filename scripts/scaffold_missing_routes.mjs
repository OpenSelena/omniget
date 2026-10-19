import fs from 'node:fs';
import path from 'node:path';

const mappings = JSON.parse(fs.readFileSync('scripts/route_mappings.json', 'utf8'));
const assetFiles = fs.readdirSync('build_v100/_app/immutable/assets');

const cssMap = {};
for (const file of assetFiles) {
  const m = file.match(/^(\d+)\.([^.]+)\.css$/);
  if (m) {
    cssMap[parseInt(m[1], 10)] = file;
  }
}

let createdCount = 0;

for (const m of mappings) {
  if (fs.existsSync(m.targetFile)) {
    console.log(`Skipping existing: ${m.targetFile}`);
    continue;
  }

  const dir = path.dirname(m.targetFile);
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }

  const cssFile = cssMap[m.nodeIndex] || '';
  const isLayout = m.type === 'layout';

  let content = '';
  if (isLayout) {
    content = `<script lang="ts">
  import V100NodeView from "$lib/v100/V100NodeView.svelte";
  import type { Snippet } from "svelte";

  let { children }: { children?: Snippet } = $props();
</script>

<V100NodeView
  nodeFile="${m.nodeFile}"
  ${cssFile ? `cssFile="${cssFile}"` : ''}
/>

{#if children}
  {@render children()}
{/if}
`;
  } else {
    content = `<script lang="ts">
  import V100NodeView from "$lib/v100/V100NodeView.svelte";
</script>

<V100NodeView
  nodeFile="${m.nodeFile}"
  ${cssFile ? `cssFile="${cssFile}"` : ''}
/>
`;
  }

  fs.writeFileSync(m.targetFile, content, 'utf8');
  console.log(`Created: ${m.targetFile} (node ${m.nodeIndex})`);
  createdCount++;
}

console.log(`\nSuccessfully created ${createdCount} route files!`);
