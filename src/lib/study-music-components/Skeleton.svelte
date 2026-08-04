<script lang="ts">
  type Props = {
    width?: string;
    height?: string;
    rounded?: "sm" | "md" | "lg" | "full" | "none";
    block?: boolean;
  };

  let { width = "100%", height = "16px", rounded = "sm", block = false }: Props = $props();

  const radius = $derived(
    rounded === "full"
      ? "999px"
      : rounded === "lg"
        ? "12px"
        : rounded === "md"
          ? "8px"
          : rounded === "sm"
            ? "4px"
            : "0",
  );
</script>

<span
  class="skel"
  class:block
  style:width
  style:height
  style:border-radius={radius}
  aria-hidden="true"
></span>

<style>
  .skel {
    display: inline-block;
    background: linear-gradient(
      90deg,
      color-mix(in oklab, var(--secondary) 8%, transparent) 0%,
      color-mix(in oklab, var(--secondary) 16%, transparent) 50%,
      color-mix(in oklab, var(--secondary) 8%, transparent) 100%
    );
    background-size: 200% 100%;
    animation: skel-shimmer 1.4s ease-in-out infinite;
    vertical-align: middle;
  }
  .block { display: block; }
  @keyframes skel-shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }
</style>
