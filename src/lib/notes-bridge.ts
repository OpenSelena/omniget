import { pluginInvoke } from "$lib/plugin-invoke";

export type NotePage = {
  id: number;
  name: string;
  title: string | null;
  aliases: string[];
  tags: string[];
  journal_day: number | null;
  created_at: number;
  updated_at: number;
};

export type PageSummary = {
  id: number;
  name: string;
  title: string | null;
  journal_day: number | null;
  block_count: number;
  updated_at: number;
};

export type BlockKind =
  | "paragraph"
  | "heading_1"
  | "heading_2"
  | "heading_3"
  | "bullet"
  | "numbered"
  | "todo"
  | "quote"
  | "code"
  | "callout"
  | "divider"
  | "image"
  | "embed"
  | "video";

export type NoteBlock = {
  id: number;
  uuid: string;
  page_id: number;
  parent_id: number | null;
  order_idx: number;
  content: string;
  collapsed: boolean;
  properties: Record<string, unknown>;
  created_at: number;
  updated_at: number;
};

export type BlockNode = NoteBlock & {
  children: BlockNode[];
};

export type InsertPosition = "end" | "start" | "after" | "before";

export type DuplicateReport = {
  new_root_id: number;
  blocks_created: number;
};

export type RefKind = "bracket" | "hash";

export type LinkedRef = {
  block_id: number;
  block_uuid: string;
  page_id: number;
  page_name: string;
  snippet: string;
  updated_at: number;
};

export type GraphFilter = {
  include_tags?: string[];
  exclude_tags?: string[];
  namespace_prefix?: string | null;
  min_refs?: number | null;
  created_after?: number | null;
  created_before?: number | null;
};

export type GraphNode = {
  id: number;
  name: string;
  tags: string[];
  ref_count: number;
  block_count: number;
};

export type GraphEdge = {
  from: number;
  to: number;
  weight: number;
};

export type Graph = {
  nodes: GraphNode[];
  edges: GraphEdge[];
};

export type JournalSummary = {
  page_id: number;
  name: string;
  title: string | null;
  journal_day: number;
  block_count: number;
  updated_at: number;
};

export type JournalEntryRef = {
  page_id: number;
  journal_day: number;
  name: string;
  title: string;
};

export type BlockStatus =
  | "TODO"
  | "DOING"
  | "DONE"
  | "LATER"
  | "NOW"
  | "WAITING"
  | "CANCELED";

export type StatusCounts = {
  todo: number;
  doing: number;
  done: number;
  later: number;
  now: number;
  waiting: number;
  canceled: number;
  total_with_status: number;
};

export type TagSummary = {
  page_id: number;
  name: string;
  ref_count: number;
  block_count: number;
};

export type TemplateSummary = {
  page_id: number;
  name: string;
  title: string | null;
  block_count: number;
  placeholders: string[];
  updated_at: number;
};

export type ApplyReport = {
  blocks_created: number;
  root_block_ids: number[];
};

export type BuiltinKind =
  | "daily-journal"
  | "lesson-notes"
  | "book-highlights"
  | "weekly-review"
  | "concept-page";

export type EmbedTarget =
  | { kind: "page"; name: string }
  | { kind: "block"; uuid: string };

export type ResolvedEmbed =
  | { kind: "page"; page: NotePage; nodes: BlockNode[]; truncated: boolean }
  | { kind: "block"; node: BlockNode; truncated: boolean }
  | { kind: "cycle"; target: EmbedTarget }
  | { kind: "missing"; target: EmbedTarget };

export type EmbedParseResult = {
  block_refs: string[];
  embeds: EmbedTarget[];
};

export type SearchHit = {
  block_id: number;
  block_uuid: string;
  page_id: number;
  page_name: string;
  snippet: string;
  rank: number;
  updated_at: number;
};

export type QueryResult = {
  blocks: NoteBlock[];
  total: number;
  has_more: boolean;
};

export type QuerySort =
  | "updated-desc"
  | "updated-asc"
  | "created-desc"
  | "created-asc"
  | "status"
  | `property:${string}-asc`
  | `property:${string}-desc`;

