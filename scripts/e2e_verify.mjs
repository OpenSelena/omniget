#!/usr/bin/env node

/**
 * OmniGet Documentation Suite E2E Test Runner & Verification Harness
 *
 * Implements a 4-tier opaque-box test architecture validating:
 * - Tier 1: Feature Coverage (Links, Search Modal, Theme, Copy Buttons, Immutability)
 * - Tier 2: Boundary & Corner Cases (Missing Anchors, Empty Queries, file:/// Normalization, Sticky Header Offsets)
 * - Tier 3: Cross-Feature Interactions (Theme Persistence, Modal Navigation, Deep-Link Offsets, Close Triggers)
 * - Tier 4: Real-World Scenarios (Reading Flow, Mobile 768px Collapse, Anti-AI Slop Lexicon & Staging Linter)
 */

import { runTier1Tests } from '../tests/tier1_features.mjs';
import { runTier2Tests } from '../tests/tier2_boundaries.mjs';
import { runTier3Tests } from '../tests/tier3_interactions.mjs';
import { runTier4Tests } from '../tests/tier4_realworld.mjs';

const args = process.argv.slice(2);
const isVerbose = args.includes('--verbose');
const isBaseline = args.includes('--baseline');
const tierFilter = args.find(a => a.startsWith('--tier='))?.split('=')[1];

// ANSI Color Helpers
const RESET = '\x1b[0m';
const BOLD = '\x1b[1m';
const GREEN = '\x1b[32m';
const RED = '\x1b[31m';
const YELLOW = '\x1b[33m';
const CYAN = '\x1b[36m';
const GRAY = '\x1b[90m';

console.log(`${BOLD}${CYAN}========================================================================${RESET}`);
console.log(`${BOLD}${CYAN}   OmniGet Documentation Suite: Comprehensive E2E Verification Harness  ${RESET}`);
console.log(`${BOLD}${CYAN}========================================================================${RESET}`);
console.log(`${GRAY}Mode: Opaque-Box Native Node Harness | Target: 7 Doc Pages + Root Landing Page${RESET}\n`);

const allResults = [];

// Execute Tier 1
if (!tierFilter || tierFilter === '1') {
  console.log(`${BOLD}Running Tier 1: Feature Coverage Tests...${RESET}`);
  const t1 = runTier1Tests();
  allResults.push(...t1);
}

// Execute Tier 2
if (!tierFilter || tierFilter === '2') {
  console.log(`${BOLD}Running Tier 2: Boundary & Corner Case Tests...${RESET}`);
  const t2 = runTier2Tests();
  allResults.push(...t2);
}

// Execute Tier 3
if (!tierFilter || tierFilter === '3') {
  console.log(`${BOLD}Running Tier 3: Cross-Feature Interaction Tests...${RESET}`);
  const t3 = runTier3Tests();
  allResults.push(...t3);
}

// Execute Tier 4
if (!tierFilter || tierFilter === '4') {
  console.log(`${BOLD}Running Tier 4: Real-World Scenario & Anti-AI Slop Tests...${RESET}`);
  const t4 = runTier4Tests();
  allResults.push(...t4);
}

console.log('\n' + `${BOLD}--- Test Execution Results by Tier ---${RESET}`);

const tiers = [1, 2, 3, 4];
let totalPassed = 0;
let totalFailed = 0;

for (const tier of tiers) {
  const tierResults = allResults.filter(r => r.tier === tier);
  if (tierResults.length === 0) continue;

  const passed = tierResults.filter(r => r.passed).length;
  const failed = tierResults.filter(r => !r.passed).length;
  totalPassed += passed;
  totalFailed += failed;

  const statusColor = failed === 0 ? GREEN : RED;
  console.log(`\n${BOLD}Tier ${tier} Summary:${RESET} ${statusColor}${passed}/${tierResults.length} Passed (${failed} Failed)${RESET}`);

  for (const r of tierResults) {
    const icon = r.passed ? `${GREEN}✔ PASS${RESET}` : `${RED}✘ FAIL${RESET}`;
    console.log(`  ${icon}  ${r.name}`);
    if (r.details) {
      console.log(`         ${GRAY}${r.details}${RESET}`);
    }
    if (!r.passed && (isVerbose || isBaseline)) {
      if (r.error) {
        console.log(`         ${YELLOW}Defect: ${r.error}${RESET}`);
      }
    }
  }
}

console.log('\n' + `${BOLD}${CYAN}========================================================================${RESET}`);
console.log(`${BOLD}Final Verification Summary:${RESET}`);
console.log(`  Total Tests Run : ${allResults.length}`);
console.log(`  Passing Tests    : ${GREEN}${totalPassed}${RESET}`);
console.log(`  Failing Tests    : ${totalFailed > 0 ? RED : GREEN}${totalFailed}${RESET}`);
const passRate = ((totalPassed / allResults.length) * 100).toFixed(1);
console.log(`  Pass Rate        : ${totalFailed === 0 ? GREEN : YELLOW}${passRate}%${RESET}`);
console.log(`${BOLD}${CYAN}========================================================================${RESET}\n`);

if (totalFailed > 0) {
  if (isBaseline) {
    console.log(`${YELLOW}[BASELINE NOTE] Baseline scorecard recorded with ${totalFailed} expected pre-existing issues.${RESET}`);
    console.log(`${YELLOW}Downstream milestones (M2: Design/CSS/Structure, M3: Copy/Slop) will resolve these defects.${RESET}\n`);
    process.exit(0);
  } else {
    console.log(`${RED}[FAIL] Suite detected ${totalFailed} failing assertions. Exiting with status 1.${RESET}`);
    console.log(`${GRAY}(Tip: Use 'node scripts/e2e_verify.mjs --baseline' to view baseline without non-zero exit code)${RESET}\n`);
    process.exit(1);
  }
} else {
  console.log(`${GREEN}[SUCCESS] All ${totalPassed} E2E tests passed with 100% compliance!${RESET}\n`);
  process.exit(0);
}
