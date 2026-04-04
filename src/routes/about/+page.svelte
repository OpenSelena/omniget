<script lang="ts">
    import { t } from "$lib/i18n";
</script>

<div class="about-page">
    <div class="about-hero">
        <img src="/loop.png" alt="Loop" class="about-loop" draggable="false" />
        <h1>OmniGet</h1>
        <p class="about-tagline">{$t("about.tagline")}</p>
    </div>

    <div class="about-links">
        <a href="/about/project" class="about-link">{$t("about.project")}</a>
        <a href="/about/changelog" class="about-link">{$t("about.changelog")}</a>
        <a href="/about/terms" class="about-link">{$t("about.terms")}</a>
        <a href="/about/roadmap" class="about-link">{$t("about.roadmap")}</a>
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

    .about-credit {
        font-size: 12px;
        color: var(--tertiary);
        margin: 0;
    }
</style>