export type OpSummary = {
  op_id: string;
  kind: string;
  created_at: number;
  row_count: number;
  undone: boolean;
};

export type UndoSummary = {
  op_id: string;
  kind: string;
  blocks_affected: number;
  direction: string;
};

export type ContentSnapshot = {
  id: number;
  block_id: number;
  content: string;
  created_at: number;
};

export type ParsedBlock = {
  depth: number;
  content: string;
  properties: Record<string, unknown>;
};

export type ParsedPage = {
  title: string | null;
  aliases: string[];
  tags: string[];
  blocks: ParsedBlock[];
};

export type ImportReport = {
  page_id: number;
  blocks_created: number;
};

export type PropertyKeyStat = {
  key: string;
  block_count: number;
  distinct_values: number;
};

export type AssetKind = "attachment" | "cover" | "inline_image";

export type NoteSettingValue = string | number | boolean | object | null;

const NS = "study";

// #region pages

export async function notesPagesCreate(args: {
  name: string;
  title?: string;
  journalDay?: number;
}): Promise<{ id: number }> {
  return pluginInvoke<{ id: number }>(NS, "study:notes:pages:create", args);
}

export async function notesPagesEnsure(args: {
  name: string;
  title?: string;
}): Promise<{ id: number }> {
  return pluginInvoke<{ id: number }>(NS, "study:notes:pages:ensure", args);
}

export async function notesPagesGet(id: number): Promise<NotePage> {
  return pluginInvoke<NotePage>(NS, "study:notes:pages:get", { id });
}

export async function notesPagesGetByName(name: string): Promise<NotePage | null> {
  return pluginInvoke<NotePage | null>(NS, "study:notes:pages:get_by_name", { name });
}

export async function notesPagesRename(args: {
  id: number;
  newName: string;
}): Promise<{ ok: true }> {
  return pluginInvoke<{ ok: true }>(NS, "study:notes:pages:rename", args);
}

export async function notesPagesDelete(id: number): Promise<{ removed: boolean }> {
  return pluginInvoke<{ removed: boolean }>(NS, "study:notes:pages:delete", { id });
}

export async function notesPagesList(): Promise<PageSummary[]> {
  return pluginInvoke<PageSummary[]>(NS, "study:notes:pages:list", {});
}

export async function notesPagesRenameCascade(args: {
  id: number;
  newName: string;
}): Promise<{ blocks_updated: number }> {
  return pluginInvoke<{ blocks_updated: number }>(
    NS,
    "study:notes:pages:rename_cascade",
    args,
  );
}

export async function notesPagesResolve(name: string): Promise<NotePage | null> {
  return pluginInvoke<NotePage | null>(NS, "study:notes:pages:resolve", { name });
}

export async function notesPagesSetAliases(args: {
  id: number;
  aliases: string[];
}): Promise<{ ok: true }> {
  return pluginInvoke<{ ok: true }>(NS, "study:notes:pages:set_aliases", args);
}

export async function notesPagesAddAlias(args: {
  id: number;
  alias: string;
}): Promise<{ ok: true }> {
  return pluginInvoke<{ ok: true }>(NS, "study:notes:pages:add_alias", args);
}

export async function notesPagesRemoveAlias(args: {
  id: number;
  alias: string;
}): Promise<{ ok: true }> {
  return pluginInvoke<{ ok: true }>(NS, "study:notes:pages:remove_alias", args);
}

export async function notesPagesSetTags(args: {
  id: number;
  tags: string[];
}): Promise<{ ok: true }> {
  return pluginInvoke<{ ok: true }>(NS, "study:notes:pages:set_tags", args);
}

export async function notesPagesAddTag(args: {
  id: number;
  tag: string;
}): Promise<{ ok: true }> {
  return pluginInvoke<{ ok: true }>(NS, "study:notes:pages:add_tag", args);
}

export async function notesPagesRemoveTag(args: {
  id: number;
  tag: string;
}): Promise<{ ok: true }> {
  return pluginInvoke<{ ok: true }>(NS, "study:notes:pages:remove_tag", args);
}

