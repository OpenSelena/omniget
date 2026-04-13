<script lang="ts">
    import { t } from "$lib/i18n";
    import { getVersion } from "@tauri-apps/api/app";
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

    <div class="about-links">
        <a href="/about/project" class="about-link">{$t("about.tab.project")}</a>
        <a href="/about/changelog" class="about-link">{$t("about.tab.changelog")}</a>
        <a href="/about/terms" class="about-link">{$t("about.tab.terms")}</a>
        <a href="/about/roadmap" class="about-link">{$t("about.tab.roadmap")}</a>
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
        width: 120px;
        height: 120px;
        border-radius: 50%;
        object-fit: cover;
        pointer-events: none;
        user-select: none;
    }

    .about-hero h1 {
        font-size: 24px;
        font-weight: 600;
        margin: 0;
    }

    .about-tagline {
        font-size: 14px;
        color: var(--tertiary);
        margin: 0;
    }

    .about-desc {
        font-size: 13px;
        color: var(--secondary);
        margin: 0;
        max-width: 340px;
    }

    .about-version {
        font-size: 12px;
        color: var(--tertiary);
        background: var(--button);
        padding: 3px 10px;
        border-radius: var(--border-radius);
    }

    .about-build {
        font-family: var(--font-mono, ui-monospace, SFMono-Regular, Menlo, Consolas, monospace);
        font-size: 10.5px;
        color: var(--tertiary);
        opacity: 0.75;
        letter-spacing: 0.3px;
        user-select: all;
    }

    .about-links {
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
        justify-content: center;
    }

    .about-link {
        padding: 8px 16px;
        background: var(--button);
        border-radius: var(--border-radius);
        color: var(--secondary);
        font-size: 13px;
        font-weight: 500;
        text-decoration: none;
        box-shadow: var(--button-box-shadow);
    }

    .about-link:hover {
        background: var(--button-hover);
    }

    .about-external {
        display: flex;
        gap: 12px;
        justify-content: center;
    }

    .about-ext-link {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 6px 14px;
        background: var(--button-elevated);
        border: 1px solid var(--button-stroke);
        border-radius: var(--border-radius);
        color: var(--secondary);
        font-size: 13px;
        text-decoration: none;
        transition: background 0.15s ease;
    }

    .about-ext-link:hover {
        background: var(--button-hover);
    }

    .about-ext-link svg {
        flex-shrink: 0;
    }

    .about-credit {
        font-size: 12px;
        color: var(--tertiary);
        margin: 0;
    }
</style>
