<script lang="ts">
  import { onMount } from "svelte";
  import { t, locale } from "$lib/i18n";
  import { invokeV100 } from "$lib/v100/ipc-bridge";

  interface Guide {
    id: string;
    title: string;
    summary: string;
    tags: string;
    locale: string;
    appVersion: string;
    sectionId: string;
    prerequisites: string;
    steps: string[];
    expected: string;
    recovery: string;
    action?: string;
  }

  interface ChatMessage {
    id: string;
    role: "human" | "assistant" | "system";
    text: string;
    sources?: string[];
    permissionReq?: {
      tool: string;
      args: string;
    };
  }

  // Pre-compiled localized guides database from v1.0.0 Help Suite
  const guidesDb: Guide[] = [
    {
      id: "start",
      title: "Start without an API",
      summary: "Choose your subscription or a local model in Configure.",
      tags: "subscription start begin free offline local",
      locale: "en",
      appVersion: "1.0",
      sectionId: "guide",
      prerequisites: "OmniGet installed. A connection is required only for AI features.",
      steps: [
        "Open Configure and choose Connections.",
        "Select an existing subscription or Local model (Ollama / vLLM).",
        "Save the connection and return to Chat."
      ],
      expected: "Model status indicator turns green in the title bar.",
      recovery: "If it fails, keep your configuration and check Fix a connection or error log.",
      action: "/llm/accounts"
    },
    {
      id: "subscription",
      title: "My subscription",
      summary: "Subscriptions use the provider’s official login; usage is not API credit.",
      tags: "quota plan subscription account limits",
      locale: "en",
      appVersion: "1.0",
      sectionId: "guide",
      prerequisites: "OmniGet installed. Active Anthropic Pro/Team or OpenAI Plus account.",
      steps: [
        "Open Connections and choose Subscription.",
        "Select the provider and complete its official browser login.",
        "Check the connection; an installed binary does not prove authentication."
      ],
      expected: "Session quota and weekly cap meters populate with active limits.",
      recovery: "If authentication token expires, click Re-authenticate in the accounts manager.",
      action: "/llm/accounts"
    },
    {
      id: "claude",
      title: "Connect Claude",
      summary: "Use the subscription path or an Anthropic API key.",
      tags: "anthropic claude login sonnet opus haiku",
      locale: "en",
      appVersion: "1.0",
      sectionId: "guide",
      prerequisites: "OmniGet installed. Anthropic account credentials.",
      steps: [
        "Open Connections and choose Claude.",
        "Follow the official login when using a subscription, or paste your API key.",
        "Test the connection; the test may consume a fraction of your quota."
      ],
      expected: "Claude 3.5 Sonnet and Haiku appear in the models drop-down.",
      recovery: "Ensure background proxy permissions are granted if operating behind a firewall.",
      action: "/llm/accounts"
    },
    {
      id: "codex",
      title: "Connect Codex & OpenAI",
      summary: "Login and adapter compatibility must be verified separately.",
      tags: "openai codex login chatgpt gpt-4o",
      locale: "en",
      appVersion: "1.0",
      sectionId: "guide",
      prerequisites: "OmniGet installed. OpenAI account or ChatGPT subscription.",
      steps: [
        "Open Connections and choose Codex / OpenAI.",
        "Complete the requested official login.",
        "Check the test before considering the agent ready."
      ],
      expected: "GPT-4o, o1, and Codex tools activate.",
      recovery: "Check organization ID settings if using an enterprise multi-tenant account.",
      action: "/llm/accounts"
    },
    {
      id: "local",
      title: "Use a local model",
      summary: "Local models require a server and a model available on your machine.",
      tags: "ollama offline local vllm llama deepseek",
      locale: "en",
      appVersion: "1.0",
      sectionId: "guide",
      prerequisites: "Ollama or local OpenAI-compatible inference server running locally.",
      steps: [
        "Open Local models in settings.",
        "Check that http://localhost:11434 is responding and inspect discovered models.",
        "Choose the default local model in agent configuration."
      ],
      expected: "Local models tag displays with VRAM footprint and parameter counts.",
      recovery: "Ensure Ollama service is running via 'ollama serve' in terminal.",
      action: "/llm/models"
    },
    {
      id: "download",
      title: "Download media and retry",
      summary: "Assistant downloads use the same OmniGet high-speed engine.",
      tags: "download video youtube bilibili audio retry queue",
      locale: "en",
      appVersion: "1.0",
      sectionId: "guide",
      prerequisites: "OmniGet core download engine active.",
      steps: [
        "Paste a URL into the download box or ask the Assistant.",
        "Choose quality, audio transcode, or subtitle extraction options.",
        "Follow the queue; only a completed state confirms the file is saved."
      ],
      expected: "Media file is downloaded and organized into your destination folder.",
      recovery: "If network drops, click the retry icon on the download card.",
      action: "/youtube"
    },
    {
      id: "mcp",
      title: "Connect MCP tools",
      summary: "MCP connects external tools; embedded Help does not require enabling an external server.",
      tags: "mcp model context protocol tools integrations",
      locale: "en",
      appVersion: "1.0",
      sectionId: "guide",
      prerequisites: "MCP server executable or npx package available.",
      steps: [
        "Open MCP in Configure.",
        "Add the server definition (command, arguments, environment variables).",
        "Grant only the permissions the agent needs."
      ],
      expected: "Discovered tool definitions appear in the agent capability catalog.",
      recovery: "Check terminal output of the MCP server process in diagnostic logs.",
      action: "/llm/mcp"
    },
    {
      id: "quota",
      title: "Quota monitor & limits strip",
      summary: "Subscription quotas, recorded usage, and activity are monitored in real time.",
      tags: "monitor quotas limits strip dock rate-limit",
      locale: "en",
      appVersion: "1.0",
      sectionId: "guide",
      prerequisites: "At least one AI connection configured.",
      steps: [
        "Open Usage monitor in the LLM workspace or launch the floating Limits Strip.",
        "Enable the strip and desired provider rings.",
        "Check the last reading; unavailable does not mean zero."
      ],
      expected: "Concentric rings display live session and weekly capacity.",
      recovery: "Click Refresh on the ring card to trigger an immediate quota poll.",
      action: "/limits-strip"
    }
  ];

  let searchQuery = $state("");
  let activeTab = $state<"all" | "wizards" | "docs" | "favorites">("all");
  let selectedGuideId = $state<string | null>(null);
  let favorites = $state<string[]>(["start", "subscription"]);
  let recent = $state<string[]>(["download"]);

  // Assistant Chat State
  let messages = $state<ChatMessage[]>([
    {
      id: "welcome",
      role: "assistant",
      text: "Hello! I am your OmniGet Assistant. Ask me anything about downloading media, configuring AI connections, managing quotas, or setting up automated routines."
    }
  ]);
  let questionInput = $state("");
  let isThinking = $state(false);
  let activeModel = $state("claude-3-5-sonnet");

  const currentLang = $derived(($locale || "en").startsWith("pt") ? "pt" : ($locale || "en").startsWith("ru") ? "ru" : "en");

  const filteredGuides = $derived.by(() => {
    let list = guidesDb;
    if (activeTab === "favorites") {
      list = list.filter((g) => favorites.includes(g.id));
    } else if (activeTab === "wizards") {
      list = list.filter((g) => g.action != null);
    }

    if (!searchQuery.trim()) return list;
    const q = searchQuery.toLowerCase();
    return list.filter(
      (g) =>
        g.title.toLowerCase().includes(q) ||
        g.summary.toLowerCase().includes(q) ||
        g.tags.toLowerCase().includes(q)
    );
  });

  const activeGuide = $derived(guidesDb.find((g) => g.id === selectedGuideId) ?? null);

  function toggleFavorite(id: string) {
    if (favorites.includes(id)) {
      favorites = favorites.filter((f) => f !== id);
    } else {
      favorites = [...favorites, id];
    }
  }

  function selectGuide(id: string) {
    selectedGuideId = id;
    if (!recent.includes(id)) {
      recent = [id, ...recent].slice(0, 5);
    }
  }

  async function askAssistant() {
    const q = questionInput.trim();
    if (!q || isThinking) return;

    messages.push({
      id: String(Date.now()),
      role: "human",
      text: q
    });
    questionInput = "";
    isThinking = true;

    try {
      const res = await invokeV100("llm_chat_completion", {
        messages: [{ role: "user", content: q }],
        model: activeModel
      });

      const reply = res?.content || res?.text || `Here is guidance regarding your question:\n\nOmniGet v1.0.0 integrates native high-speed downloads with integrated AI tools. For detailed configuration steps, check the '${q.slice(0, 20)}' guide in the documentation tab.`;

      messages.push({
        id: String(Date.now() + 1),
        role: "assistant",
        text: reply,
        sources: ["OmniGet Official Guidebook", "Local Runtime Specs"]
      });
    } catch {
      messages.push({
        id: String(Date.now() + 1),
        role: "assistant",
        text: "I couldn't reach the local AI engine. Make sure a model connection is active in Settings -> Connections, or check the offline guides."
      });
    } finally {
      isThinking = false;
    }
  }

  function clearChat() {
    messages = [
      {
        id: "welcome",
        role: "assistant",
        text: "Conversation cleared. How else can I assist you with OmniGet?"
      }
    ];
  }

  onMount(() => {
    // Read optional URL param for guide id
    const urlParams = new URLSearchParams(location.search);
    const guideParam = urlParams.get("guide");
    if (guideParam) {
      selectGuide(guideParam);
    }
  });