export async function notesPagesListByTag(tag: string): Promise<PageSummary[]> {
  return pluginInvoke<PageSummary[]>(NS, "study:notes:pages:list_by_tag", { tag });
}

// #endregion

// #region refs

export async function notesPagesLinkedRefs(pageId: number): Promise<LinkedRef[]> {
  return pluginInvoke<LinkedRef[]>(NS, "study:notes:pages:linked_refs", { pageId });
}

export async function notesPagesUnlinkedRefs(pageId: number): Promise<LinkedRef[]> {
  return pluginInvoke<LinkedRef[]>(NS, "study:notes:pages:unlinked_refs", { pageId });
}

export async function notesPagesRefsCount(pageId: number): Promise<{ count: number }> {
  return pluginInvoke<{ count: number }>(NS, "study:notes:pages:refs_count", { pageId });
}

export async function notesRefsRebuildAll(): Promise<{ total_refs: number }> {
  return pluginInvoke<{ total_refs: number }>(NS, "study:notes:refs:rebuild_all", {});
}

// #endregion

// #region graph

export async function notesGraph(filter?: GraphFilter): Promise<Graph> {
  return pluginInvoke<Graph>(NS, "study:notes:graph", { filter });
}

// #endregion

// #region journal

export async function notesJournalToday(args?: {
  applyTemplateId?: number;
}): Promise<JournalEntryRef> {
  return pluginInvoke<JournalEntryRef>(NS, "study:notes:journal:today", args ?? {});
}

export async function notesJournalForDay(args: {
  day: number;
  applyTemplateId?: number;
}): Promise<JournalEntryRef> {
  return pluginInvoke<JournalEntryRef>(NS, "study:notes:journal:for_day", args);
}

export async function notesJournalGetByDay(day: number): Promise<NotePage | null> {
  return pluginInvoke<NotePage | null>(NS, "study:notes:journal:get_by_day", { day });
}

export async function notesJournalPrev(day: number): Promise<{ journal_day: number | null }> {
  return pluginInvoke<{ journal_day: number | null }>(NS, "study:notes:journal:prev", {
    day,
  });
}

export async function notesJournalNext(day: number): Promise<{ journal_day: number | null }> {
  return pluginInvoke<{ journal_day: number | null }>(NS, "study:notes:journal:next", {
    day,
  });
}

export async function notesJournalListRecent(limit?: number): Promise<JournalSummary[]> {
  return pluginInvoke<JournalSummary[]>(NS, "study:notes:journal:list_recent", { limit });
}

export async function notesJournalListInRange(args: {
  fromDay: number;
  toDay: number;
}): Promise<JournalSummary[]> {
  return pluginInvoke<JournalSummary[]>(NS, "study:notes:journal:list_in_range", args);
}

// #endregion

// #region status & deadline

export async function notesBlocksSetStatus(args: {
  id: number;
  status?: BlockStatus | null;
}): Promise<{ ok: true }> {
  return pluginInvoke<{ ok: true }>(NS, "study:notes:blocks:set_status", args);
}

export async function notesBlocksGetStatus(id: number): Promise<{ status: BlockStatus | null }> {
  return pluginInvoke<{ status: BlockStatus | null }>(
    NS,
    "study:notes:blocks:get_status",
    { id },
  );
}

export async function notesBlocksCycleStatus(id: number): Promise<{ status: BlockStatus }> {
  return pluginInvoke<{ status: BlockStatus }>(NS, "study:notes:blocks:cycle_status", {
    id,
  });
}

export async function notesBlocksBulkSetStatus(args: {
  ids: number[];
  status?: BlockStatus | null;
}): Promise<{ updated: number }> {
  return pluginInvoke<{ updated: number }>(
    NS,
    "study:notes:blocks:bulk_set_status",
    args,
  );
}

export async function notesBlocksStatusCounts(pageId: number): Promise<StatusCounts> {
  return pluginInvoke<StatusCounts>(NS, "study:notes:blocks:status_counts", { pageId });
}

export async function notesBlocksListByStatus(args: {
  status: BlockStatus;
  pageId?: number;
  limit?: number;
}): Promise<NoteBlock[]> {
  return pluginInvoke<NoteBlock[]>(NS, "study:notes:blocks:list_by_status", args);
}

