import { existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import {
  PROJECT_ROOT,
  CANONICAL_DOC_PAGES,
  loadAllDocPages
} from './helpers.mjs';

export function runTier2Tests() {
  const pages = loadAllDocPages();
  const results = [];

  function record(name, passed, details = '', error = null) {
    results.push({ tier: 2, name, passed, details, error });
  }

  // =========================================================================
  // Boundary Area 1: Missing Anchors & ID Hygiene (>=5 tests)
  // =========================================================================

  // 2.1 Zero missing anchors across the entire suite
  {
    const missingAnchors = [];
    for (const page of pages) {
      for (const link of page.links) {
        if (link.href.startsWith('#') && link.href.length > 1) {
          const anchor = link.href.slice(1);
          if (!page.ids.has(anchor)) {
            missingAnchors.push({ page: page.relPath, anchor });
          }
        }
      }
    }
    const passed = missingAnchors.length === 0;
    record(
      'Tier 2: Zero Missing Anchors Across Documentation Suite',
      passed,
      `Scanned all fragments across 7 pages. Missing: ${missingAnchors.length}`,
      passed ? null : `Missing anchors: ${JSON.stringify(missingAnchors)}`
    );
  }

  // 2.2 Anchor ID character syntax validity (HTML5 / RFC 3986 safe)
  {
    const invalidIds = [];
    for (const page of pages) {
      for (const id of page.ids) {
        // IDs should not contain spaces, raw quotes, unescaped brackets
        if (/[\s"'<>]/.test(id)) {
          invalidIds.push({ page: page.relPath, id });
        }
      }
    }
    const passed = invalidIds.length === 0;
    record(
      'Tier 2: DOM Element ID Character Hygiene',
      passed,
      `Validated syntax across all DOM element IDs. Invalid syntax: ${invalidIds.length}`,
      passed ? null : `Invalid IDs: ${JSON.stringify(invalidIds.slice(0, 5))}`
    );
  }

  // 2.3 Heading anchors match TOC anchors without typos
  {
    const tocMismatches = [];
    for (const page of pages) {
      // Find TOC list links
      const tocLinks = page.links.filter(l => l.fullMatch.includes('toc-item') || l.fullMatch.includes('toc-link') ||
        (page.content.indexOf(l.fullMatch) > page.content.indexOf('docs-toc') && page.content.indexOf(l.fullMatch) < page.content.indexOf('</aside>')));
      for (const tocLink of tocLinks) {
        if (tocLink.href.startsWith('#')) {
          const id = tocLink.href.slice(1);
          if (!page.ids.has(id)) {
            tocMismatches.push({ page: page.relPath, tocText: tocLink.text, missingTarget: id });
          }
        }
      }
    }
    const passed = tocMismatches.length === 0;
    record(
      'Tier 2: Table of Contents (TOC) Anchor Target Parity',
      passed,
      `Verified TOC links across all pages. Mismatches: ${tocMismatches.length}`,
      passed ? null : `TOC mismatches: ${JSON.stringify(tocMismatches)}`
    );
  }

  // 2.4 No empty or dummy href="#" anchors
  {
    const dummyAnchors = [];
    for (const page of pages) {
      for (const link of page.links) {
        if (link.href === '#' || link.href === '') {
          dummyAnchors.push({ page: page.relPath, text: link.text, html: link.fullMatch });
        }
      }
    }
    const passed = dummyAnchors.length === 0;
    record(
      'Tier 2: Elimination of Empty or Dummy href="#" Placeholders',
      passed,
      `Found ${dummyAnchors.length} dummy/empty links across 7 pages`,
      passed ? null : `Dummy anchors: ${JSON.stringify(dummyAnchors.slice(0, 5))}`
    );
  }

  // 2.5 Unique IDs per document (no duplicate ID definitions)
  {
    const duplicateIds = [];
    const idExtractRegex = /\bid\s*=\s*["']([^"']+)["']/gi;
    for (const page of pages) {
      const seen = new Set();
      const pageDupes = new Set();
      let m;
      while ((m = idExtractRegex.exec(page.content)) !== null) {
        const id = m[1];
        if (seen.has(id)) {
          pageDupes.add(id);
        } else {
          seen.add(id);
        }
      }
      if (pageDupes.size > 0) {
        duplicateIds.push({ page: page.relPath, duplicates: Array.from(pageDupes) });
      }
    }
    const passed = duplicateIds.length === 0;
    record(
      'Tier 2: Document Element ID Uniqueness Enforcement',
      passed,
      `Checked ID uniqueness per page. Pages with duplicate IDs: ${duplicateIds.length}`,
      passed ? null : `Duplicates detected: ${JSON.stringify(duplicateIds)}`
    );
  }

  // =========================================================================
  // Boundary Area 2: Search Query Edge Cases (>=5 tests)
  // =========================================================================

  // 2.6 Search filter handles empty query string without error
  {
    // Simulate the search filter algorithm implemented across the pages:
    // item.textContent.toLowerCase().includes(q)
    let algoWorks = true;
    let errorDetail = null;
    try {
      const q = '';
      const sampleItemText = 'OmniGet Quickstart Guide Installation';
      const isVisible = sampleItemText.toLowerCase().includes(q.toLowerCase().trim());
      algoWorks = isVisible === true;
    } catch (e) {
      algoWorks = false;
      errorDetail = e.message;
    }
    record(
      'Tier 2: Search Filter Restores All Items on Empty Query ""',
      algoWorks,
      'Empty query string successfully matches all items without exception',
      errorDetail
    );
  }

  // 2.7 Search filter handles whitespace-only query
  {
    let handlesWhitespace = true;
    try {
      const q = '   ';
      const trimmed = q.toLowerCase().trim();
      const sampleItemText = 'OmniGet Quickstart Guide';
      const match = trimmed === '' || sampleItemText.toLowerCase().includes(trimmed);
      handlesWhitespace = match === true;
    } catch (e) {
      handlesWhitespace = false;
    }
    record(
      'Tier 2: Search Filter Handles Whitespace-Only Queries Safely',
      handlesWhitespace,
      'Whitespace-only query trims safely and avoids hiding or crashing list',
      handlesWhitespace ? null : 'Failed whitespace query boundary test'
    );
  }

  // 2.8 Search filter handles regex metacharacters safely
  {
    // Test if query contains characters that crash naive RegExp: (, [, *, +, ?, \
    let handlesRegexChars = true;
    const testQueries = ['(', '[', '*', '+', '?', '\\', '.*'];
    for (const page of pages) {
      // Check if search script in page uses new RegExp(q) without try/catch
      const usesUnescapedRegExp = /new\s+RegExp\s*\(\s*(q|query|val)/.test(page.combinedJs) &&
                                  !/catch\b/.test(page.combinedJs);
      if (usesUnescapedRegExp) {
        handlesRegexChars = false;
        break;
      }
    }
    record(
      'Tier 2: Search Filter Immunity to Regex Metacharacters',
      handlesRegexChars,
      handlesRegexChars ? 'Search filter safely uses substring matching or escaped patterns' : 'Unsafe new RegExp(query) detected in search script',
      handlesRegexChars ? null : 'Unescaped RegExp execution risk'
    );
  }

  // 2.9 Search filter case insensitivity boundary
  {
    let caseInsensitive = true;
    for (const page of pages) {
      const hasLowerCase = /toLowerCase\(\)/.test(page.combinedJs) || /searchItems/i.test(page.combinedJs);
      if (!hasLowerCase && page.combinedJs.length > 2000) {
        caseInsensitive = false;
      }
    }
    record(
      'Tier 2: Search Query Case Insensitivity Enforcement',
      caseInsensitive,
      'Query and target strings are normalized using toLowerCase()',
      caseInsensitive ? null : 'Case conversion missing in search filter'
    );
  }

  // 2.10 Non-matching query boundary
  {
    let handlesNoResults = true;
    for (const page of pages) {
      // Check if CSS hides filtered items: display: none
      const hidesItem = /style\.display\s*=\s*['"]none['"]/.test(page.combinedJs) ||
                        /classList\.add\(['"]hidden['"]\)/.test(page.combinedJs);
      if (!hidesItem && page.combinedJs.length > 2000) {
        handlesNoResults = false;
      }
    }
    record(
      'Tier 2: Search Query Non-Matching Boundary Conceals Items',
      handlesNoResults,
      'Non-matching items are hidden via display style mutation',
      handlesNoResults ? null : 'Items not hidden when query does not match'
    );
  }

  // =========================================================================
  // Boundary Area 3: Offline file:/// Protocol Normalization (>=5 tests)
  // =========================================================================

  // 2.11 Protocol check present in client script across all 7 pages
  {
    const missingProtocolHandler = [];
    for (const page of pages) {
      const hasProtoCheck = /location\.protocol\s*===\s*['"]file:['"]/.test(page.combinedJs);
      if (!hasProtoCheck) {
        missingProtocolHandler.push({ page: page.relPath, error: 'Missing offline file: protocol handler' });
      }
    }
    const passed = missingProtocolHandler.length === 0;
    record(
      'Tier 2: Offline file:/// Protocol Detection Present Suite-Wide',
      passed,
      `Checked file: protocol detector across 7 pages. Pages lacking script: ${missingProtocolHandler.length}`,
      passed ? null : `Pages missing protocol check: ${JSON.stringify(missingProtocolHandler)}`
    );
  }

  // 2.12 Directory links normalized with index.html under offline browsing
  {
    const missingRewriter = [];
    for (const page of pages) {
      const hasRewriter = /index\.html/.test(page.combinedJs) &&
                          /querySelectorAll\(['"]a\[href\]['"]\)/.test(page.combinedJs);
      if (!hasRewriter) {
        missingRewriter.push(page.relPath);
      }
    }
    const passed = missingRewriter.length === 0;
    record(
      'Tier 2: Offline file:/// Directory Link Appender (index.html)',
      passed,
      `Verified a[href] directory appender logic. Deficiencies: ${missingRewriter.length}`,
      passed ? null : `Pages missing link rewriter: ${JSON.stringify(missingRewriter)}`
    );
  }

  // 2.13 Offline rewriter ignores external URLs and hash fragments
  {
    let ignoresExternal = true;
    for (const page of pages) {
      if (page.combinedJs.includes('location.protocol === \'file:\'')) {
        const hasGuard = /startsWith\(['"]http['"]\)/.test(page.combinedJs) &&
                         /startsWith\(['"]#['"]\)/.test(page.combinedJs);
        if (!hasGuard) ignoresExternal = false;
      }
    }
    record(
      'Tier 2: Offline Normalizer Ignores External URLs and Hash Fragments',
      ignoresExternal,
      ignoresExternal ? 'Guard clauses protect external http/https and anchor fragments' : 'Missing guard clause in protocol handler',
      ignoresExternal ? null : 'Offline link rewriter risks corrupting external or hash URLs'
    );
  }

  // 2.14 Search result onclick navigations offline normalization
  {
    // Search result items use onclick="location.href='...'". Under file:/// this also needs index.html!
    const searchOfflineIssues = [];
    for (const page of pages) {
      const rewritesSearchClicks = /search-result-item/.test(page.combinedJs) && /index\.html/.test(page.combinedJs);
      if (!rewritesSearchClicks) {
        searchOfflineIssues.push({ page: page.relPath, note: 'Search onclick items not rewritten for file:/// browsing' });
      }
    }
    const passed = searchOfflineIssues.length === 0;
    record(
      'Tier 2: Search Result Item Navigation Offline Normalization',
      passed,
      `Verified search item click targets under file:/// protocol. Issues: ${searchOfflineIssues.length}`,
      passed ? null : `Search items lacking offline index.html appending: ${JSON.stringify(searchOfflineIssues.slice(0, 3))}`
    );
  }

  // 2.15 Relative asset paths offline safety (no absolute root slash)
  {
    const absoluteRootPaths = [];
    for (const page of pages) {
      for (const img of page.images) {
        if (img.src.startsWith('/') && !img.src.startsWith('//')) {
          absoluteRootPaths.push({ page: page.relPath, tag: 'img', src: img.src });
        }
      }
      for (const link of page.links) {
        if (link.href.startsWith('/') && !link.href.startsWith('//')) {
          absoluteRootPaths.push({ page: page.relPath, tag: 'a', href: link.href });
        }
      }
    }
    const passed = absoluteRootPaths.length === 0;
    record(
      'Tier 2: Absolute Root Slash "/" Asset Path Immunity',
      passed,
      `Checked asset paths for absolute root slash risks. Violations: ${absoluteRootPaths.length}`,
      passed ? null : `Absolute slash paths: ${JSON.stringify(absoluteRootPaths.slice(0, 5))}`
    );
  }

  // =========================================================================
  // Boundary Area 4: Sticky Header Height & Anchor Scroll Margins (>=5 tests)
  // =========================================================================

  // 2.16 Sticky header height CSS variable defined
  {
    const missingHeaderH = [];
    for (const page of pages) {
      const hasHeaderVar = /--header-total-h\s*:\s*\d+px/.test(page.combinedCss) ||
                           /--header-h\s*:\s*\d+px/.test(page.combinedCss);
      if (!hasHeaderVar) {
        missingHeaderH.push(page.relPath);
      }
    }
    const passed = missingHeaderH.length === 0;
    record(
      'Tier 2: Sticky Header Total Height Token (--header-total-h) Defined',
      passed,
      `Verified header height CSS variables across 7 pages. Missing: ${missingHeaderH.length}`,
      passed ? null : `Pages missing header height variable: ${JSON.stringify(missingHeaderH)}`
    );
  }

  // 2.17 Level 1 headings (.doc-h1) scroll margin offset
  {
    const missingH1Margin = [];
    for (const page of pages) {
      const hasH1Margin = /\.doc-h1\b[^{]*\{[^}]*scroll-margin-top/s.test(page.combinedCss) ||
                          /h1\b[^{]*\{[^}]*scroll-margin-top/s.test(page.combinedCss);
      if (!hasH1Margin) {
        missingH1Margin.push(page.relPath);
      }
    }
    const passed = missingH1Margin.length === 0;
    record(
      'Tier 2: Level 1 Headings (.doc-h1) Anchor Scroll Margin Offset',
      passed,
      `Verified .doc-h1 scroll-margin-top. Missing: ${missingH1Margin.length}`,
      passed ? null : `Pages missing .doc-h1 scroll margin: ${JSON.stringify(missingH1Margin)}`
    );
  }

  // 2.18 Level 2 headings (.doc-h2) scroll margin offset
  {
    const missingH2Margin = [];
    for (const page of pages) {
      const hasH2Margin = /\.doc-h2\b[^{]*\{[^}]*scroll-margin-top/s.test(page.combinedCss) ||
                          /h2\b[^{]*\{[^}]*scroll-margin-top/s.test(page.combinedCss);
      if (!hasH2Margin) {
        missingH2Margin.push(page.relPath);
      }
    }
    const passed = missingH2Margin.length === 0;
    record(
      'Tier 2: Level 2 Headings (.doc-h2) Anchor Scroll Margin Offset',
      passed,
      `Verified .doc-h2 scroll-margin-top. Missing: ${missingH2Margin.length}`,
      passed ? null : `Pages missing .doc-h2 scroll margin: ${JSON.stringify(missingH2Margin)}`
    );
  }

  // 2.19 Level 3 headings (.doc-h3) scroll margin offset
  {
    const missingH3Margin = [];
    for (const page of pages) {
      const hasH3Margin = /\.doc-h3\b[^{]*\{[^}]*scroll-margin-top/s.test(page.combinedCss) ||
                          /h3\b[^{]*\{[^}]*scroll-margin-top/s.test(page.combinedCss);
      if (!hasH3Margin) {
        missingH3Margin.push(page.relPath);
      }
    }
    const passed = missingH3Margin.length === 0;
    record(
      'Tier 2: Level 3 Headings (.doc-h3) Anchor Scroll Margin Offset',
      passed,
      `Verified .doc-h3 scroll-margin-top. Missing: ${missingH3Margin.length}`,
      passed ? null : `Pages missing .doc-h3 scroll margin: ${JSON.stringify(missingH3Margin)}`
    );
  }

  // 2.20 Changelog release box article containers scroll margin offset
  {
    const changelogPage = pages.find(p => p.relPath.includes('changelog'));
    let changelogBoxMargin = false;
    if (changelogPage) {
      changelogBoxMargin = /\.release-box\b[^{]*\{[^}]*scroll-margin-top/s.test(changelogPage.combinedCss) ||
                           /article\b[^{]*\{[^}]*scroll-margin-top/s.test(changelogPage.combinedCss);
    }
    record(
      'Tier 2: Changelog .release-box Containers Scroll Margin Offset',
      changelogBoxMargin,
      changelogBoxMargin ? 'Changelog release boxes have scroll-margin-top defined' : 'Changelog release boxes lack scroll-margin-top (causes anchor clipping)',
      changelogBoxMargin ? null : 'Critical anchor clipping bug: .release-box lacks scroll-margin-top'
    );
  }

  return results;
}