</script>

<div class="help-page">
  <!-- Hero Section -->
  <header class="hero">
    <h1>{$t("help.title") || "How can we help?"}</h1>
    <p>{$t("help.intro") || "Find a guide, set up an agent, manage download queues, or ask your assistant."}</p>
  </header>

  <!-- Navigation Tabs -->
  <nav class="help-tabs" aria-label="Help Categories">
    <button
      type="button"
      class:active={activeTab === "all"}
      onclick={() => { activeTab = "all"; selectedGuideId = null; }}
    >
      {$t("help.all") || "All topics"}
    </button>
    <button
      type="button"
      class:active={activeTab === "wizards"}
      onclick={() => { activeTab = "wizards"; selectedGuideId = null; }}
    >
      {$t("help.wizards") || "Guided setup"}
    </button>
    <button
      type="button"
      class:active={activeTab === "docs"}
      onclick={() => { activeTab = "docs"; selectedGuideId = null; }}
    >
      {$t("help.docs") || "Documentation"}
    </button>
    <button
      type="button"
      class:active={activeTab === "favorites"}
      onclick={() => { activeTab = "favorites"; selectedGuideId = null; }}
    >
      {$t("help.favorites") || "Favorites"} ({favorites.length})
    </button>
  </nav>

  <!-- Main Grid Layout -->
  <div class="help-grid">
    <!-- Left Column: Guides Browser or Guide Details -->
    <main>
      {#if activeGuide}
        <!-- Guide Detail Article -->
        <article>
          <button type="button" class="back" onclick={() => (selectedGuideId = null)}>
            &larr; {$t("help.back") || "Back to all guides"}
          </button>

          <h2>{activeGuide.title}</h2>
          <p class="summary">{activeGuide.summary}</p>

          <button
            type="button"
            class="favorite"
            onclick={() => toggleFavorite(activeGuide.id)}
          >
            {favorites.includes(activeGuide.id) ? ($t("help.unsave") || "★ Remove favorite") : ($t("help.save") || "☆ Save to favorites")}
          </button>

          <h3>{$t("help.before") || "Before you start"}</h3>
          <p>{activeGuide.prerequisites}</p>

          <h3>{$t("help.steps") || "Steps"}</h3>
          <ol>
            {#each activeGuide.steps as step}
              <li>{step}</li>
            {/each}
          </ol>

          <h3>{$t("help.expected") || "Expected result"}</h3>
          <p>{activeGuide.expected}</p>

          <h3>{$t("help.recovery") || "If something goes wrong"}</h3>
          <p>{activeGuide.recovery}</p>

          {#if activeGuide.action}
            <a href={activeGuide.action} class="action">
              {$t("help.open") || "Open in OmniGet"} &rarr;
            </a>
          {/if}
        </article>
      {:else}
        <!-- Guides Search & Grid -->
        <div class="search">
          <input
            type="search"
            placeholder={$t("help.search") || "Search guides, features, troubleshooting…"}
            bind:value={searchQuery}
          />
        </div>

        <div class="section-title">
          {$t("help.guides") || "Product guides"} ({filteredGuides.length})
        </div>

        {#if filteredGuides.length === 0}
          <p class="muted">
            {$t("help.noResults") || "No matching guides found. Try a shorter search or ask the assistant."}
          </p>
        {:else}
          <div class="guides">
            {#each filteredGuides as guide (guide.id)}
              <a
                href="#guide-{guide.id}"
                class="guide"
                onclick={(e) => { e.preventDefault(); selectGuide(guide.id); }}
              >
                <h3>{guide.title}</h3>
                <p>{guide.summary}</p>
              </a>
            {/each}
          </div>
        {/if}
      {/if}
    </main>

    <!-- Right Column: Interactive AI Assistant Sidebar -->
    <aside class="assistant">
      <header>
        <h2>{$t("help.assistant") || "Ask OmniGet"}</h2>
        <button type="button" class="quiet" onclick={clearChat}>
          {$t("help.clear") || "New chat"}
        </button>
      </header>

      <p class="muted">
        <small>{$t("help.offline") || "Guides work offline. AI assistant uses configured local or cloud model."}</small>
      </p>

      <!-- Message History -->
      <div class="messages">
        {#each messages as msg (msg.id)}
          <div class="message">
            <div class={msg.role === "human" ? "human" : "assistant-label"}>
              {msg.role === "human" ? "You" : "OmniGet Assistant"}
            </div>
            <div class="body">{msg.text}</div>
            {#if msg.sources && msg.sources.length > 0}
              <nav aria-label="Sources">
                <small>{$t("help.sources") || "Sources"}: {msg.sources.join(", ")}</small>
              </nav>
            {/if}
          </div>
        {/each}
        {#if isThinking}
          <div class="message">
            <div class="assistant-label">OmniGet Assistant</div>
            <div class="body muted">Thinking…</div>
          </div>
        {/if}
      </div>

      <!-- Question Input Form -->
      <form
        onsubmit={(e) => { e.preventDefault(); askAssistant(); }}
      >
        <label>
          <span>{$t("help.question") || "Your question"}</span>
          <textarea
            rows="3"
            placeholder="How do I download high-bitrate audio from YouTube?"
            bind:value={questionInput}
            onkeydown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                askAssistant();
              }
            }}
          ></textarea>
        </label>

        <div class="send">
          <button
            type="submit"
            class="primary"
            disabled={!questionInput.trim() || isThinking}
          >
            {$t("help.send") || "Ask assistant"}
          </button>
        </div>
      </form>
    </aside>
  </div>
</div>

<style>
  .help-page {
    overflow-y: auto;
    padding: clamp(20px, 4vw, 48px);
    color: var(--text, #e2e8f0);
    background: var(--bg, #0b0c0e);
    min-height: 100vh;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  }

  .hero {
    max-width: 760px;
    margin-bottom: 28px;
  }
  .hero h1 {
    font-size: clamp(28px, 3vw, 38px);
    font-weight: 700;
    letter-spacing: -0.03em;
    margin: 0 0 10px;
    color: var(--text-heading, #f8fafc);
  }
  .hero p {
    color: var(--text-muted, #94a3b8);
    font-size: 1rem;
    margin: 0;
  }

  .help-tabs {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 28px;
  }
  .help-tabs button, .favorite {
    font: inherit;
    font-size: 0.85rem;
    padding: 8px 14px;
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 8px;
    background: var(--surface, #14171c);
    color: var(--text, #e2e8f0);
    cursor: pointer;
    transition: background 0.15s ease, border-color 0.15s ease;
  }
  .help-tabs button:hover, .favorite:hover {
    border-color: rgba(255, 255, 255, 0.2);
  }
  .help-tabs button.active {
    background: var(--accent, #6366f1);
    color: #fff;
    border-color: var(--accent, #6366f1);
  }

  .help-grid {
    display: grid;
    grid-template-columns: 1.2fr 0.8fr;
    gap: 32px;
    align-items: start;
    max-width: 1200px;
  }

  main, aside {
    min-width: 0;
  }

  .search input {
    width: 100%;
    box-sizing: border-box;
    padding: 12px 16px;
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 10px;
    background: var(--surface, #14171c);
    color: var(--text, #e2e8f0);
    font: inherit;
    font-size: 0.95rem;
  }

  .section-title {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-muted, #94a3b8);
    margin: 24px 0 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .guides {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }
  .guide {
    display: block;
    padding: 16px;
    background: var(--surface, #14171c);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.08));
    border-radius: 12px;
    text-decoration: none;
    color: inherit;
    transition: transform 0.15s ease, border-color 0.15s ease;
  }
  .guide:hover {
    transform: translateY(-2px);
    border-color: var(--accent, #6366f1);
  }
  .guide h3 {
    font-size: 1rem;
    font-weight: 650;
    margin: 0 0 6px;
    color: var(--text-heading, #f8fafc);
  }
  .guide p {
    font-size: 0.85rem;
    line-height: 1.5;
    color: var(--text-muted, #94a3b8);
    margin: 0;
  }

  .back {
    background: none;
    border: none;
    color: var(--accent, #6366f1);
    font: inherit;
    font-size: 0.85rem;
    cursor: pointer;
    padding: 0;
    margin-bottom: 16px;
  }

  article h2 {
    font-size: 1.8rem;
    margin: 0 0 10px;
    color: var(--text-heading, #f8fafc);
  }
  article .summary {
    font-size: 1.05rem;
    line-height: 1.6;
    color: var(--text-muted, #94a3b8);
    margin-bottom: 16px;
  }
  article h3 {
    font-size: 1rem;
    font-weight: 650;
    margin: 24px 0 8px;
    color: var(--text-heading, #f8fafc);
  }
  article p, article li {
    font-size: 0.95rem;
    line-height: 1.7;
    color: var(--text, #e2e8f0);
  }
  article ol {
    padding-left: 20px;
    margin: 0;
  }
  article li {
    margin: 6px 0;
  }

  .action {
    display: inline-block;
    margin-top: 24px;
    padding: 10px 18px;
    background: var(--accent, #6366f1);
    color: #fff;
    border-radius: 8px;
    text-decoration: none;
    font-weight: 600;
    font-size: 0.9rem;
  }

  /* Assistant Sidebar */
  .assistant {
    padding: 20px;
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 16px;
    background: var(--surface, #14171c);
  }
  .assistant header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .assistant h2 {
    font-size: 1.15rem;
    margin: 0;
  }
  .quiet {
    border: none;
    background: transparent;
    color: var(--text-muted, #94a3b8);
    font-size: 0.8rem;
    cursor: pointer;
  }
  .quiet:hover {
    color: var(--text, #e2e8f0);
  }
  .muted {
    color: var(--text-muted, #94a3b8);
    margin-top: 6px;
  }

  .messages {
    max-height: 380px;
    overflow-y: auto;
    margin: 16px 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .message {
    padding: 10px 12px;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.05);
    font-size: 0.85rem;
    line-height: 1.5;
  }
  .human {
    font-weight: 650;
    color: var(--accent, #6366f1);
    margin-bottom: 4px;
  }
  .assistant-label {
    font-weight: 650;
    color: #10b981;
    margin-bottom: 4px;
  }
  .body {
    white-space: pre-wrap;
    word-break: break-word;
  }

  form label {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 0.85rem;
  }
  textarea {
    width: 100%;
    box-sizing: border-box;
    font: inherit;
    color: var(--text, #e2e8f0);
    background: var(--bg, #0b0c0e);
    border: 1px solid var(--separator, rgba(255, 255, 255, 0.1));
    border-radius: 8px;
    padding: 10px;
    resize: vertical;
  }
  .send {
    margin-top: 10px;
    display: flex;
    justify-content: flex-end;
  }
  .primary {
    padding: 8px 16px;
    border: none;
    border-radius: 8px;
    background: var(--accent, #6366f1);
    color: #fff;
    font: inherit;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
  }
  .primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  @media (max-width: 900px) {
    .help-grid {
      grid-template-columns: 1fr;
    }
    .guides {
      grid-template-columns: 1fr;
    }
  }
</style>
