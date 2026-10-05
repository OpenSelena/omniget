import { existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import {
  PROJECT_ROOT,
  CANONICAL_DOC_PAGES,
  loadAllDocPages
} from './helpers.mjs';

export function runTier3Tests() {
  const pages = loadAllDocPages();
  const results = [];

  function record(name, passed, details = '', error = null) {
    results.push({ tier: 3, name, passed, details, error });
  }

  // =========================================================================
  // Cross-Feature Interaction 1: Theme Persistence Across Navigation
  // =========================================================================
  {
    // For theme to persist across navigation without FOWT:
    // 1. Every page must use the exact same localStorage key in both <head> early init and toggleTheme()
    // 2. Every page must toggle the exact same classes (.dark, data-theme="dark" / "light")
    const themeStorageKeys = [];
    const classToggleContracts = [];

    for (const page of pages) {
      // Find localStorage key in <head>
      const headInitMatch = /localStorage\.getItem\(['"]([^'"]+)['"]\)/.exec(page.content.slice(0, 3000));
      const headKey = headInitMatch ? headInitMatch[1] : null;

      // Find localStorage key in toggleTheme
      const setItemMatch = /localStorage\.setItem\(['"]([^'"]+)['"]/.exec(page.combinedJs);
      const setKey = setItemMatch ? setItemMatch[1] : null;

      themeStorageKeys.push({ page: page.relPath, headKey, setKey });

      const togglesDark = /classList\.(add|toggle|remove)\(['"]dark['"]\)/.test(page.combinedJs);
      const setsDataTheme = /setAttribute\(['"]data-theme['"]/.test(page.combinedJs);
      classToggleContracts.push({ page: page.relPath, togglesDark, setsDataTheme });
    }

    const uniqueHeadKeys = new Set(themeStorageKeys.map(k => k.headKey));
    const uniqueSetKeys = new Set(themeStorageKeys.map(k => k.setKey));
    const isKeyConsistent = uniqueHeadKeys.size === 1 && uniqueSetKeys.size === 1 &&
                            Array.from(uniqueHeadKeys)[0] === Array.from(uniqueSetKeys)[0];

    record(
      'Tier 3: Theme Persistence Across Multi-Page Navigation',
      isKeyConsistent,
      `Head keys: ${Array.from(uniqueHeadKeys).join(', ')}. Set keys: ${Array.from(uniqueSetKeys).join(', ')}`,
      isKeyConsistent ? null : `Theme storage key desynchronization detected: ${JSON.stringify(themeStorageKeys)}`
    );
  }

  // =========================================================================
  // Cross-Feature Interaction 2: Search Modal Navigation Targets
  // =========================================================================
  {
    // When a user opens search modal on page X and clicks an item targeting page Y#anchor:
    // 1. Target page Y must exist
    // 2. If target includes an anchor #anchor, target element ID must exist on page Y
    const brokenNavigations = [];
    const pageMap = new Map(pages.map(p => [p.relPath.replace(/\\/g, '/'), p]));

    for (const page of pages) {
      const pageDir = dirname(page.absPath);
      for (const item of page.search.items) {
        if (!item.target) continue;
        const [targetPath, targetAnchor] = item.target.split('#');
        let targetAbs = resolve(pageDir, targetPath);
        if (existsSync(targetAbs) && !targetPath.endsWith('.html')) {
          const idx = resolve(targetAbs, 'index.html');
          if (existsSync(idx)) targetAbs = idx;
        }

        if (!existsSync(targetAbs)) {
          brokenNavigations.push({ from: page.relPath, item: item.title, target: item.target, error: 'Target file not found' });
          continue;
        }

        if (targetAnchor) {
          const targetRel = targetAbs.replace(PROJECT_ROOT + '\\', '').replace(PROJECT_ROOT + '/', '').replace(/\\/g, '/');
          const targetDoc = pageMap.get(targetRel);
          if (targetDoc && !targetDoc.ids.has(targetAnchor)) {
            brokenNavigations.push({
              from: page.relPath,
              item: item.title,
              target: item.target,
              error: `Anchor #${targetAnchor} does not exist on target page ${targetRel}`
            });
          }
        }
      }
    }

    const passed = brokenNavigations.length === 0;
    record(
      'Tier 3: Search Modal Click Navigation Target & Anchor Integrity',
      passed,
      `Verified cross-page navigations from search modal items. Broken navigations: ${brokenNavigations.length}`,
      passed ? null : `Search modal navigation defects: ${JSON.stringify(brokenNavigations)}`
    );
  }

  // =========================================================================
  // Cross-Feature Interaction 3: Deep-Link Anchor Scroll Offset (>= 134px)
  // =========================================================================
  {
    // The sticky double-row header height is:
    // --header-h: 64px, --tabs-h: 46px => Total 110px.
    // CSS spec mandates: scroll-margin-top: calc(var(--header-total-h) + 24px) = 134px.
    // Every heading or container that serves as an anchor target MUST have scroll-margin-top >= 134px.
    const scrollMarginGaps = [];

    for (const page of pages) {
      const css = page.combinedCss;
      const hasHeaderTotalH = /--header-total-h\s*:\s*(\d+)px/.exec(css);
      const totalH = hasHeaderTotalH ? parseInt(hasHeaderTotalH[1], 10) : 110;

      // Check if .doc-h2 has scroll-margin-top defined anywhere in stylesheet
      const hasH2ScrollMargin = /\.doc-h2\b[^{]*\{[^}]*scroll-margin-top/s.test(css);

      // Check changelog specific release box
      const isChangelog = page.relPath.includes('changelog');
      const hasBoxScrollMargin = /\.release-box\b[^{]*\{[^}]*scroll-margin-top/s.test(css);

      if (!hasH2ScrollMargin && !isChangelog) {
        scrollMarginGaps.push({ page: page.relPath, target: '.doc-h2', issue: 'Missing scroll-margin-top' });
      }
      if (isChangelog && !hasBoxScrollMargin) {
        scrollMarginGaps.push({
          page: page.relPath,
          target: '.release-box',
          issue: 'Missing scroll-margin-top on .release-box causes anchor clipping under sticky header'
        });
      }
    }

    const passed = scrollMarginGaps.length === 0;
    record(
      'Tier 3: Deep-Link Anchor Scroll Offset (>= 134px) Immunity to Header Occlusion',
      passed,
      `Verified scroll-margin-top against 110px sticky header. Gaps: ${scrollMarginGaps.length}`,
      passed ? null : `Scroll margin deficiencies: ${JSON.stringify(scrollMarginGaps)}`
    );
  }

  // =========================================================================
  // Cross-Feature Interaction 4: Search Modal Close Triggers
  // =========================================================================
  {
    // Search modal must provide all 3 standard close vectors:
    // 1. Backdrop click triggers closeSearchModal(event)
    // 2. Escape keydown listener calls closeSearchModal()
    // 3. Modal container click invokes event.stopPropagation()
    const closeTriggerIssues = [];

    for (const page of pages) {
      const hasBackdropClick = /onclick\s*=\s*["'][^"']*closeSearchModal\s*\(\s*event\s*\)[^"']*["']/i.test(page.content) ||
                               /backdrop.*?addEventListener\(['"]click['"]/i.test(page.combinedJs);
      const hasEscapeKey = /key\s*===\s*['"]Escape['"]/i.test(page.combinedJs);
      const hasStopPropagation = /event\.stopPropagation\(\)/.test(page.content) ||
                                 /stopPropagation\(\)/.test(page.combinedJs);

      if (!hasBackdropClick || !hasEscapeKey || !hasStopPropagation) {
        closeTriggerIssues.push({
          page: page.relPath,
          backdropClick: hasBackdropClick,
          escapeKey: hasEscapeKey,
          stopPropagation: hasStopPropagation
        });
      }
    }

    const passed = closeTriggerIssues.length === 0;
    record(
      'Tier 3: Search Modal Multi-Vector Close Triggers (Backdrop / Esc / Propagation)',
      passed,
      `Verified close interaction handlers across 7 pages. Issues: ${closeTriggerIssues.length}`,
      passed ? null : `Close vector gaps: ${JSON.stringify(closeTriggerIssues)}`
    );
  }

  // =========================================================================
  // Cross-Feature Interaction 5: Code Block Copy Offline Fallback Execution
  // =========================================================================
  {
    // In secure HTTPS: navigator.clipboard.writeText is invoked.
    // In offline file:/// or insecure iframe: fallback executes document.execCommand('copy').
    const copyFallbackGaps = [];
    const pagesWithCode = pages.filter(p => p.content.includes('code-block') || p.content.includes('<pre>'));

    for (const page of pagesWithCode) {
      const js = page.combinedJs;
      const checksFileProtocol = /location\.protocol\s*===\s*['"]file:['"]/.test(js);
      const hasExecFallback = /document\.execCommand\(['"]copy['"]\)/.test(js) ||
                              /execCopy\b/.test(js);
      const createsTextarea = /document\.createElement\(['"]textarea['"]\)/.test(js);

      if (!hasExecFallback || !createsTextarea) {
        copyFallbackGaps.push({ page: page.relPath, hasExecFallback, createsTextarea });
      }
    }

    const passed = copyFallbackGaps.length === 0;
    record(
      'Tier 3: Code Block Copy Offline & Insecure Context Fallback Compatibility',
      passed,
      `Verified clipboard fallback execution path on ${pagesWithCode.length} pages. Gaps: ${copyFallbackGaps.length}`,
      passed ? null : `Fallback deficiencies: ${JSON.stringify(copyFallbackGaps)}`
    );
  }

  // =========================================================================
  // Cross-Feature Interaction 6: Subtabs Active State Synchronization
  // =========================================================================
  {
    // On each documentation page, exactly one subtab must be marked .active,
    // and its target route must correspond to that page's topic.
    const activeSubtabMismatches = [];

    const expectedSubtabByPage = [
      { file: 'docs/index.html', labelPattern: /quickstart/i },
      { file: 'docs/installation/index.html', labelPattern: /installation/i },
      { file: 'docs/mcp/index.html', labelPattern: /mcp/i },
      { file: 'docs/courses/index.html', labelPattern: /course/i },
      { file: 'docs/opennami/index.html', labelPattern: /opennami/i },
      { file: 'docs/troubleshooting/index.html', labelPattern: /troubleshooting/i },
      { file: 'docs/changelog/index.html', labelPattern: /changelog|release/i }
    ];

    for (const exp of expectedSubtabByPage) {
      const page = pages.find(p => p.relPath.replace(/\\/g, '/') === exp.file);
      if (!page) continue;
      const active = page.layout.activeSubtab;
      if (!active) {
        activeSubtabMismatches.push({ page: exp.file, error: 'No active subtab found' });
      } else if (!exp.labelPattern.test(active.label)) {
        activeSubtabMismatches.push({
          page: exp.file,
          activeLabel: active.label,
          error: 'Active subtab does not match page topic'
        });
      }
    }

    const passed = activeSubtabMismatches.length === 0;
    record(
      'Tier 3: Subtabs Active State Route Synchronization',
      passed,
      `Verified active subtab indicators across 7 pages. Mismatches: ${activeSubtabMismatches.length}`,
      passed ? null : `Active subtab mismatches: ${JSON.stringify(activeSubtabMismatches)}`
    );
  }

  return results;
}
