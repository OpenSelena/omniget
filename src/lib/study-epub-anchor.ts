export type EpubSelectionAnchor = {
  chapterId: string;
  chapterHref: string;
  chapterIndex: number;
  charOffsetStart: number;
  charOffsetEnd: number;
  selectedText: string;
  contextBefore: string;
  contextAfter: string;
};

export type EpubSelectionEvent = EpubSelectionAnchor & {
  rectInIframe: { x: number; y: number; width: number; height: number };
};

export type EpubChapterHighlight = {
  id: number;
  color: string | null;
  char_offset_start: number;
  char_offset_end: number;
};

export type EpubAnchorJson = {
  version: number;
  chapter_id: string;
  chapter_href: string;
  chapter_index: number;
  char_offset_start: number;
  char_offset_end: number;
  selected_text: string;
  context_before: string;
  context_after: string;
};

type WalkEntry = { node: Text; start: number; end: number };

const HIGHLIGHT_STYLES_ID = "study-hl-styles";
const HIGHLIGHT_CSS = `
mark.study-hl { background: rgba(250, 204, 21, 0.42); padding: 0; color: inherit; border-radius: 1px; }
mark.study-hl[data-color="green"] { background: rgba(134, 239, 172, 0.45); }
mark.study-hl[data-color="pink"]  { background: rgba(249, 168, 212, 0.45); }
mark.study-hl[data-color="blue"]  { background: rgba(147, 197, 253, 0.45); }
mark.study-hl[data-color="red"]   { background: rgba(252, 165, 165, 0.45); }
`;

export function ensureHighlightStyles(doc: Document): void {
  if (doc.getElementById(HIGHLIGHT_STYLES_ID)) return;
  const style = doc.createElement("style");
  style.id = HIGHLIGHT_STYLES_ID;
  style.textContent = HIGHLIGHT_CSS;
  doc.head.appendChild(style);
}

export function plainTextWalker(doc: Document): WalkEntry[] {
  const out: WalkEntry[] = [];
  if (!doc.body) return out;
  const walker = doc.createTreeWalker(doc.body, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      const parent = (node as Text).parentElement;
      if (!parent) return NodeFilter.FILTER_ACCEPT;
      const tag = parent.tagName;
      if (tag === "SCRIPT" || tag === "STYLE") return NodeFilter.FILTER_REJECT;
      return NodeFilter.FILTER_ACCEPT;
    },
  });
  let pos = 0;
  let node = walker.nextNode() as Text | null;
  while (node) {
    const len = node.data.length;
    out.push({ node, start: pos, end: pos + len });
    pos += len;
    node = walker.nextNode() as Text | null;
  }
  return out;
}

function findOffsetInWalk(
  walk: WalkEntry[],
  offset: number,
): { node: Text; localOffset: number } | null {
  for (const entry of walk) {
    if (offset >= entry.start && offset <= entry.end) {
      return { node: entry.node, localOffset: offset - entry.start };
    }
  }
  return null;
}

function rangeOffsetForNode(
  walk: WalkEntry[],
  node: Node,
  offset: number,
): number | null {
  if (node.nodeType === Node.TEXT_NODE) {
    for (const entry of walk) {
      if (entry.node === node) return entry.start + offset;
    }
    return null;
  }
  if (node.childNodes.length === 0) {
    for (const entry of walk) {
      if (entry.node.parentNode === node) return entry.start;
    }
    return null;
  }
  const idx = Math.min(offset, node.childNodes.length - 1);
  const child = node.childNodes[idx];
  if (child.nodeType === Node.TEXT_NODE) {
    for (const entry of walk) {
      if (entry.node === child) return entry.start;
    }
  }
  for (const entry of walk) {
    if (node.contains(entry.node)) return entry.start;
  }
  return null;
}

export function captureSelection(
  doc: Document,
  range: Range,
  chapterId: string,
  chapterHref: string,
  chapterIndex: number,
): EpubSelectionAnchor | null {
  const walk = plainTextWalker(doc);
  if (walk.length === 0) return null;
  const startOff = rangeOffsetForNode(
    walk,
    range.startContainer,
    range.startOffset,
  );
  const endOff = rangeOffsetForNode(
    walk,
    range.endContainer,
    range.endOffset,
  );
  if (startOff == null || endOff == null) return null;
  const charStart = Math.min(startOff, endOff);
  const charEnd = Math.max(startOff, endOff);
  if (charEnd === charStart) return null;
  const fullText = walk.map((e) => e.node.data).join("");
  const selectedText = fullText.slice(charStart, charEnd);
  if (!selectedText.trim()) return null;
  const contextBefore = fullText.slice(Math.max(0, charStart - 80), charStart);
  const contextAfter = fullText.slice(charEnd, Math.min(fullText.length, charEnd + 80));
  return {
    chapterId,
    chapterHref,
    chapterIndex,
    charOffsetStart: charStart,
    charOffsetEnd: charEnd,
    selectedText,
    contextBefore,
    contextAfter,
  };
}

export function clearHighlights(doc: Document): void {
  if (!doc.body) return;
  const marks = doc.body.querySelectorAll("mark.study-hl");
  marks.forEach((mark) => {
    const parent = mark.parentNode;
    if (!parent) return;
    while (mark.firstChild) parent.insertBefore(mark.firstChild, mark);
    parent.removeChild(mark);
    parent.normalize();
  });
}

export function applyHighlight(
  doc: Document,
  charStart: number,
  charEnd: number,
  anchorId: number,
  colorKey: string,
): boolean {
  if (charEnd <= charStart) return false;
  const walk = plainTextWalker(doc);
  if (walk.length === 0) return false;

  const segments: Array<{ node: Text; localStart: number; localEnd: number }> = [];
  for (const entry of walk) {
    if (entry.end <= charStart) continue;
    if (entry.start >= charEnd) break;
    const localStart = Math.max(0, charStart - entry.start);
    const localEnd = Math.min(entry.node.data.length, charEnd - entry.start);
    if (localEnd > localStart) {
      segments.push({ node: entry.node, localStart, localEnd });
    }
  }
  if (segments.length === 0) return false;

  for (const seg of segments) {
    let target = seg.node;
    if (seg.localStart > 0) {
      target = target.splitText(seg.localStart);
    }
    const innerLen = seg.localEnd - seg.localStart;
    if (target.data.length > innerLen) {
      target.splitText(innerLen);
    }
    const mark = doc.createElement("mark");
    mark.className = "study-hl";
    mark.setAttribute("data-anchor-id", String(anchorId));
    mark.setAttribute("data-color", colorKey);
    target.parentNode?.insertBefore(mark, target);
    mark.appendChild(target);
  }
  return true;
}
