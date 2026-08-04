<script lang="ts">
  type Option = { value: string; label: string };

  type Props = {
    value: string;
    options: Option[];
    onChange: (next: string) => void;
  };

  let { value, options, onChange }: Props = $props();
</script>

<select
  class="select"
  value={value}
  onchange={(e) => onChange((e.target as HTMLSelectElement).value)}
>
  {#each options as o (o.value)}
    <option value={o.value}>{o.label}</option>
  {/each}
</select>

<style>
  .select {
    padding: 6px 10px;
    background: color-mix(in oklab, var(--content-bg) 80%, var(--accent) 4%);
    border: 1px solid color-mix(in oklab, var(--content-border) 70%, transparent);
    border-radius: 6px;
    color: inherit;
    font-size: 13px;
    cursor: pointer;
  }
</style>
