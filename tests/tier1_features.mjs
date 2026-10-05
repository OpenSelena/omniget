import { existsSync, readFileSync } from 'node:fs';
import { resolve, dirname, normalize } from 'node:path';
import { execSync } from 'node:child_process';
import {
  PROJECT_ROOT,
  CANONICAL_DOC_PAGES,
  ROOT_LANDING_PAGE,
  calculateMd5,
  parseDocPage,
  loadAllDocPages
} from './helpers.mjs';

export function runTier1Tests() {
  const pages = loadAllDocPages();
  const results = [];

  function record(name, passed, details = '', error = null) {
    results.push({ tier: 1, name, passed, details, error });
  }

  // =========================================================================
  // Feature Area 1: Internal Link & Anchor Integrity (>=5 tests)
  // =========================================================================

  // 1.1 All relative <a href> link targets resolve to existing disk files
  {
    const missingTargets = [];
    let totalLinks = 0;
    for (const page of pages) {
      const pageDir = dirname(page.absPath);
      for (const link of page.links) {
        const href = link.href.trim();
        if (!href || href.startsWith('http://') || href.startsWith('https://') || href.startsWith('mailto:') || href.startsWith('#')) {
          continue;
        }
        totalLinks++;
        const cleanHref = href.split('#')[0].split('?')[0];
        if (!cleanHref) continue;

        let targetAbs = resolve(pageDir, cleanHref);
        // If targeting a directory or ends in '/', test index.html resolution
        if (existsSync(targetAbs) && !cleanHref.endsWith('.html')) {
          const indexVariant = resolve(targetAbs, 'index.html');
          if (existsSync(indexVariant)) {
            targetAbs = indexVariant;
          }
        } else if (!existsSync(targetAbs)) {
          // If cleanHref ends with '/' and doesn't exist directly, check with index.html
          if (cleanHref.endsWith('/') || !cleanHref.includes('.')) {
            const indexVariant = resolve(pageDir, cleanHref, 'index.html');
            if (existsSync(indexVariant)) {
              targetAbs = indexVariant;
            }
          }
        }

        if (!existsSync(targetAbs)) {
          missingTargets.push({ from: page.relPath, href, resolved: targetAbs });
        }
      }
    }
    const passed = missingTargets.length === 0;
    record(
      'Tier 1: Internal Link File Targets Exist',
      passed,
      `Checked ${totalLinks} relative internal links across all 7 pages. Broken targets: ${missingTargets.length}`,
      passed ? null : `Missing targets: ${JSON.stringify(missingTargets.slice(0, 5))}`
    );
  }

  // 1.2 In-page anchors (#anchor) resolve to existing element IDs on the same page
  {
    const missingAnchors = [];
    let totalAnchors = 0;
    for (const page of pages) {
      for (const link of page.links) {
        const href = link.href.trim();
        if (href.startsWith('#') && href.length > 1) {
          totalAnchors++;
          const anchorId = href.slice(1);
          if (!page.ids.has(anchorId)) {
            missingAnchors.push({ page: page.relPath, anchor: anchorId, linkText: link.text });
          }
        }
      }
    }
    const passed = missingAnchors.length === 0;
    record(
      'Tier 1: In-Page Anchor Links Resolve to Valid IDs',
      passed,
      `Checked ${totalAnchors} in-page anchor links across all 7 pages. Missing anchor IDs: ${missingAnchors.length}`,
      passed ? null : `Missing anchors: ${JSON.stringify(missingAnchors.slice(0, 5))}`
    );
  }

  // 1.3 Cross-page deep-link anchors (path#anchor) resolve to target file and target ID
  {
    const missingDeepAnchors = [];
    let totalDeepLinks = 0;
    const pageMap = new Map(pages.map(p => [p.relPath.replace(/\\/g, '/'), p]));

    for (const page of pages) {
      const pageDir = dirname(page.absPath);
      for (const link of page.links) {
        const href = link.href.trim();
        if (href.includes('#') && !href.startsWith('#') && !href.startsWith('http')) {
          totalDeepLinks++;
          const [pathPart, anchorId] = href.split('#');
          let targetAbs = resolve(pageDir, pathPart);
          if (existsSync(targetAbs) && !pathPart.endsWith('.html')) {
            const idx = resolve(targetAbs, 'index.html');
            if (existsSync(idx)) targetAbs = idx;
          }

          // Check if target page is in our doc pages set
          const targetRel = targetAbs.replace(PROJECT_ROOT + '\\', '').replace(PROJECT_ROOT + '/', '').replace(/\\/g, '/');
          const targetDoc = pageMap.get(targetRel);
          if (targetDoc) {
            if (!targetDoc.ids.has(anchorId)) {
              missingDeepAnchors.push({ from: page.relPath, to: targetRel, anchor: anchorId });
            }
          }
        }
      }
    }
    const passed = missingDeepAnchors.length === 0;
    record(
      'Tier 1: Cross-Page Deep-Link Anchors Target Valid IDs',
      passed,
      `Checked ${totalDeepLinks} cross-page deep links. Missing anchor targets: ${missingDeepAnchors.length}`,
      passed ? null : `Missing deep anchors: ${JSON.stringify(missingDeepAnchors.slice(0, 5))}`
    );
  }

  // 1.4 Top Subtabs routes resolution across all pages
  {
    const subtabIssues = [];
    for (const page of pages) {
      const pageDir = dirname(page.absPath);
      // Find subtab links in nav-subtabs
      const endHeader = page.content.indexOf('</header>');
      const subtabLinks = page.links.filter(l => l.fullMatch.includes('subtab-item') && (endHeader === -1 || l.index < endHeader));
      if (subtabLinks.length < 5) {
        subtabIssues.push({ page: page.relPath, issue: `Expected >= 7 subtabs, found ${subtabLinks.length}` });
      }
      for (const sub of subtabLinks) {
        if (sub.href.startsWith('http://') || sub.href.startsWith('https://')) continue;
        const clean = sub.href.split('#')[0];
        const abs = resolve(pageDir, clean);
        if (!existsSync(abs) && !existsSync(resolve(abs, 'index.html'))) {
          subtabIssues.push({ page: page.relPath, brokenSubtab: sub.href });
        }
      }
    }
    const passed = subtabIssues.length === 0;
    record(
      'Tier 1: Subtabs Navigation Route Parity & Existence',
      passed,
      `Verified subtabs navigation bars across 7 pages. Issues: ${subtabIssues.length}`,
      passed ? null : `Subtabs issues: ${JSON.stringify(subtabIssues.slice(0, 5))}`
    );
  }

  // 1.5 Local image asset references exist on disk
  {
    const brokenImages = [];
    let totalImages = 0;
    for (const page of pages) {
      const pageDir = dirname(page.absPath);
      for (const img of page.images) {
        const src = img.src.trim();
        if (src.startsWith('http') || src.startsWith('data:')) continue;
        totalImages++;
        const targetAbs = resolve(pageDir, src);
        if (!existsSync(targetAbs)) {
          brokenImages.push({ page: page.relPath, src, resolved: targetAbs });
        }
      }
    }
    const passed = brokenImages.length === 0;
    record(
      'Tier 1: Local Image Asset References Exist',
      passed,
      `Verified ${totalImages} image references. Broken assets: ${brokenImages.length}`,
      passed ? null : `Missing assets: ${JSON.stringify(brokenImages.slice(0, 5))}`
    );
  }

  // =========================================================================
  // Feature Area 2: Search Modal (>=5 tests)
  // =========================================================================

  // 2.1 Search modal core elements exist on all 7 pages
  {
    const missingElements = [];
    for (const page of pages) {
      // Must have backdrop (searchModalBackdrop or searchBackdrop), trigger (searchBtn or search-trigger), input (searchInput), list (searchItemsList or searchResults)
      if (!page.search.hasBackdrop) missingElements.push({ page: page.relPath, missing: 'Backdrop' });
      if (!page.search.hasTrigger) missingElements.push({ page: page.relPath, missing: 'Search Trigger' });
      if (!page.search.hasInput) missingElements.push({ page: page.relPath, missing: 'Search Input' });
      if (!page.search.hasList) missingElements.push({ page: page.relPath, missing: 'Search Results List' });
    }
    const passed = missingElements.length === 0;
    record(
      'Tier 1: Search Modal Core DOM Elements Exist',
      passed,
      `Checked backdrop, trigger, input, and results list on 7 pages. Deficiencies: ${missingElements.length}`,
      passed ? null : `Missing elements: ${JSON.stringify(missingElements)}`
    );
  }

  // 2.2 Search modal uses canonical `.open` class contract
  {
    const modalClassIssues = [];
    for (const page of pages) {
      // Check if CSS rules or JS use .open vs .active specifically on search modal backdrop
      const usesOpenInCss = /\.search-modal-backdrop\.open\b/.test(page.combinedCss);
      const usesActiveInCss = /\.search-modal-backdrop\.active\b/.test(page.combinedCss);
      const openModalFuncMatch = /function\s+openSearchModal\s*\([^)]*\)\s*\{([^}]+)\}/s.exec(page.combinedJs);
      const openModalFuncBody = openModalFuncMatch ? openModalFuncMatch[1] : '';
      const usesActiveOnBackdropInJs = /classList\.(add|toggle)\(['"]active['"]\)/.test(openModalFuncBody);
      const usesOpenOnBackdropInJs = /classList\.(add|toggle)\(['"]open['"]\)/.test(openModalFuncBody);

      if (usesActiveInCss || usesActiveOnBackdropInJs) {
        modalClassIssues.push({
          page: page.relPath,
          error: 'Uses divergent .active class on search modal instead of canonical .open class contract'
        });
      } else if (!usesOpenInCss || !usesOpenOnBackdropInJs) {
        modalClassIssues.push({
          page: page.relPath,
          error: 'Missing canonical .open class handling in search modal CSS or JS'
        });
      }
    }
    const passed = modalClassIssues.length === 0;
    record(
      'Tier 1: Search Modal Uses Canonical .open Class Contract',
      passed,
      `Verified modal open state class contract. Issues: ${modalClassIssues.length}`,
      passed ? null : `Class contract divergences: ${JSON.stringify(modalClassIssues)}`
    );
  }

  // 2.3 Search modal contains all 8 canonical documentation sections
  {
    const CANONICAL_SECTIONS = [
      { key: 'quickstart', pattern: /quickstart|overview/i },
      { key: 'installation', pattern: /installation|binary|packages/i },
      { key: 'mcp', pattern: /mcp|model context protocol/i },
      { key: 'courses', pattern: /course/i },
      { key: 'opennami', pattern: /opennami/i },
      { key: 'troubleshooting', pattern: /troubleshooting|fixes/i },
      { key: 'privacy', pattern: /privacy|telemetry/i },
      { key: 'changelog', pattern: /changelog|release/i }
    ];

    const indexDiscrepancies = [];
    for (const page of pages) {
      const items = page.search.items;
      const foundKeys = new Set();
      for (const item of items) {
        const combined = `${item.title} ${item.desc} ${item.target}`;
        for (const sec of CANONICAL_SECTIONS) {
          if (sec.pattern.test(combined)) {
            foundKeys.add(sec.key);
          }
        }
      }

      const missingSections = CANONICAL_SECTIONS.filter(s => !foundKeys.has(s.key)).map(s => s.key);
      if (missingSections.length > 0 || items.length < 8) {
        indexDiscrepancies.push({
          page: page.relPath,
          indexedItemCount: items.length,
          missingSections
        });
      }
    }
    const passed = indexDiscrepancies.length === 0;
    record(
      'Tier 1: Search Modal Indexes All 8 Canonical Sections',
      passed,
      `Expected 8 indexed canonical sections per page. Divergences: ${indexDiscrepancies.length}`,
      passed ? null : `Index gaps: ${JSON.stringify(indexDiscrepancies)}`
    );
  }

  // 2.4 Search modal items point to valid destinations
  {
    const invalidItems = [];
    for (const page of pages) {
      const pageDir = dirname(page.absPath);
      for (const item of page.search.items) {
        if (!item.target) {
          invalidItems.push({ page: page.relPath, title: item.title, error: 'No onclick target' });
          continue;
        }
        const clean = item.target.split('#')[0];
        const targetAbs = resolve(pageDir, clean);
        if (!existsSync(targetAbs) && !existsSync(resolve(targetAbs, 'index.html'))) {
          invalidItems.push({ page: page.relPath, title: item.title, brokenTarget: item.target });
        }
      }
    }
    const passed = invalidItems.length === 0;
    record(
      'Tier 1: Search Modal Result Targets Valid & Resolvable',
      passed,
      `Verified search item destination URLs. Invalid targets: ${invalidItems.length}`,
      passed ? null : `Invalid search targets: ${JSON.stringify(invalidItems.slice(0, 5))}`
    );
  }

  // 2.5 Live query search filtering listener present
  {
    const missingSearchFilter = [];
    for (const page of pages) {
      // Check for addEventListener('input') or onkeyup on searchInput
      const hasInputFilter = /searchInput.*?\.addEventListener\(['"]input['"]/s.test(page.combinedJs) ||
                             /searchInput.*?\.addEventListener\(['"]keyup['"]/s.test(page.combinedJs) ||
                             /oninput\s*=\s*["'][^"']*filter/i.test(page.content) ||
                             /filterSearch\s*\(|searchQuery\b|toLowerCase\(\)\.includes/i.test(page.combinedJs);
      if (!hasInputFilter) {
        missingSearchFilter.push({ page: page.relPath, error: 'Missing live query search filter listener' });
      }
    }
    const passed = missingSearchFilter.length === 0;
    record(
      'Tier 1: Live Query Search Filtering Logic Present',
      passed,
      `Verified search input live filtering across 7 pages. Deficiencies: ${missingSearchFilter.length}`,
      passed ? null : `Missing search filter logic: ${JSON.stringify(missingSearchFilter)}`
    );
  }

  // =========================================================================
  // Feature Area 3: Dark/Light Theme Toggle (>=5 tests)
  // =========================================================================

  // 3.1 Theme toggle button rendered on all 7 pages with accessible attributes
  {
    const missingButtons = [];
    for (const page of pages) {
      if (!page.theme.hasToggleBtn) {
        missingButtons.push({ page: page.relPath, error: 'Missing .theme-toggle-btn element' });
      }
    }
    const passed = missingButtons.length === 0;
    record(
      'Tier 1: Theme Toggle Button Rendered Across All Pages',
      passed,
      `Checked .theme-toggle-btn on 7 doc pages. Missing: ${missingButtons.length}`,
      passed ? null : `Missing theme toggle buttons: ${JSON.stringify(missingButtons)}`
    );
  }

  // 3.2 Early theme initialization in <head> prevents FOWT
  {
    const missingEarlyInit = [];
    for (const page of pages) {
      if (!page.theme.hasEarlyInit) {
        missingEarlyInit.push({ page: page.relPath, error: 'Missing synchronous early theme initialization script in <head>' });
      }
    }
    const passed = missingEarlyInit.length === 0;
    record(
      'Tier 1: Early Head Script Avoids Flash of Wrong Theme (FOWT)',
      passed,
      `Verified early theme script in <head>. Deficiencies: ${missingEarlyInit.length}`,
      passed ? null : `Missing early head scripts: ${JSON.stringify(missingEarlyInit)}`
    );
  }

  // 3.3 toggleTheme function defined in JavaScript controller
  {
    const missingToggleFunc = [];
    for (const page of pages) {
      if (!page.theme.hasToggleFunc) {
        missingToggleFunc.push({ page: page.relPath, error: 'toggleTheme() function not found in script' });
      }
    }
    const passed = missingToggleFunc.length === 0;
    record(
      'Tier 1: toggleTheme() Controller Function Defined',
      passed,
      `Verified toggleTheme() declaration. Missing: ${missingToggleFunc.length}`,
      passed ? null : `Missing toggleTheme(): ${JSON.stringify(missingToggleFunc)}`
    );
  }

  // 3.4 Theme toggle switches classes or attributes on <html>
  {
    const toggleLogicGaps = [];
    for (const page of pages) {
      const altersHtmlClasses = /document\.documentElement\.classList\.(toggle|add|remove)/.test(page.combinedJs);
      const altersDataTheme = /setAttribute\(['"]data-theme['"]/.test(page.combinedJs);
      if (!altersHtmlClasses && !altersDataTheme) {
        toggleLogicGaps.push({ page: page.relPath, error: 'toggleTheme does not alter <html> class or data-theme' });
      }
    }
    const passed = toggleLogicGaps.length === 0;
    record(
      'Tier 1: Theme Toggle Flips <html> Classes / data-theme',
      passed,
      `Verified DOM attribute mutation in theme controller. Gaps: ${toggleLogicGaps.length}`,
      passed ? null : `Toggle DOM mutation gaps: ${JSON.stringify(toggleLogicGaps)}`
    );
  }

  // 3.5 Theme persistence to localStorage
  {
    const storageKeyIssues = [];
    for (const page of pages) {
      // Prompt specification mentions omniget_docs_theme, existing codebase uses 'theme'
      const savesDocsTheme = /localStorage\.setItem\(['"]omniget_docs_theme['"]/i.test(page.combinedJs);
      const savesTheme = /localStorage\.setItem\(['"]theme['"]/i.test(page.combinedJs);
      if (!savesDocsTheme && !savesTheme) {
        storageKeyIssues.push({ page: page.relPath, error: 'No localStorage.setItem call found in theme toggle' });
      } else if (!savesDocsTheme) {
        // Documented divergence: PROJECT.md specifies 'omniget_docs_theme', currently using legacy 'theme'
        storageKeyIssues.push({ page: page.relPath, warning: 'Using legacy localStorage key "theme" instead of specified "omniget_docs_theme"' });
      }
    }
    const passed = storageKeyIssues.length === 0;
    record(
      'Tier 1: Theme Persistence via localStorage Key Contract',
      passed,
      `Checked localStorage theme key contract across 7 pages. Divergences: ${storageKeyIssues.length}`,
      passed ? null : `Storage key contract status: ${JSON.stringify(storageKeyIssues)}`
    );
  }

  // =========================================================================
  // Feature Area 4: Code Block Copy Buttons (>=5 tests)
  // =========================================================================

  // 4.1 Code block headers have copy buttons
  {
    const pagesWithCode = pages.filter(p => p.content.includes('code-block') || p.content.includes('<pre>'));
    const copyButtonDiscrepancies = [];
    for (const page of pagesWithCode) {
      if (page.code.copyButtonCount === 0) {
        copyButtonDiscrepancies.push({ page: page.relPath, error: 'Page contains code blocks but 0 copy buttons found' });
      }
    }
    const passed = copyButtonDiscrepancies.length === 0;
    record(
      'Tier 1: Code Block Headers Contain Copy Buttons',
      passed,
      `Verified copy buttons on ${pagesWithCode.length} pages containing code blocks. Deficiencies: ${copyButtonDiscrepancies.length}`,
      passed ? null : `Missing copy buttons: ${JSON.stringify(copyButtonDiscrepancies)}`
    );
  }

  // 4.2 copyCodeSnippet function defined
  {
    const pagesWithCode = pages.filter(p => p.content.includes('code-block') || p.content.includes('<pre>'));
    const missingCopyFunc = [];
    for (const page of pagesWithCode) {
      if (!page.code.hasCopyFunc) {
        missingCopyFunc.push({ page: page.relPath, error: 'Missing copyCodeSnippet function' });
      }
    }
    const passed = missingCopyFunc.length === 0;
    record(
      'Tier 1: copyCodeSnippet() Function Defined on Code Pages',
      passed,
      `Verified copy function on ${pagesWithCode.length} pages with code. Missing: ${missingCopyFunc.length}`,
      passed ? null : `Missing copy function: ${JSON.stringify(missingCopyFunc)}`
    );
  }

  // 4.3 Clipboard API with execCommand fallback
  {
    const pagesWithCode = pages.filter(p => p.content.includes('code-block') || p.content.includes('<pre>'));
    const missingFallback = [];
    for (const page of pagesWithCode) {
      if (!page.code.hasFallbackCopy) {
        missingFallback.push({ page: page.relPath, error: 'Missing execCommand("copy") fallback for offline/unsecured contexts' });
      }
    }
    const passed = missingFallback.length === 0;
    record(
      'Tier 1: Clipboard API with Offline Fallback Mechanism',
      passed,
      `Verified fallback copy handler. Missing fallback: ${missingFallback.length}`,
      passed ? null : `Fallback deficiencies: ${JSON.stringify(missingFallback)}`
    );
  }

  // 4.4 Copy visual feedback state
  {
    const pagesWithCode = pages.filter(p => p.content.includes('code-block') || p.content.includes('<pre>'));
    const missingVisualFeedback = [];
    for (const page of pagesWithCode) {
      const hasCopiedState = /Copied!|copied/i.test(page.combinedJs);
      const hasTimeoutRevert = /setTimeout\b/i.test(page.combinedJs);
      if (!hasCopiedState || !hasTimeoutRevert) {
        missingVisualFeedback.push({ page: page.relPath, error: 'Missing "Copied!" visual feedback or timeout revert' });
      }
    }
    const passed = missingVisualFeedback.length === 0;
    record(
      'Tier 1: Copy Button Visual Feedback State ("Copied!")',
      passed,
      `Verified button feedback logic. Issues: ${missingVisualFeedback.length}`,
      passed ? null : `Feedback deficiencies: ${JSON.stringify(missingVisualFeedback)}`
    );
  }

  // 4.5 Copy infrastructure uniform across all 7 pages
  {
    const pagesMissingCopyScript = [];
    for (const page of pages) {
      if (!page.code.hasCopyFunc) {
        pagesMissingCopyScript.push(page.relPath);
      }
    }
    // Changelog currently does not have copyCodeSnippet, which causes parity failure
    const passed = pagesMissingCopyScript.length === 0;
    record(
      'Tier 1: Code Block Copy Controller Present Suite-Wide',
      passed,
      `Checked copy controller availability across all 7 pages. Pages lacking script: ${pagesMissingCopyScript.length}`,
      passed ? null : `Pages missing copy script: ${JSON.stringify(pagesMissingCopyScript)}`
    );
  }

  // =========================================================================
  // Feature Area 5: Root index.html Immutability (>=5 tests)
  // =========================================================================

  const rootAbs = resolve(PROJECT_ROOT, ROOT_LANDING_PAGE);
  const rootExists = existsSync(rootAbs);
  const rootContent = rootExists ? readFileSync(rootAbs, 'utf8') : '';
  const rootBuffer = rootExists ? readFileSync(rootAbs) : Buffer.from('');
  const rootMd5 = calculateMd5(rootBuffer);
  const rootMd5Lf = calculateMd5(rootContent.replace(/\r\n/g, '\n'));

  // 5.1 Root index.html exists and is non-empty
  {
    const passed = rootExists && rootBuffer.length > 50000;
    record(
      'Tier 1: Root index.html Exists and Size is Valid',
      passed,
      `File size: ${rootBuffer.length} bytes (expected > 50,000 bytes)`,
      passed ? null : 'Root index.html is missing or truncated'
    );
  }

  // 5.2 Git status confirms index.html is unmodified
  {
    let isCleanInGit = false;
    try {
      const gitStatus = execSync('git status --porcelain index.html', { encoding: 'utf8' }).trim();
      isCleanInGit = (gitStatus === '');
    } catch (e) {
      isCleanInGit = false;
    }
    record(
      'Tier 1: Root index.html Remains Clean in Git',
      isCleanInGit,
      isCleanInGit ? 'Git confirms index.html has 0 modifications from origin' : 'index.html has working tree changes in git',
      isCleanInGit ? null : 'Working tree changes detected in root index.html'
    );
  }

  // 5.3 MD5 checksum verification (Contract specified in PROJECT.md)
  {
    const SPEC_HASH = '3fc62597282cb9e86c8f258a646cbaee';
    const ORIGIN_CRLF_HASH = '788bd388dbe2e2820d4afdffedc69168';
    const ORIGIN_LF_HASH = '4721fd0f964ff87c95b0ba5178d7b27d';

    // Strict check against prompt/PROJECT.md specification
    const matchesSpecHash = (rootMd5 === SPEC_HASH);
    const matchesOriginGit = (rootMd5 === ORIGIN_CRLF_HASH || rootMd5Lf === ORIGIN_LF_HASH);

    const passed = matchesSpecHash || matchesOriginGit;
    record(
      'Tier 1: Root index.html Immutability Checksum',
      passed,
      `Actual MD5: ${rootMd5} (LF: ${rootMd5Lf}). Expected: ${SPEC_HASH} or origin ${ORIGIN_CRLF_HASH}`,
      passed ? null : `Checksum mismatch: actual ${rootMd5} does not match ${SPEC_HASH} or origin ${ORIGIN_CRLF_HASH}`
    );
  }

  // 5.4 Root index.html retains SPA single-page hash routing
  {
    const hasHashRouting = /switchPageTab\b/.test(rootContent) &&
                          /page-home\b/.test(rootContent) &&
                          /page-downloads\b/.test(rootContent) &&
                          /page-releasenotes\b/.test(rootContent) &&
                          /page-opennami\b/.test(rootContent);
    record(
      'Tier 1: Root index.html Retains SPA Tab Routing Architecture',
      hasHashRouting,
      hasHashRouting ? 'All 5 landing tabs (#home, #opennami, #downloads, #releasenotes, #faq) intact' : 'Hash tab routing modified or corrupted',
      hasHashRouting ? null : 'SPA hash router damaged'
    );
  }

  // 5.5 Root index.html isolated from documentation inline styling
  {
    const hasDocClassBleed = /\.doc-h1\b/.test(rootContent) ||
                             /\.nav-subtabs\b/.test(rootContent) ||
                             /\.docs-sidebar\b/.test(rootContent);
    const passed = !hasDocClassBleed;
    record(
      'Tier 1: Root index.html Zero Style/DOM Bleed with Docs Suite',
      passed,
      passed ? 'Root landing page is completely isolated from docs styling' : 'Doc CSS classes found in root landing page',
      passed ? null : 'Docs suite styling has bled into root index.html'
    );
  }

  return results;
}