export async function notesBlocksSetDeadline(args: {
  id: number;
  deadline?: string | null;
}): Promise<{ ok: true }> {
  return pluginInvoke<{ ok: true }>(NS, "study:notes:blocks:set_deadline", args);
}

export async function notesBlocksListByDeadline(args: {
  beforeIso: string;
  pageId?: number;
}): Promise<NoteBlock[]> {
  return pluginInvoke<NoteBlock[]>(NS, "study:notes:blocks:list_by_deadline", args);
}

// #endregion

// #region tags

export async function notesTagsList(limit?: number): Promise<TagSummary[]> {
  return pluginInvoke<TagSummary[]>(NS, "study:notes:tags:list", { limit });
}

export async function notesTagsAutocomplete(args: {
  query: string;
  limit?: number;
}): Promise<TagSummary[]> {
  return pluginInvoke<TagSummary[]>(NS, "study:notes:tags:autocomplete", args);
}

export async function notesTagsRenameCascade(args: {
  pageId: number;
  newName: string;
}): Promise<{ blocks_updated: number }> {
  return pluginInvoke<{ blocks_updated: number }>(
    NS,
    "study:notes:tags:rename_cascade",
    args,
  );
}

// #endregion

// #region embed

export async function notesEmbedResolveBlock(uuid: string): Promise<NoteBlock | null> {
  return pluginInvoke<NoteBlock | null>(NS, "study:notes:embed:resolve_block", { uuid });
}

export async function notesEmbedResolve(target: EmbedTarget): Promise<ResolvedEmbed> {
  return pluginInvoke<ResolvedEmbed>(NS, "study:notes:embed:resolve", { target });
}

export async function notesEmbedParse(content: string): Promise<EmbedParseResult> {
  return pluginInvoke<EmbedParseResult>(NS, "study:notes:embed:parse", { content });
}

// #endregion

// #region property

export async function notesBlocksPropertySet(args: {
  blockId: number;
  key: string;
  value: unknown;
}): Promise<{ ok: true }> {
  return pluginInvoke<{ ok: true }>(NS, "study:notes:blocks:property:set", args);
}

export async function notesBlocksPropertyGet(args: {
  blockId: number;
  key: string;
}): Promise<{ value: unknown }> {
  return pluginInvoke<{ value: unknown }>(NS, "study:notes:blocks:property:get", args);
}

export async function notesBlocksPropertyRemove(args: {
  blockId: number;
  key: string;
}): Promise<{ ok: true }> {
  return pluginInvoke<{ ok: true }>(NS, "study:notes:blocks:property:remove", args);
}

export async function notesBlocksPropertyMerge(args: {
  blockId: number;
  patch: Record<string, unknown>;
}): Promise<{ ok: true }> {
  return pluginInvoke<{ ok: true }>(NS, "study:notes:blocks:property:merge", args);
}

export async function notesBlocksPropertyListKeys(args?: {
  pageId?: number;
}): Promise<PropertyKeyStat[]> {
  return pluginInvoke<PropertyKeyStat[]>(
    NS,
    "study:notes:blocks:property:list_keys",
    args ?? {},
  );
}

export async function notesBlocksPropertyListBlocks(args: {
  key: string;
  value?: unknown;
  pageId?: number;
  limit?: number;
}): Promise<NoteBlock[]> {
  return pluginInvoke<NoteBlock[]>(
    NS,
    "study:notes:blocks:property:list_blocks",
    args,
  );
}

export async function notesBlocksPropertyBulkSet(args: {
  ids: number[];
  key: string;
  value: unknown;
}): Promise<{ updated: number }> {
  return pluginInvoke<{ updated: number }>(
    NS,
    "study:notes:blocks:property:bulk_set",
    args,
  );
}

// #endregion

// #region markdown

export async function notesMarkdownImport(args: {
  name: string;
  markdown: string;
}): Promise<ImportReport> {
  return pluginInvoke<ImportReport>(NS, "study:notes:markdown:import", args);
}

