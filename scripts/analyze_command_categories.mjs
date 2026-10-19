import fs from 'node:fs';

const data = JSON.parse(fs.readFileSync('X:/REA/workspace/discovered_ipc_commands.json', 'utf8'));
const schemas = JSON.parse(fs.readFileSync('X:/REA/workspace/ipc_command_schemas.json', 'utf8')).commands;

const categories = {};

for (const item of data.standardCommands) {
  const cmd = item.command;
  let prefix = cmd.split('_')[0];
  if (['llm', 'arena', 'agentkit', 'mission', 'company', 'mcp', 'routine', 'roster', 'retro', 'loop', 'memory'].includes(prefix)) {
    prefix = 'llm_ecosystem';
  } else if (['tool', 'tools'].includes(prefix)) {
    prefix = 'tools';
  } else if (['music', 'video', 'tube', 'read', 'world', 'washmybelly'].includes(prefix)) {
    prefix = 'media_superpowers';
  } else if (['download', 'downloads', 'after'].includes(prefix)) {
    prefix = 'downloads';
  }

  if (!categories[prefix]) {
    categories[prefix] = { total: 0, inRepo: 0, newInV100: [], commands: [] };
  }
  categories[prefix].total++;
  if (item.existsInRepo) {
    categories[prefix].inRepo++;
  } else {
    categories[prefix].newInV100.push(cmd);
  }
  categories[prefix].commands.push(cmd);
}

console.log('=== IPC COMMAND CATEGORIES ===');
for (const [cat, info] of Object.entries(categories).sort((a, b) => b[1].total - a[1].total)) {
  console.log(`${cat.padEnd(20)}: Total=${String(info.total).padEnd(4)} InRepo=${String(info.inRepo).padEnd(4)} New=${info.newInV100.length}`);
}

fs.writeFileSync('scripts/command_categories.json', JSON.stringify(categories, null, 2));
console.log('Saved to scripts/command_categories.json');
