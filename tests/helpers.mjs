import { readFileSync, existsSync, statSync } from 'node:fs';
import { resolve, join, dirname, normalize } from 'node:path';
import { createHash } from 'node:crypto';

export const PROJECT_ROOT = resolve('.');
export const DOCS_ROOT = resolve('docs');

export const CANONICAL_DOC_PAGES = [
  'docs/index.html',
  'docs/installation/index.html',
  'docs/mcp/index.html',
  'docs/courses/index.html',
  'docs/opennami/index.html',
  'docs/troubleshooting/index.html',
  'docs/changelog/index.html'
];

export const ROOT_LANDING_PAGE = 'index.html';

export function calculateMd5(content) {
  const buf = Buffer.isBuffer(content) ? content : Buffer.from(content, 'utf8');
  return createHash('md5').update(buf).digest('hex');
}

/**
 * Robust, lightweight HTML/CSS/JS parser tailored for opaque-box doc audit.
 */
export function parseDocPage(relPath) {
  const absPath = resolve(PROJECT_ROOT, relPath);
  if (!existsSync(absPath)) {
    throw new Error(`File not found: ${absPath}`);
  }
  const content = readFileSync(absPath, 'utf8');
  const buffer = readFileSync(absPath);

  // Extract all element IDs
  const idRegex = /\bid\s*=\s*["']([^"']+)["']/gi;
  const ids = new Set();
  let match;
  while ((match = idRegex.exec(content)) !== null) {
    ids.add(match[1]);
  }

  // Extract all <a href="..."> links
  const aRegex = /<a\b([^>]*)>(.*?)<\/a>/gis;
  const links = [];
  while ((match = aRegex.exec(content)) !== null) {
    const attrs = match[1];
    const text = match[2].replace(/<[^>]+>/g, '').trim();
    const hrefMatch = /href\s*=\s*["']([^"']*)["']/i.exec(attrs);
    if (hrefMatch) {
      links.push({
        href: hrefMatch[1],
        text,
        fullMatch: match[0],
        index: match.index
      });
    }
  }

  // Extract all <img src="..."> tags
  const imgRegex = /<img\b([^>]*)>/gi;
  const images = [];
  while ((match = imgRegex.exec(content)) !== null) {
    const attrs = match[1];
    const srcMatch = /src\s*=\s*["']([^"']*)["']/i.exec(attrs);
    const altMatch = /alt\s*=\s*["']([^"']*)["']/i.exec(attrs);
    if (srcMatch) {
      images.push({
        src: srcMatch[1],
        alt: altMatch ? altMatch[1] : '',
        index: match.index
      });
    }
  }

  // Extract Headings (h1 to h6)
  const headingRegex = /<(h[1-6])\b([^>]*)>(.*?)<\/\1>/gis;
  const headings = [];
  while ((match = headingRegex.exec(content)) !== null) {
    const level = match[1].toLowerCase();
    const attrs = match[2];
    const rawInner = match[3];
    const text = rawInner.replace(/<[^>]+>/g, '').trim();
    const idMatch = /id\s*=\s*["']([^"']+)["']/i.exec(attrs);
    const classMatch = /class\s*=\s*["']([^"']+)["']/i.exec(attrs);
    headings.push({
      level,
      id: idMatch ? idMatch[1] : null,
      className: classMatch ? classMatch[1] : '',
      text,
      rawInner,
      index: match.index
    });
  }

  // Extract release box articles (specifically used in changelog)
  const articleRegex = /<article\b([^>]*)>(.*?)<\/article>/gis;
  const articles = [];
  while ((match = articleRegex.exec(content)) !== null) {
    const attrs = match[1];
    const idMatch = /id\s*=\s*["']([^"']+)["']/i.exec(attrs);
    const classMatch = /class\s*=\s*["']([^"']+)["']/i.exec(attrs);
    articles.push({
      id: idMatch ? idMatch[1] : null,
      className: classMatch ? classMatch[1] : '',
      index: match.index
    });
  }

  // Extract style blocks
  const styleRegex = /<style\b[^>]*>(.*?)<\/style>/gis;
  const styles = [];
  while ((match = styleRegex.exec(content)) !== null) {
    styles.push(match[1]);
  }
  const combinedCss = styles.join('\n');

  // Extract script blocks
  const scriptRegex = /<script\b[^>]*>(.*?)<\/script>/gis;
  const scripts = [];
  while ((match = scriptRegex.exec(content)) !== null) {
    scripts.push(match[1]);
  }
  const combinedJs = scripts.join('\n');

  // Extract meta tags
  const metaRegex = /<meta\b([^>]*)>/gi;
  const metas = [];
  while ((match = metaRegex.exec(content)) !== null) {
    const attrs = match[1];
    const nameMatch = /name\s*=\s*["']([^"']+)["']/i.exec(attrs);
    const propMatch = /property\s*=\s*["']([^"']+)["']/i.exec(attrs);
    const contentMatch = /content\s*=\s*["']([^"']+)["']/i.exec(attrs);
    metas.push({
      name: nameMatch ? nameMatch[1] : (propMatch ? propMatch[1] : ''),
      content: contentMatch ? contentMatch[1] : ''
    });
  }

  // Extract prose text stripped of scripts, styles, and tags
  let strippedText = content
    .replace(/<style\b[^>]*>.*?<\/style>/gis, ' ')
    .replace(/<script\b[^>]*>.*?<\/script>/gis, ' ')
    .replace(/<!--.*?-->/gis, ' ')
    .replace(/<[^>]+>/g, ' ')
    .replace(/&nbsp;/g, ' ')
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&rarr;/g, '->')
    .replace(/&larr;/g, '<-')
    .replace(/\s+/g, ' ');

  // Extract Search Modal specific structure
  // Check for search backdrop (#searchModalBackdrop or #searchBackdrop)
  const hasSearchBackdrop = /id\s*=\s*["'](searchModalBackdrop|searchBackdrop)["']/i.test(content) ||
                            /class\s*=\s*["'][^"']*search-modal-backdrop[^"']*["']/i.test(content);
  const backdropMatch = /<div\b[^>]*id\s*=\s*["'](searchModalBackdrop|searchBackdrop)["'][^>]*>/i.exec(content) ||
                        /<div\b[^>]*class\s*=\s*["'][^"']*search-modal-backdrop[^"']*["'][^>]*>/i.exec(content);
  const backdropId = backdropMatch ? (/id\s*=\s*["']([^"']+)["']/i.exec(backdropMatch[0])?.[1] || null) : null;

  // Check search trigger button
  const hasSearchTrigger = /id\s*=\s*["']searchBtn["']/i.test(content) ||
                           /class\s*=\s*["'][^"']*search-trigger[^"']*["']/i.test(content);

  // Check search input
  const hasSearchInput = /id\s*=\s*["']searchInput["']/i.test(content) ||
                         /class\s*=\s*["'][^"']*search-modal-input[^"']*["']/i.test(content);

  // Check search items list container
  const hasSearchList = /id\s*=\s*["'](searchItemsList|searchResults)["']/i.test(content) ||
                        /class\s*=\s*["'][^"']*search-results-list[^"']*["']/i.test(content);

  // Extract search result items
  const searchItemRegex = /<li\b[^>]*class\s*=\s*["'][^"']*search-result-item[^"']*["'][^>]*>(.*?)<\/li>/gis;
  const searchItems = [];
  while ((match = searchItemRegex.exec(content)) !== null) {
    const full = match[0];
    const inner = match[1];
    const onclickMatch = /onclick\s*=\s*["']location\.href\s*=\s*['"]([^'"]+)['"]\s*["']/i.exec(full) ||
                         /onclick\s*=\s*["'][^"']*href\s*=\s*['"]([^'"]+)['"]\s*["']/i.exec(full);
    const titleMatch = /<span\b[^>]*class\s*=\s*["'][^"']*search-result-title[^"']*["'][^>]*>(.*?)<\/span>/is.exec(inner) ||
                       /<div\b[^>]*class\s*=\s*["'][^"']*search-result-title[^"']*["'][^>]*>(.*?)<\/div>/is.exec(inner) ||
                       /<strong\b[^>]*>(.*?)<\/strong>/is.exec(inner);
    const descMatch = /<span\b[^>]*class\s*=\s*["'][^"']*search-result-desc[^"']*["'][^>]*>(.*?)<\/span>/is.exec(inner) ||
                      /<div\b[^>]*class\s*=\s*["'][^"']*search-result-desc[^"']*["'][^>]*>(.*?)<\/div>/is.exec(inner);
    searchItems.push({
      target: onclickMatch ? onclickMatch[1] : null,
      title: titleMatch ? titleMatch[1].replace(/<[^>]+>/g, '').trim() : '',
      desc: descMatch ? descMatch[1].replace(/<[^>]+>/g, '').trim() : '',
      fullHtml: full
    });
  }

  // Extract code blocks and copy buttons
  const codeBlockRegex = /<div\b[^>]*class\s*=\s*["'][^"']*code-block[^"']*["'][^>]*>(.*?)<\/div>\s*<\/div>/gis;
  const codeBlocks = [];
  const copyBtnRegex = /<button\b[^>]*class\s*=\s*["'][^"']*(copy-btn|btn-copy)[^"']*["'][^>]*>/gi;
  let copyButtonCount = 0;
  while ((match = copyBtnRegex.exec(content)) !== null) {
    copyButtonCount++;
  }

  // Pagination grid
  const pageNavGridIndex = content.indexOf('class="page-nav-grid"');
  let paginationPrev = null;
  let paginationNext = null;
  let hasPageNavGrid = false;
  if (pageNavGridIndex !== -1) {
    hasPageNavGrid = true;
    const gridChunk = content.slice(pageNavGridIndex, pageNavGridIndex + 1500);
    const cardRegex = /<a\b[^>]*href\s*=\s*["']([^"']+)["'][^>]*class\s*=\s*["'][^"']*page-nav-card[^"']*["'][^>]*>(.*?)<\/a>/gis;
    let cardMatch;
    while ((cardMatch = cardRegex.exec(gridChunk)) !== null) {
      const href = cardMatch[1];
      const inner = cardMatch[2];
      if (/Previous/i.test(inner)) {
        paginationPrev = href;
      } else if (/Next/i.test(inner)) {
        paginationNext = href;
      }
    }
  }

  // Breadcrumbs
  const hasBreadcrumbs = /class\s*=\s*["'][^"']*breadcrumbs[^"']*["']/i.test(content);

  // Subtabs container
  const subtabsContainerMatch = /id\s*=\s*["']topSubtabs["']/i.test(content);
  let activeSubtab = null;
  const topSubtabsIdx = content.indexOf('id="topSubtabs"');
  let subtabsChunk = '';
  if (topSubtabsIdx !== -1) {
    const endHeader = content.indexOf('</header>', topSubtabsIdx);
    subtabsChunk = content.slice(topSubtabsIdx, endHeader !== -1 ? endHeader : topSubtabsIdx + 12000);
  } else {
    const subtabsClassIdx = content.indexOf('nav-subtabs');
    if (subtabsClassIdx !== -1) {
      const endHeader = content.indexOf('</header>', subtabsClassIdx);
      subtabsChunk = content.slice(subtabsClassIdx, endHeader !== -1 ? endHeader : subtabsClassIdx + 12000);
    }
  }
  if (subtabsChunk) {
    const aRegex = /<a\b([^>]*)>(.*?)<\/a>/gis;
    let aMatch;
    while ((aMatch = aRegex.exec(subtabsChunk)) !== null) {
      const attrs = aMatch[1];
      const inner = aMatch[2];
      const classMatch = /class\s*=\s*["']([^"']+)["']/i.exec(attrs);
      if (classMatch && /\bactive\b/.test(classMatch[1])) {
        const hrefMatch = /href\s*=\s*["']([^"']+)["']/i.exec(attrs);
        activeSubtab = {
          href: hrefMatch ? hrefMatch[1] : '',
          label: inner.replace(/<[^>]+>/g, '').trim()
        };
        break;
      }
    }
  }

  return {
    relPath,
    absPath,
    content,
    buffer,
    ids,
    links,
    images,
    headings,
    articles,
    styles,
    combinedCss,
    scripts,
    combinedJs,
    metas,
    strippedText,
    search: {
      hasBackdrop: hasSearchBackdrop,
      backdropId,
      hasTrigger: hasSearchTrigger,
      hasInput: hasSearchInput,
      hasList: hasSearchList,
      items: searchItems
    },
    theme: {
      hasToggleBtn: /class\s*=\s*["'][^"']*theme-toggle-btn[^"']*["']/i.test(content),
      hasEarlyInit: /localStorage\.getItem\(['"](theme|omniget_docs_theme)['"]\)/i.test(content) &&
                    content.indexOf('localStorage.getItem') < (content.indexOf('</head>') > 0 ? content.indexOf('</head>') : 3000),
      hasToggleFunc: /function\s+toggleTheme\s*\(/i.test(combinedJs),
      usesDocThemeKey: /omniget_docs_theme/i.test(combinedJs),
      usesStandardThemeKey: /localStorage\.setItem\(['"]theme['"]/i.test(combinedJs)
    },
    code: {
      copyButtonCount,
      hasCopyFunc: /function\s+copyCodeSnippet\s*\(/i.test(combinedJs),
      hasFallbackCopy: /execCommand\(['"]copy['"]\)/i.test(combinedJs) || /document\.createElement\(['"]textarea['"]\)/i.test(combinedJs)
    },
    pagination: {
      hasGrid: hasPageNavGrid,
      prev: paginationPrev,
      next: paginationNext
    },
    layout: {
      hasBreadcrumbs,
      hasSubtabsId: subtabsContainerMatch,
      activeSubtab,
      isCanonicalHeader: /class\s*=\s*["'][^"']*nav-header[^"']*["']/i.test(content),
      isCanonicalMain: /<main\b[^>]*class\s*=\s*["'][^"']*docs-main[^"']*["']/i.test(content)
    }
  };
}

export function loadAllDocPages() {
  return CANONICAL_DOC_PAGES.map(p => parseDocPage(p));
}