export async function notesMarkdownParse(content: string): Promise<ParsedPage> {
  return pluginInvoke<ParsedPage>(NS, "study:notes:markdown:parse", { content });
}

// #endregion

// #region search

export async function notesSearchBlocks(args: {
  query: string;
  limit?: number;
}): Promise<SearchHit[]> {
  return pluginInvoke<SearchHit[]>(NS, "study:notes:search:blocks", args);
}

export async function notesSearchPages(args: {
  query: string;
  limit?: number;
}): Promise<PageSummary[]> {
  return pluginInvoke<PageSummary[]>(NS, "study:notes:search:pages", args);
}

export async function notesSearchRebuild(): Promise<{ indexed: number }> {
  return pluginInvoke<{ indexed: number }>(NS, "study:notes:search:rebuild", {});
}

// #endregion

// #region query

export async function notesQueryRun(args: {
  expr: string;
  limit?: number;
  offset?: number;
  sort?: QuerySort;
}): Promise<QueryResult> {
  return pluginInvoke<QueryResult>(NS, "study:notes:query:run", args);
}

export async function notesQueryInvalidateCache(): Promise<{ ok: true; size_after: number }> {
  return pluginInvoke<{ ok: true; size_after: number }>(
    NS,
    "study:notes:query:invalidate_cache",
    {},
  );
}

// #endregion

// #region export

export async function notesExportPageJson(pageId: number): Promise<{ json: string }> {
  return pluginInvoke<{ json: string }>(NS, "study:notes:export:page_json", { pageId });
}

export async function notesExportPageOpml(pageId: number): Promise<{ opml: string }> {
  return pluginInvoke<{ opml: string }>(NS, "study:notes:export:page_opml", { pageId });
}

export async function notesExportPageHtml(pageId: number): Promise<{ html: string }> {
  return pluginInvoke<{ html: string }>(NS, "study:notes:export:page_html", { pageId });
}

export async function notesExportGraphJson(): Promise<{ json: string }> {
  return pluginInvoke<{ json: string }>(NS, "study:notes:export:graph_json", {});
}

export async function notesExportPageMd(pageId: number): Promise<{ markdown: string }> {
  return pluginInvoke<{ markdown: string }>(NS, "study:notes:export:page_md", { pageId });
}

export async function notesExportSubtreeMd(blockId: number): Promise<{ markdown: string }> {
  return pluginInvoke<{ markdown: string }>(NS, "study:notes:export:subtree_md", {
    blockId,
  });
}

// #endregion

// #region templates

export async function notesTemplatesMark(args: {
  pageId: number;
  isTemplate?: boolean;
}): Promise<{ ok: true }> {
  return pluginInvoke<{ ok: true }>(NS, "study:notes:templates:mark", args);
}

export async function notesTemplatesList(): Promise<TemplateSummary[]> {
  return pluginInvoke<TemplateSummary[]>(NS, "study:notes:templates:list", {});
}

export async function notesTemplatesPlaceholders(pageId: number): Promise<string[]> {
  return pluginInvoke<string[]>(NS, "study:notes:templates:placeholders", { pageId });
}

export async function notesTemplatesApply(args: {
  templatePageId: number;
  targetPageId: number;
  parentBlockId?: number;
  vars?: Record<string, string>;
}): Promise<ApplyReport> {
  return pluginInvoke<ApplyReport>(NS, "study:notes:templates:apply", args);
}

export async function notesTemplatesApplyBuiltin(args: {
  kind: BuiltinKind;
  targetPageId: number;
  parentBlockId?: number;
  vars?: Record<string, string>;
}): Promise<ApplyReport> {
  return pluginInvoke<ApplyReport>(NS, "study:notes:templates:apply_builtin", args);
}

// #endregion

// #region undo

export async function notesUndoSnapshot(blockId: number): Promise<{ ok: true }> {
  return pluginInvoke<{ ok: true }>(NS, "study:notes:undo:snapshot", { blockId });
}

export async function notesUndoLast(blockId: number): Promise<{ restored: boolean }> {
  return pluginInvoke<{ restored: boolean }>(NS, "study:notes:undo:last", { blockId });
}

