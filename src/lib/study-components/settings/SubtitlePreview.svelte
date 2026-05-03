<script lang="ts">
  type Props = {
    text?: string;
    size: number;
    textColor: string;
    backgroundColor: string;
    outlineColor: string;
    opacity: number;
    font: string;
    bold: boolean;
  };

  let {
    text = "Exemplo de legenda",
    size,
    textColor,
    backgroundColor,
    outlineColor,
    opacity,
    font,
    bold,
  }: Props = $props();

  const fontFamily = $derived.by(() => {
    if (font === "serif") return 'Georgia, "Times New Roman", serif';
    if (font === "sans") return 'system-ui, -apple-system, "Segoe UI", sans-serif';
    return "system-ui, sans-serif";
  });

  const opacityFactor = $derived(Math.max(0, Math.min(1, opacity / 100)));
</script>

<div class="canvas">
  <span
    class="cap"
    style:font-family={fontFamily}
    style:font-size={`${(size / 100) * 18}px`}
    style:font-weight={bold ? "700" : "500"}
    style:color={textColor}
    style:background-color={`color-mix(in oklab, ${backgroundColor} ${opacityFactor * 100}%, transparent)`}
    style:text-shadow={`-1px -1px 0 ${outlineColor}, 1px -1px 0 ${outlineColor}, -1px 1px 0 ${outlineColor}, 1px 1px 0 ${outlineColor}`}
  >
    {text}
  </span>
</div>

<style>
  .canvas {
    width: 100%;
    aspect-ratio: 16 / 6;
    background: linear-gradient(120deg, #1a1a1a, #0a0a0a 60%, #2a1a2a);
    background-size: cover;
    border-radius: 8px;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding-bottom: 14px;
    overflow: hidden;
  }

  .cap {
    display: inline-block;
    padding: 4px 10px;
    border-radius: 3px;
    line-height: 1.3;
    max-width: 80%;
    text-align: center;
  }
</style>
