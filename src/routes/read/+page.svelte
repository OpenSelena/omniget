<script lang="ts">
  import { onMount } from "svelte";

  interface Book {
    id: string;
    title: string;
    author: string;
    cover: string;
    totalPages: number;
    currentPage: number;
    category: string;
    lastRead: string;
  }

  interface Bookmark {
    id: string;
    page: number;
    title: string;
    createdAt: string;
  }

  // Reactive Svelte 5 state runes
  let selectedTab = $state<"library" | "reader">("library");
  let activeBook = $state<Book | null>(null);
  let currentPage = $state(1);
  let fontSize = $state(18);
  let theme = $state<"dark" | "sepia" | "light">("dark");
  let fontFamily = $state<"serif" | "sans" | "mono">("serif");
  let isTocOpen = $state(false);
  let searchQuery = $state("");

  // Sample books
  let books = $state<Book[]>([
    {
      id: "b-1",
      title: "Designing Data-Intensive Applications",
      author: "Martin Kleppmann",
      cover: "https://images.unsplash.com/photo-1544716278-ca5e3f4abd8c?w=300&auto=format&fit=crop&q=80",
      totalPages: 560,
      currentPage: 42,
      category: "Computer Science",
      lastRead: "Today"
    },
    {
      id: "b-2",
      title: "Rust for Systems Programming",
      author: "Jim Blandy & Jason Orendorff",
      cover: "https://images.unsplash.com/photo-1532012164546-f432f2e3d360?w=300&auto=format&fit=crop&q=80",
      totalPages: 420,
      currentPage: 180,
      category: "Engineering",
      lastRead: "Yesterday"
    },
    {
      id: "b-3",
      title: "The Architecture of Open Source Applications",
      author: "Amy Brown & Greg Wilson",
      cover: "https://images.unsplash.com/photo-1512820790803-83ca734da794?w=300&auto=format&fit=crop&q=80",
      totalPages: 380,
      currentPage: 95,
      category: "Architecture",
      lastRead: "3 days ago"
    }
  ]);

  let bookmarks = $state<Bookmark[]>([
    { id: "bm-1", page: 12, title: "Reliability, Scalability, and Maintainability", createdAt: "10:30 AM" },
    { id: "bm-2", page: 42, title: "Data Models and Query Languages", createdAt: "Yesterday" }
  ]);

  let chapters = $state([
    { page: 1, title: "1. Reliable, Scalable, and Maintainable Applications" },
    { page: 42, title: "2. Data Models and Query Languages" },
    { page: 98, title: "3. Storage and Retrieval" },
    { page: 160, title: "4. Encoding and Evolution" },
    { page: 220, title: "5. Replication & Distributed Consensus" }
  ]);

  let filteredBooks = $derived(
    searchQuery.trim() === ""
      ? books
      : books.filter(
          b =>
            b.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
            b.author.toLowerCase().includes(searchQuery.toLowerCase())
        )
  );

  function openBook(book: Book) {
    activeBook = book;
    currentPage = book.currentPage;
    selectedTab = "reader";
  }

  function turnPage(delta: number) {
    if (!activeBook) return;
    const next = Math.max(1, Math.min(activeBook.totalPages, currentPage + delta));
    currentPage = next;
    activeBook.currentPage = next;
  }

  function addBookmark() {
    if (!activeBook) return;
    const bm: Bookmark = {
      id: "bm-" + Date.now(),
      page: currentPage,
      title: `Page ${currentPage} bookmark`,
      createdAt: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
    };
    bookmarks = [bm, ...bookmarks];
  }
</script>

<svelte:head>
  <title>OmniReader - {activeBook ? activeBook.title : "Library"}</title>
</svelte:head>