export async function notesUndoHistory(args: {
  blockId: number;
  limit?: number;
}): Promise<ContentSnapshot[]> {
  return pluginInvoke<ContentSnapshot[]>(NS, "study:notes:undo:history", args);
}

export async function notesUndoRestoreTo(args: {
  blockId: number;
  snapshotId: number;
}): Promise<{ restored: boolean }> {
  return pluginInvoke<{ restored: boolean }>(NS, "study:notes:undo:restore_to", args);
}

export async function notesUndoClear(blockId: number): Promise<{ removed: number }> {
  return pluginInvoke<{ removed: number }>(NS, "study:notes:undo:clear", { blockId });
}

export async function notesUndoLastOp(opId?: string): Promise<UndoSummary> {
  return pluginInvoke<UndoSummary>(NS, "study:notes:undo:last_op", { opId });
}

export async function notesUndoRedoLast(opId?: string): Promise<UndoSummary> {
  return pluginInvoke<UndoSummary>(NS, "study:notes:undo:redo_last", { opId });
}

export async function notesUndoListOps(limit?: number): Promise<OpSummary[]> {
  return pluginInvoke<OpSummary[]>(NS, "study:notes:undo:list_ops", { limit });
}

// #endregion

// #region blocks

export async function notesBlocksCreate(args: {
  pageId: number;
  parentId?: number | null;
  content?: string;
  position?: InsertPosition;
  anchorId?: number;
}): Promise<{ id: number }> {
  return pluginInvoke<{ id: number }>(NS, "study:notes:blocks:create", args);
}

export async function notesBlocksGet(id: number): Promise<NoteBlock> {
  return pluginInvoke<NoteBlock>(NS, "study:notes:blocks:get", { id });
}

export async function notesBlocksUpdate(args: {
  id: number;
  content?: string;
  collapsed?: boolean;
  properties?: Record<string, unknown>;
}): Promise<{ ok: true }> {
  return pluginInvoke<{ ok: true }>(NS, "study:notes:blocks:update", args);
}

export async function notesBlocksDelete(id: number): Promise<{ removed: boolean }> {
  return pluginInvoke<{ removed: boolean }>(NS, "study:notes:blocks:delete", { id });
}

export async function notesBlocksDeleteMany(ids: number[]): Promise<{ removed: number }> {
  return pluginInvoke<{ removed: number }>(NS, "study:notes:blocks:delete_many", { ids });
}

export async function notesBlocksMove(args: {
  id: number;
  newParentId?: number | null;
  newOrderIdx: number;
}): Promise<{ ok: true }> {
  return pluginInvoke<{ ok: true }>(NS, "study:notes:blocks:move", args);
}

export async function notesBlocksMoveMany(args: {
  ids: number[];
  newParentId?: number | null;
  newOrderIdx: number;
}): Promise<{ moved: number }> {
  return pluginInvoke<{ moved: number }>(NS, "study:notes:blocks:move_many", args);
}

export async function notesBlocksIndent(id: number): Promise<{ ok: true }> {
  return pluginInvoke<{ ok: true }>(NS, "study:notes:blocks:indent", { id });
}

export async function notesBlocksOutdent(id: number): Promise<{ ok: true }> {
  return pluginInvoke<{ ok: true }>(NS, "study:notes:blocks:outdent", { id });
}

export async function notesBlocksCollapseToggle(id: number): Promise<{ collapsed: boolean }> {
  return pluginInvoke<{ collapsed: boolean }>(
    NS,
    "study:notes:blocks:collapse_toggle",
    { id },
  );
}

export async function notesBlocksListChildren(args: {
  pageId: number;
  parentId?: number | null;
}): Promise<NoteBlock[]> {
  return pluginInvoke<NoteBlock[]>(NS, "study:notes:blocks:list_children", args);
}

export async function notesBlocksPageTree(pageId: number): Promise<BlockNode[]> {
  return pluginInvoke<BlockNode[]>(NS, "study:notes:blocks:page_tree", { pageId });
}

export async function notesBlocksSubtree(id: number): Promise<BlockNode> {
  return pluginInvoke<BlockNode>(NS, "study:notes:blocks:subtree", { id });
}

