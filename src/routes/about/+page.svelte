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

    const cards = [
        { href: "/about/changelog", titleKey: "about.card_changelog_title", descKey: "about.card_changelog_desc" },
        { href: "/about/project", titleKey: "about.card_project_title", descKey: "about.card_project_desc" },
        { href: "/about/terms", titleKey: "about.card_terms_title", descKey: "about.card_terms_desc" },
    ] as const;

    async function openAuthorGithub(e: Event) {
        e.preventDefault();
        await open("https://github.com/OpenSelena");
    }

    async function openProjectGithub(e: Event) {
        e.preventDefault();
        await open("https://github.com/OpenSelena/omniget");
    }

    async function openTelegram(e: Event) {
        e.preventDefault();
        await open("https://t.me/OpenSelena");
    }
</script>

<div class="about-overview">
    <header class="about-hero">
        <img src="/favicon.png" alt="" class="about-app-icon" width="64" height="64" draggable="false" />
        <div class="about-identity">
            <div class="about-name-row">
                <h1>OmniGet</h1>
                {#if version}
                    <span class="tag about-version">{$t("about.version")} {version}</span>
                {/if}
            </div>
            <p class="about-tagline">{$t("about.tagline")}</p>
            <p class="about-desc">{$t("about.description")}</p>
            {#if buildDetails}
                <span class="about-build">{buildDetails}</span>
            {/if}
        </div>
    </header>

    <div class="about-cards">
        {#each cards as card}
            <a href={card.href} class="surface-card interactive about-card">
                <span class="list-row">
                    <span class="list-row-content">
                        <span class="list-row-title">{$t(card.titleKey)}</span>
                        <span class="list-row-sub about-card-desc">{$t(card.descKey)}</span>
                    </span>
                    <span class="list-row-trailing about-card-chevron" aria-hidden="true">›</span>
                </span>
            </a>
        {/each}
    </div>

    <div class="about-external">
        <a href="https://github.com/OpenSelena/omniget" target="_blank" rel="noopener" class="btn about-ext-link" onclick={openProjectGithub}>
            <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"/>
            </svg>
            {$t("about.star_button")}
        </a>
        <a href="https://t.me/OpenSelena" target="_blank" rel="noopener" class="btn about-ext-link" onclick={openTelegram}>
            <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
                <path d="M11.944 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0a12 12 0 0 0-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 0 1 .171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.48.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635z"/>
            </svg>
            Telegram
        </a>
    </div>

    <footer class="about-footer">
        <hr class="separator" />
        <p class="about-credit">{$t("about.credit")}</p>
        <a href="https://github.com/OpenSelena" class="about-watermark" onclick={openAuthorGithub} title="@OpenSelena">
            @OpenSelena
        </a>
    </footer>
</div>

<style>
    .about-overview {
        display: flex;
        flex-direction: column;
        gap: var(--space-6);
    }

    .about-hero {
        display: flex;
        align-items: flex-start;
        gap: var(--space-5);
        padding: var(--space-2) 0;
    }

    .about-app-icon {
        width: 72px;
        height: 72px;
        border-radius: 17px;
        object-fit: cover;
        box-shadow: 0 8px 20px rgba(var(--shadow-ink), var(--elev-alpha-2)), 0 0 0 var(--hairline) var(--content-border);
        flex-shrink: 0;
    }

    .about-identity {
        display: flex;
        flex-direction: column;
        gap: var(--space-2);
        min-width: 0;
    }

    .about-name-row {
        display: flex;
        align-items: center;
        gap: var(--space-3);
        flex-wrap: wrap;
    }

    .about-identity h1 {
        font-family: var(--font-display);
        font-size: var(--text-2xl);
        line-height: var(--leading-2xl);
        font-weight: 700;
        letter-spacing: var(--track-tight);
        margin: 0;
    }

    .about-tagline {
        font-size: var(--text-md);
        line-height: var(--leading-md);
        color: var(--text-muted);
        margin: 0;
    }

    .about-desc {
        font-size: var(--text-base);
        line-height: var(--leading-base);
        color: var(--text-muted);
        margin: 0;
        max-width: 60ch;
    }

    .about-build {
        font-family: var(--font-mono);
        font-size: var(--text-xs);
        color: var(--text-dim);
        user-select: all;
    }

    .about-cards {
        display: flex;
        flex-direction: column;
        gap: 0;
        background: var(--surface);
        border-radius: var(--radius-lg);
        box-shadow: inset 0 0 0 var(--hairline) var(--content-border);
        overflow: hidden;
    }

    .about-card {
        display: block;
        text-decoration: none;
        color: inherit;
        border-radius: 0;
        box-shadow: none;
        position: relative;
    }

    .about-card + .about-card::before {
        content: "";
        position: absolute;
        top: 0;
        left: var(--space-4);
        right: 0;
        height: var(--hairline);
        background: var(--separator);
    }

    .about-card .list-row {
        padding: var(--space-3) var(--space-4);
        border-radius: 0;
    }

    .about-card-desc {
        white-space: normal;
    }

    .about-card-chevron {
        font-size: var(--text-lg);
        line-height: 1;
    }

    .about-external {
        display: flex;
        gap: var(--space-2);
        flex-wrap: wrap;
    }

    .about-ext-link {
        text-decoration: none;
    }

    .about-footer {
        display: flex;
        flex-direction: column;
        gap: var(--space-1);
    }

    .about-footer .separator {
        margin-bottom: var(--space-3);
    }

    .about-credit {
        font-size: var(--text-xs);
        line-height: var(--leading-xs);
        color: var(--text-muted);
        margin: 0;
    }

    .about-watermark {
        font-size: var(--text-xs);
        color: var(--text-dim);
        text-decoration: none;
        width: fit-content;
        transition: color var(--duration-fast) var(--ease-out);
    }

    @media (hover: hover) {
        .about-watermark:hover {
            color: var(--text);
        }
    }

    @media (prefers-reduced-motion: reduce) {
        .about-watermark {
            transition: none;
        }
    }

    @media (max-width: 520px) {
        .about-hero {
            flex-direction: column;
            align-items: center;
            text-align: center;
        }

        .about-name-row {
            justify-content: center;
        }
    }
</style>