<div class="read-container" class:sepia-theme={theme === "sepia"} class:light-theme={theme === "light"}>
  <!-- Top Bar -->
  <header class="reader-header">
    <div class="header-left">
      {#if selectedTab === "reader"}
        <button type="button" class="btn-icon" onclick={() => (selectedTab = "library")} title="Back to Library">
          <svg viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" fill="none" stroke-width="2">
            <path d="M19 12H5M12 19l-7-7 7-7" />
          </svg>
        </button>
      {:else}
        <a href="/" class="btn-icon" title="Dashboard">
          <svg viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" fill="none" stroke-width="2">
            <path d="M19 12H5M12 19l-7-7 7-7" />
          </svg>
        </a>
      {/if}

      <div>
        <h1 class="header-title">
          {selectedTab === "reader" && activeBook ? activeBook.title : "OmniReader"}
        </h1>
        <span class="header-sub">
          {selectedTab === "reader" && activeBook ? activeBook.author : "E-Book & Document Study Suite"}
        </span>
      </div>
    </div>

    <!-- Controls for Reader Mode -->
    {#if selectedTab === "reader"}
      <div class="reader-controls">
        <!-- Table of Contents Toggle -->
        <button
          type="button"
          class="btn-icon"
          class:active={isTocOpen}
          onclick={() => (isTocOpen = !isTocOpen)}
          title="Table of Contents"
        >
          <svg viewBox="0 0 24 24" width="18" height="18" stroke="currentColor" fill="none" stroke-width="2">
            <line x1="8" y1="6" x2="21" y2="6" />
            <line x1="8" y1="12" x2="21" y2="12" />
            <line x1="8" y1="18" x2="21" y2="18" />
            <line x1="3" y1="6" x2="3.01" y2="6" />
            <line x1="3" y1="12" x2="3.01" y2="12" />
            <line x1="3" y1="18" x2="3.01" y2="18" />
          </svg>
        </button>

        <!-- Font Size Adjusters -->
        <div class="font-controls">
          <button type="button" class="btn-text" onclick={() => (fontSize = Math.max(14, fontSize - 2))}>A-</button>
          <span class="font-indicator">{fontSize}px</span>
          <button type="button" class="btn-text" onclick={() => (fontSize = Math.min(28, fontSize + 2))}>A+</button>
        </div>

        <!-- Font Family Selector -->
        <div class="font-family-select">
          <button type="button" class="btn-fam" class:active={fontFamily === "serif"} onclick={() => (fontFamily = "serif")}>Serif</button>
          <button type="button" class="btn-fam" class:active={fontFamily === "sans"} onclick={() => (fontFamily = "sans")}>Sans</button>
          <button type="button" class="btn-fam" class:active={fontFamily === "mono"} onclick={() => (fontFamily = "mono")}>Mono</button>
        </div>

        <!-- Theme Toggle (Dark / Sepia / Light) -->
        <div class="theme-select">
          <button type="button" class="btn-theme dark" class:active={theme === "dark"} onclick={() => (theme = "dark")} title="Dark"></button>
          <button type="button" class="btn-theme sepia" class:active={theme === "sepia"} onclick={() => (theme = "sepia")} title="Sepia"></button>
          <button type="button" class="btn-theme light" class:active={theme === "light"} onclick={() => (theme = "light")} title="Light"></button>
        </div>

        <!-- Bookmark button -->
        <button type="button" class="btn-icon" onclick={addBookmark} title="Add Bookmark">
          ★
        </button>
      </div>
    {:else}
      <!-- Library Search Input -->
      <div class="search-box">
        <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" fill="none" stroke-width="2">
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
        </svg>
        <input type="text" placeholder="Search books, authors..." bind:value={searchQuery} />
      </div>
    {/if}
  </header>

  <!-- Content Workspace -->
  <div class="workspace-layout">
    {#if selectedTab === "library"}
      <!-- Books Library Grid -->
      <main class="library-view">
        <div class="books-grid">
          {#each filteredBooks as book}
            <div class="book-card" onclick={() => openBook(book)}>
              <div class="cover-wrapper">
                <img src={book.cover} alt={book.title} class="book-cover" />
                <div class="progress-badge">
                  {Math.round((book.currentPage / book.totalPages) * 100)}%
                </div>
              </div>
              <div class="book-info">
                <span class="category-tag">{book.category}</span>
                <h3 class="book-title">{book.title}</h3>
                <span class="book-author">{book.author}</span>
                <div class="book-meta">Page {book.currentPage} of {book.totalPages} · {book.lastRead}</div>
              </div>
            </div>
          {/each}
        </div>
      </main>

    {:else if selectedTab === "reader" && activeBook}
      <!-- Reader View -->
      <main class="reader-view">
        <!-- Sidebar Table of Contents / Bookmarks -->
        {#if isTocOpen}
          <aside class="toc-sidebar">
            <h3 class="toc-heading">Chapters</h3>
            <div class="chapters-list">
              {#each chapters as ch}
                <button
                  type="button"
                  class="toc-row"
                  class:active={currentPage >= ch.page && currentPage < ch.page + 50}
                  onclick={() => (currentPage = ch.page)}
                >
                  <span class="toc-page">p.{ch.page}</span>
                  <span class="toc-text">{ch.title}</span>
                </button>
              {/each}
            </div>

            <h3 class="toc-heading" style="margin-top: 24px;">Bookmarks</h3>
            <div class="bookmarks-list">
              {#each bookmarks as bm}
                <button type="button" class="bm-row" onclick={() => (currentPage = bm.page)}>
                  <span class="bm-star">★</span>
                  <span class="bm-text">Page {bm.page} · {bm.createdAt}</span>
                </button>
              {/each}
            </div>
          </aside>
        {/if}

        <!-- Book Reading Page -->
        <article class="page-container" class:font-serif={fontFamily === "serif"} class:font-sans={fontFamily === "sans"} class:font-mono={fontFamily === "mono"}>
          <div class="page-paper" style:font-size="{fontSize}px">
            <h2 class="chapter-title">Chapter 2: Data Models and Query Languages</h2>

            <p class="reading-para">
              The limits of my language mean the limits of my world. Data models are perhaps the most important part of developing software, because they have such a profound effect: not only on how the software is written, but also on how we think about the problem that we are solving.
            </p>

            <p class="reading-para">
              Most applications are built by layering one data model on top of another. For each layer, the key question is: how is it represented in terms of the next-lower layer? For example:
            </p>

            <ul class="reading-list">
              <li>As an application developer, you model the real world in terms of objects, data structures, and APIs.</li>
              <li>When you want to store those data structures, you express them in terms of a general-purpose data model, such as JSON or XML documents, tables in a relational database, or a graph model.</li>
              <li>The engineers who built your database software decided on a way of representing that JSON/relational/graph data in terms of bytes on disk or in memory.</li>
            </ul>

            <p class="reading-para">
              In a complex application there may be even more intermediary layers, such as APIs built on top of APIs, but the basic idea remains the same: each layer hides the complexity of the layers below it by providing a clean data model.
            </p>
          </div>

          <!-- Bottom Page Navigation Bar -->
          <footer class="page-footer">
            <button type="button" class="btn-page" onclick={() => turnPage(-1)} disabled={currentPage <= 1}>
              Previous Page
            </button>

            <div class="page-progress">
              <input
                type="range"
                min="1"
                max={activeBook.totalPages}
                bind:value={currentPage}
                class="page-slider"
              />
              <span class="page-label">
                Page {currentPage} of {activeBook.totalPages} ({Math.round((currentPage / activeBook.totalPages) * 100)}%)
              </span>
            </div>

            <button type="button" class="btn-page" onclick={() => turnPage(1)} disabled={currentPage >= activeBook.totalPages}>
              Next Page
            </button>
          </footer>
        </article>
      </main>
    {/if}
  </div>
</div>

<style>
  .read-container {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    background: #090d16;
    color: #f3f4f6;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    transition: background 0.2s ease, color 0.2s ease;
  }

  .read-container.sepia-theme {
    background: #fbf0d9;
    color: #2b261f;
  }

  .read-container.light-theme {
    background: #f8fafc;
    color: #0f172a;
  }

  /* Header */
  .reader-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 28px;
    background: #0f172a;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .sepia-theme .reader-header {
    background: #eedcb6;
    border-bottom-color: rgba(0, 0, 0, 0.1);
  }

  .light-theme .reader-header {
    background: #ffffff;
    border-bottom-color: #e2e8f0;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .btn-icon {
    all: unset;
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.06);
    cursor: pointer;
  }

  .btn-icon:hover {
    background: rgba(255, 255, 255, 0.12);
  }

  .header-title {
    margin: 0;
    font-size: 16px;
    font-weight: 700;
  }

  .header-sub {
    font-size: 12px;
    color: #94a3b8;
  }

  .reader-controls {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .font-controls {
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(255, 255, 255, 0.06);
    padding: 3px 8px;
    border-radius: 6px;
  }

  .btn-text {
    all: unset;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    padding: 2px 6px;
  }

  .font-indicator {
    font-size: 11px;
    color: #94a3b8;
  }

  .font-family-select {
    display: flex;
    gap: 2px;
    background: rgba(255, 255, 255, 0.06);
    padding: 2px;
    border-radius: 6px;
  }

  .btn-fam {
    all: unset;
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
    color: #94a3b8;
  }

  .btn-fam.active {
    background: rgba(255, 255, 255, 0.2);
    color: #fff;
    font-weight: 600;
  }

  .theme-select {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .btn-theme {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
  }

  .btn-theme.dark { background: #090d16; }
  .btn-theme.sepia { background: #eedcb6; }
  .btn-theme.light { background: #ffffff; }

  .btn-theme.active {
    border-color: #0284c7;
    transform: scale(1.1);
  }

  .search-box {
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(255, 255, 255, 0.06);
    padding: 6px 12px;
    border-radius: 8px;
    width: 260px;
  }

  .search-box input {
    all: unset;
    flex: 1;
    font-size: 13px;
    color: #fff;
  }

  /* Library Grid */
  .workspace-layout {
    flex: 1;
    display: flex;
    min-height: 0;
  }

  .library-view {
    flex: 1;
    padding: 28px;
    overflow-y: auto;
  }

  .books-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 24px;
  }

  .book-card {
    background: #1e293b;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    overflow: hidden;
    cursor: pointer;
    transition: transform 0.15s ease, box-shadow 0.15s ease;
  }

  .book-card:hover {
    transform: translateY(-4px);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.4);
  }

  .cover-wrapper {
    position: relative;
    width: 100%;
    aspect-ratio: 3 / 4;
    background: #000;
  }

  .book-cover {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .progress-badge {
    position: absolute;
    bottom: 8px;
    right: 8px;
    background: rgba(0, 0, 0, 0.8);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 700;
    color: #38bdf8;
  }

  .book-info {
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .category-tag {
    font-size: 10px;
    text-transform: uppercase;
    color: #38bdf8;
    font-weight: 700;
    letter-spacing: 0.05em;
  }

  .book-title {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    line-height: 1.3;
  }

  .book-author {
    font-size: 12px;
    color: #94a3b8;
  }

  .book-meta {
    font-size: 11px;
    color: #64748b;
    margin-top: 4px;
  }

  /* Reader View */
  .reader-view {
    flex: 1;
    display: flex;
    min-height: 0;
  }

  .toc-sidebar {
    width: 280px;
    border-right: 1px solid rgba(255, 255, 255, 0.08);
    background: #0f172a;
    padding: 20px;
    overflow-y: auto;
  }

  .toc-heading {
    margin: 0 0 10px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #64748b;
  }

  .chapters-list, .bookmarks-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .toc-row, .bm-row {
    all: unset;
    display: flex;
    gap: 8px;
    padding: 8px;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    line-height: 1.3;
  }

  .toc-row:hover, .bm-row:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .toc-row.active {
    background: rgba(2, 132, 199, 0.15);
    color: #38bdf8;
  }

  .toc-page {
    color: #64748b;
    font-size: 11px;
  }

  /* Page Paper */
  .page-container {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 40px 24px;
  }

  .font-serif { font-family: "Georgia", "Cambria", "Times New Roman", serif; }
  .font-sans { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; }
  .font-mono { font-family: ui-monospace, "SF Mono", "Fira Code", monospace; }

  .page-paper {
    max-width: 720px;
    width: 100%;
    line-height: 1.7;
    margin-bottom: 60px;
  }

  .chapter-title {
    font-size: 1.6em;
    margin-bottom: 1.2em;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    padding-bottom: 0.4em;
  }

  .reading-para {
    margin-bottom: 1.4em;
    text-align: justify;
  }

  .reading-list {
    margin: 1.2em 0;
    padding-left: 1.6em;
  }

  .reading-list li {
    margin-bottom: 0.8em;
  }

  /* Footer */
  .page-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    max-width: 720px;
    padding-top: 20px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    gap: 20px;
  }

  .btn-page {
    all: unset;
    padding: 8px 16px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.06);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-page:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.12);
  }

  .btn-page:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .page-progress {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  .page-slider {
    width: 100%;
    accent-color: #0284c7;
    cursor: pointer;
  }

  .page-label {
    font-size: 11px;
    color: #94a3b8;
    font-variant-numeric: tabular-nums;
  }
</style>
