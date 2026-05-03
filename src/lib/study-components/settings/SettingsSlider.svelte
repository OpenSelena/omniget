<script lang="ts">
  type Props = {
    value: number;
    min: number;
    max: number;
    step?: number;
    onChange: (next: number) => void;
    debounceMs?: number;
  };

  let { value, min, max, step = 1, onChange, debounceMs = 500 }: Props = $props();
  let local = $state(value);
  let timer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    if (value !== local && timer == null) {
      local = value;
    }
  });

  function flush(next: number) {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = null;
      onChange(next);
    }, debounceMs);
  }

  function onInput(e: Event) {
    const v = Number((e.target as HTMLInputElement).value);
    local = v;
    flush(v);
  }

  $effect(() => {
    return () => {
      if (timer) {
        clearTimeout(timer);
        onChange(local);
      }
    };
  });
</script>

<input
  type="range"
  {min}
  {max}
  {step}
  value={local}
  oninput={onInput}
  class="slider"
  aria-valuemin={min}
  aria-valuemax={max}
  aria-valuenow={local}
/>

<style>
  .slider {
    width: 160px;
    height: 18px;
    accent-color: var(--accent);
  }
</style>
