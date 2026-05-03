<script lang="ts">
    import { t } from "$lib/i18n";
    import { getVersion } from "@tauri-apps/api/app";
    import { open } from "@tauri-apps/plugin-shell";
    import { BUILD_INFO } from "$lib/build-info";

    let version = $state("");

    $effect(() => {
        getVersion().then(v => { version = v; }).catch(() => {});
    });

    const buildDetails = $derived(
        [BUILD_INFO.commitShort, BUILD_INFO.branch, BUILD_INFO.date]
            .filter((part) => part && part !== "unknown")
            .join(" · ")
    );

    async function openAuthorGithub(e: Event) {
        e.preventDefault();
        await open("https://github.com/tonhowtf");
    }
</script>

<div class="about-page">
    <div class="about-hero">
        <img src="/loop.png" alt="Loop" class="about-loop" draggable="false" />
        <h1>OmniGet</h1>
        <p class="about-tagline">{$t("about.tagline")}</p>
        <p class="about-desc">{$t("about.description")}</p>
        {#if version}
            <span class="about-version">{$t("about.version")} {version}</span>
        {/if}
        {#if buildDetails}
            <span class="about-build">{buildDetails}</span>
        {/if}
    </div>

    <div class="about-external">
        <a href="https://github.com/tonhowtf/omniget" target="_blank" rel="noopener" class="about-ext-link">
            <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"/>
            </svg>
            {$t("about.star_button")}
        </a>
        <a href="https://discord.gg/jgdxyPy7Vn" target="_blank" rel="noopener" class="about-ext-link">
            <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M18.9 5.3a16.6 16.6 0 0 0-4.1-1.3 12.2 12.2 0 0 0-.5 1.1 15.4 15.4 0 0 0-4.6 0A12.2 12.2 0 0 0 9.2 4a16.6 16.6 0 0 0-4.1 1.3A17.3 17.3 0 0 0 2 17.2a16.7 16.7 0 0 0 5.1 2.6 12.5 12.5 0 0 0 1.1-1.8 10.8 10.8 0 0 1-1.7-.8l.4-.3a11.9 11.9 0 0 0 10.2 0l.4.3a10.8 10.8 0 0 1-1.7.8 12.5 12.5 0 0 0 1.1 1.8 16.7 16.7 0 0 0 5.1-2.6A17.3 17.3 0 0 0 18.9 5.3zM8.7 14.8c-1 0-1.8-.9-1.8-2s.8-2 1.8-2 1.8.9 1.8 2-.8 2-1.8 2zm6.6 0c-1 0-1.8-.9-1.8-2s.8-2 1.8-2 1.8.9 1.8 2-.8 2-1.8 2z"/>
            </svg>
            Discord
        </a>
    </div>

    <p class="about-credit">{$t("about.credit")}</p>

    <a href="https://github.com/tonhowtf" class="about-watermark" onclick={openAuthorGithub} title="@tonhowtf">
        @tonhowtf
    </a>
</div>

<style>
    .about-page {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: calc(var(--padding) * 2);
        padding: calc(var(--padding) * 3);
        text-align: center;
    }

    .about-hero {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: var(--padding);
    }

    .about-loop {
        width: 200px;
        height: 200px;
        border-radius: var(--radius-full);
        object-fit: cover;
        pointer-events: none;
        user-select: none;
    }

    .about-hero h1 {
        font-family: var(--font-display);
        font-size: var(--text-display);
        line-height: var(--leading-display);
        font-weight: 600;
        letter-spacing: -0.03em;
        margin: 0;
    }

    .about-tagline {
        font-size: var(--text-md);
        color: var(--text-muted);
        margin: 0;
    }

    .about-desc {
        font-size: var(--text-sm);
        color: var(--text);
        margin: 0;
        max-width: 360px;
    }

    .about-version {
        font-size: var(--text-xs);
        color: var(--text-dim);
        background: var(--surface);
        padding: var(--space-1) var(--space-3);
        border-radius: var(--radius-full);
    }

    .about-build {
        font-family: var(--font-mono);
        font-size: var(--text-xs);
        color: var(--text-dim);
        opacity: 0.75;
        letter-spacing: 0.3px;
        user-select: all;
    }

    .about-external {
        display: flex;
        gap: 12px;
        justify-content: center;
    }

    .about-ext-link {
        display: flex;
        align-items: center;
        gap: var(--space-2);
        padding: var(--space-2) var(--space-4);
        background: var(--surface);
        border: 1px solid var(--border);
        border-radius: var(--radius-sm);
        color: var(--text);
        font-size: var(--text-sm);
        text-decoration: none;
        transition: background var(--duration-fast) var(--ease-out), transform var(--duration-fast) var(--ease-out);
    }

    @media (hover: hover) {
        .about-ext-link:hover {
            background: var(--surface-hi);
            transform: translateY(-1px);
        }
    }

    @media (prefers-reduced-motion: reduce) {
        .about-ext-link {
            transition: background var(--duration-fast) var(--ease-out);
        }
        .about-ext-link:hover {
            transform: none;
        }
    }

    .about-ext-link svg {
        flex-shrink: 0;
    }

    .about-credit {
        font-size: 12px;
        color: var(--tertiary);
        margin: 0;
    }

    .about-watermark {
        font-size: var(--text-xs);
        font-weight: 400;
        color: var(--text-dim);
        opacity: 0.5;
        text-decoration: none;
        margin-top: var(--space-3);
        transition: opacity var(--duration-fast) var(--ease-out);
    }

    @media (hover: hover) {
        .about-watermark:hover {
            opacity: 0.9;
        }
    }
</style>