export async function notesBlocksDuplicate(id: number): Promise<DuplicateReport> {
  return pluginInvoke<DuplicateReport>(NS, "study:notes:blocks:duplicate", { id });
}

// #endregion

// #region lessons (A2)

export type LessonLinkResult = {
  page_id: number;
  block_id: number;
  page_name: string;
};

export async function notesLessonsLink(args: {
  lessonId: number;
  timestampSeconds: number;
  bodyMd: string;
}): Promise<LessonLinkResult> {
  return pluginInvoke<LessonLinkResult>(NS, "study:notes:lessons:link", args);
}

export async function notesLessonsListForLesson(lessonId: number): Promise<NoteBlock[]> {
  return pluginInvoke<NoteBlock[]>(NS, "study:notes:lessons:list_for_lesson", {
    lessonId,
  });
}

// #endregion

// #region settings (A2)

export type NoteSettingEntry = {
  key: string;
  value: NoteSettingValue;
  updated_at: number;
};

export async function notesSettingsGet(key: string): Promise<{ value: NoteSettingValue }> {
  return pluginInvoke<{ value: NoteSettingValue }>(NS, "study:notes:settings:get", {
    key,
  });
}

export async function notesSettingsSet(args: {
  key: string;
  value: NoteSettingValue;
}): Promise<{ ok: true }> {
  return pluginInvoke<{ ok: true }>(NS, "study:notes:settings:set", args);
}

export async function notesSettingsList(): Promise<NoteSettingEntry[]> {
  return pluginInvoke<NoteSettingEntry[]>(NS, "study:notes:settings:list", {});
}

// #endregion

// #region blocks turn_into (A2)

export async function notesBlocksTurnInto(args: {
  blockId: number;
  newKind: BlockKind;
}): Promise<NoteBlock> {
  return pluginInvoke<NoteBlock>(NS, "study:notes:blocks:turn_into", args);
}

// #endregion

// #region assets (A2)

export type AssetSummary = {
  id: number;
  page_id: number;
  block_id: number | null;
  kind: AssetKind;
  path: string;
  size: number | null;
  mime: string | null;
  added_at: number;
};

export async function notesAssetsUpload(args: {
  pageId: number;
  blockId?: number;
  fileBytesB64: string;
  mime: string;
  name: string;
  kind?: AssetKind;
}): Promise<{ asset_id: number; relative_path: string }> {
  return pluginInvoke<{ asset_id: number; relative_path: string }>(
    NS,
    "study:notes:assets:upload",
    args,
  );
}

export async function notesAssetsListForPage(pageId: number): Promise<AssetSummary[]> {
  return pluginInvoke<AssetSummary[]>(NS, "study:notes:assets:list_for_page", {
    pageId,
  });
}

export async function notesAssetsRemove(assetId: number): Promise<{ removed: boolean }> {
  return pluginInvoke<{ removed: boolean }>(NS, "study:notes:assets:remove", {
    assetId,
  });
}

export async function notesAssetsUploadCover(args: {
  pageId: number;
  fileBytesB64: string;
  mime: string;
  name: string;
}): Promise<{ asset_id: number; cover_url: string; width: number; height: number }> {
  return pluginInvoke<{ asset_id: number; cover_url: string; width: number; height: number }>(
    NS,
    "study:notes:assets:upload_cover",
    args,
  );
}

export async function notesCoverSetExternal(args: {
  pageId: number;
  url: string;
}): Promise<{ asset_id: number }> {
  return pluginInvoke<{ asset_id: number }>(NS, "study:notes:cover:set_external", args);
}

export async function notesCoverRemove(pageId: number): Promise<{ removed: number }> {
  return pluginInvoke<{ removed: number }>(NS, "study:notes:cover:remove", { pageId });
}

// #endregion

// #region cards export (A2)

export async function notesCardsExportToAnki(args: {
  blockIds: number[];
  deckId: number;
}): Promise<{ exported_count: number; deck_id: number }> {
  return pluginInvoke<{ exported_count: number; deck_id: number }>(
    NS,
    "study:notes:cards:export_to_anki",
    args,
  );
}

// #endregion
