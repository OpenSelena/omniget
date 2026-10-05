import { existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import {
  PROJECT_ROOT,
  CANONICAL_DOC_PAGES,
  loadAllDocPages
} from './helpers.mjs';

export function runTier4Tests() {
  const pages = loadAllDocPages();
  const results = [];

  function record(name, passed, details = '', error = null) {
    results.push({ tier: 4, name, passed, details, error });
  }

  // =========================================================================
  // Real-World Scenario 1: Sequential Reading Flow (Overview -> Changelog)
  // =========================================================================
  {
    // Reading sequence:
    // 0: docs/index.html (Quickstart)
    // 1: docs/installation/index.html (Installation)
    // 2: docs/mcp/index.html (MCP Server)
    // 3: docs/courses/index.html (Courses)
    // 4: docs/opennami/index.html (OpenNami)
    // 5: docs/troubleshooting/index.html (Troubleshooting)
    // 6: docs/changelog/index.html (Changelog)
    const expectedChain = [
      { from: 'docs/index.html', nextFile: 'installation/' },
      { from: 'docs/installation/index.html', prevFile: '../', nextFile: '../mcp/' },
      { from: 'docs/mcp/index.html', prevFile: '../installation/', nextFile: '../courses/' },
      { from: 'docs/courses/index.html', prevFile: '../mcp/', nextFile: '../opennami/' },
      { from: 'docs/opennami/index.html', prevFile: '../courses/', nextFile: '../troubleshooting/' },
      { from: 'docs/troubleshooting/index.html', prevFile: '../opennami/', nextFile: '../changelog/' },
      { from: 'docs/changelog/index.html', prevFile: '../troubleshooting/' }
    ];

    const chainBreaks = [];
    for (const step of expectedChain) {
      const page = pages.find(p => p.relPath.replace(/\\/g, '/') === step.from);
      if (!page) {
        chainBreaks.push({ step: step.from, error: 'Page missing' });
        continue;
      }

      if (!page.pagination.hasGrid) {
        chainBreaks.push({ step: step.from, error: 'Missing .page-nav-grid pagination container' });
        continue;
      }

      if (step.prevFile && (!page.pagination.prev || !page.pagination.prev.includes(step.prevFile.replace(/\/$/, '')))) {
        chainBreaks.push({
          step: step.from,
          expectedPrev: step.prevFile,
          actualPrev: page.pagination.prev,
          error: 'Broken previous page link'
        });
      }

      if (step.nextFile && (!page.pagination.next || !page.pagination.next.includes(step.nextFile.replace(/\/$/, '')))) {
        chainBreaks.push({
          step: step.from,
          expectedNext: step.nextFile,
          actualNext: page.pagination.next,
          error: 'Broken next page link'
        });
      }
    }

    const passed = chainBreaks.length === 0;
    record(
      'Tier 4: Sequential Reading Flow (Quickstart -> Changelog Chain)',
      passed,
      `Verified 7-page bidirectional reading chain. Broken links: ${chainBreaks.length}`,
      passed ? null : `Chain breaks: ${JSON.stringify(chainBreaks)}`
    );
  }

  // =========================================================================
  // Real-World Scenario 2: Mobile Responsiveness (< 768px Viewport)
  // =========================================================================

  // 2.1 Viewport meta tag on all 7 pages
  {
    const missingViewport = [];
    for (const page of pages) {
      const hasViewport = page.metas.some(m => m.name.toLowerCase() === 'viewport' && m.content.includes('width=device-width'));
      if (!hasViewport) {
        missingViewport.push(page.relPath);
      }
    }
    const passed = missingViewport.length === 0;
    record(
      'Tier 4: Mobile Viewport Meta Tag Conformance',
      passed,
      `Checked viewport meta tag on 7 pages. Missing: ${missingViewport.length}`,
      passed ? null : `Pages missing viewport meta: ${JSON.stringify(missingViewport)}`
    );
  }

  // 2.2 Collapsing .nav-links under @media (max-width: 768px)
  {
    // Survey 1 & 2 discovered: .nav-links remains displayed at <=768px on several pages, causing top header horizontal overflow.
    // CSS rule required: @media (max-width: 768px) { .nav-links { display: none; } }
    const pagesWithoutNavCollapse = [];
    for (const page of pages) {
      const css = page.combinedCss;
      // Look for media query with 768px or 820px that hides .nav-links
      const hidesNavLinks = /@media[^{]*\b(768px|820px)\b[^{]*\{[^}]*\.nav-links\b[^{]*\{[^}]*display\s*:\s*none/s.test(css) ||
                            /@media[^{]*max-width\s*:\s*(768px|820px)[^{]*\{[^{}]*\{[^{}]*display\s*:\s*none/s.test(css);
      if (!hidesNavLinks) {
        pagesWithoutNavCollapse.push(page.relPath);
      }
    }
    const passed = pagesWithoutNavCollapse.length === 0;
    record(
      'Tier 4: Mobile Responsive Navigation Collapse (@media <=768px)',
      passed,
      `Verified .nav-links collapse under 768px. Pages lacking rule: ${pagesWithoutNavCollapse.length}`,
      passed ? null : `Pages failing mobile header collapse: ${JSON.stringify(pagesWithoutNavCollapse)}`
    );
  }

  // =========================================================================
  // Real-World Scenario 3: Anti-AI Slop Lexicon Linter (17 Banned Buzzwords)
  // =========================================================================
  {
    const BANNED_BUZZWORDS = [
      { word: 'seamlessly', regex: /\bseamlessly\b/i },
      { word: 'seamless', regex: /\bseamless\b/i },
      { word: 'comprehensive', regex: /\bcomprehensive\b/i },
      { word: 'empowering', regex: /\bempower(ing|s|ed)?\b/i },
      { word: 'half the battle', regex: /\bhalf the battle\b/i },
      { word: 'revolutionize', regex: /\brevolution(ize[ds]?|ary)\b/i },
      { word: 'delve', regex: /\bdelv(e[ds]?|ing)\b/i },
      { word: 'testament', regex: /\btestament\b/i },
      { word: 'elevate', regex: /\belevat(e[ds]?|ing)\b/i },
      { word: 'comprehensive suite', regex: /\bcomprehensive suite\b/i },
      { word: 'robust powerhouse', regex: /\brobust powerhouse\b/i },
      { word: 'powerhouse', regex: /\bpowerhouse\b/i },
      { word: 'supercharge', regex: /\bsupercharg(e[ds]?|ing)\b/i },
      { word: 'cutting-edge', regex: /\bcutting-edge\b/i },
      { word: 'game-changer', regex: /\bgame[- ]changer\b/i },
      { word: 'unleash', regex: /\bunleash(ed|ing)?\b/i },
      { word: 'tapestry', regex: /\btapestry\b/i },
      { word: 'multifaceted', regex: /\bmultifaceted\b/i },
      { word: 'unparalleled', regex: /\bunparalleled\b/i }
    ];

    const violations = [];
    for (const page of pages) {
      // Check visible stripped text
      for (const bw of BANNED_BUZZWORDS) {
        const match = page.strippedText.match(bw.regex);
        if (match) {
          violations.push({ page: page.relPath, word: bw.word, match: match[0] });
        }
      }
      // Check meta descriptions
      for (const meta of page.metas) {
        for (const bw of BANNED_BUZZWORDS) {
          if (bw.regex.test(meta.content)) {
            violations.push({ page: page.relPath, word: bw.word, location: `meta:${meta.name}` });
          }
        }
      }
    }

    const passed = violations.length === 0;
    record(
      'Tier 4: Anti-AI Slop Lexicon Linter (17 Banned Buzzwords)',
      passed,
      `Audited 7 documentation pages against 17 banned buzzword patterns. Violations detected: ${violations.length}`,
      passed ? null : `Slop buzzword occurrences: ${JSON.stringify(violations)}`
    );
  }

  // =========================================================================
  // Real-World Scenario 4: Anti-AI Slop Rhetorical Question Headings & Staging
  // =========================================================================
  {
    const rhetoricalViolations = [];

    for (const page of pages) {
      // 1. Headings containing question marks
      for (const h of page.headings) {
        if (h.text.includes('?')) {
          rhetoricalViolations.push({
            page: page.relPath,
            type: 'Heading with question mark',
            heading: h.text,
            level: h.level
          });
        }
      }

      // 2. Specific rhetorical headings cataloged in Survey 3:
      // "What is OpenNami?", "What is the OmniGet MCP Server?", "Why this happens:"
      const knownQuestions = [
        /What is OpenNami\?/i,
        /What is the OmniGet MCP Server\?/i,
        /Looking for older releases/i,
        /Why this happens:/i
      ];

      for (const kq of knownQuestions) {
        if (kq.test(page.content)) {
          rhetoricalViolations.push({
            page: page.relPath,
            type: 'Cataloged rhetorical staging phrase',
            pattern: kq.toString()
          });
        }
      }
    }

    const passed = rhetoricalViolations.length === 0;
    record(
      'Tier 4: Anti-AI Slop Rhetorical Question Headings & Staging Purge',
      passed,
      `Audited headings and staging labels across all pages. Violations: ${rhetoricalViolations.length}`,
      passed ? null : `Rhetorical staging detected: ${JSON.stringify(rhetoricalViolations)}`
    );
  }

  // =========================================================================
  // Real-World Scenario 5: Anti-AI Slop Fillers & Factual Consistency
  // =========================================================================
  {
    const fillerAndFactualViolations = [];

    for (const page of pages) {
      // 1. Empty qualifiers and fillers:
      // "simply pasting", "just like opening", "designed to help you actually", "allowing you to access"
      const emptyFillers = [
        /simply pasting/i,
        /just like opening/i,
        /designed to help you actually/i,
        /allowing you to access/i,
        /highlighting true/i
      ];

      for (const ef of emptyFillers) {
        if (ef.test(page.strippedText)) {
          fillerAndFactualViolations.push({
            page: page.relPath,
            type: 'Empty filler/qualifier',
            match: ef.toString()
          });
        }
      }

      // 2. Fake contrast pairs: "Not just X, but Y", "not only X, but also Y"
      const fakeContrastRegex = /\bnot (?:only|just|merely)\b[^.!?]{5,60}\bbut (?:also)?\b/gi;
      const contrastMatches = page.strippedText.match(fakeContrastRegex);
      if (contrastMatches) {
        for (const m of contrastMatches) {
          fillerAndFactualViolations.push({
            page: page.relPath,
            type: 'Fake contrast pair',
            match: m
          });
        }
      }
    }

    // 3. Factual consistency:
    // Course platform count in courses page: must be 38 (not 37)
    const coursesPage = pages.find(p => p.relPath.includes('courses'));
    if (coursesPage) {
      if (/Supported 37 Course Platforms/i.test(coursesPage.content)) {
        fillerAndFactualViolations.push({
          page: coursesPage.relPath,
          type: 'Factual inconsistency: 37 platforms mentioned instead of 38'
        });
      }
    }

    // 4. Phantom "Coursera" references in search modals or tables
    for (const page of pages) {
      if (/Coursera/i.test(page.content)) {
        fillerAndFactualViolations.push({
          page: page.relPath,
          type: 'Factual error: Phantom "Coursera" referenced in text or search snippet'
        });
      }
    }

    const passed = fillerAndFactualViolations.length === 0;
    record(
      'Tier 4: Anti-AI Slop Passive Fillers, Fake Contrasts & Factual Consistency',
      passed,
      `Audited empty qualifiers, fake contrasts, and platform counts. Violations: ${fillerAndFactualViolations.length}`,
      passed ? null : `Violations: ${JSON.stringify(fillerAndFactualViolations)}`
    );
  }

  return results;
}
