<script lang="ts">
  type Props = {
    value: boolean;
    onChange: (next: boolean) => void;
    ariaLabel?: string;
  };

  let { value, onChange, ariaLabel }: Props = $props();
  let local = $state(value);

  $effect(() => {
    if (value !== local) local = value;
  });

  function toggle() {
    const next = !local;
    local = next;
    onChange(next);
  }
</script>

<button
  type="button"
  role="switch"
  class="toggle"
  class:on={local}
  aria-checked={local}
  aria-label={ariaLabel}
  onclick={toggle}
>
  <span class="dot"></span>
</button>

<style>
  .toggle {
    width: 38px;
    height: 22px;
    border-radius: 12px;
    background: color-mix(in oklab, var(--content-border) 70%, transparent);
    border: none;
    padding: 0;
    cursor: pointer;
    position: relative;
    transition: background var(--tg-duration-fast, 150ms);
  }

  .toggle.on {
    background: var(--accent);
  }

  .dot {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: white;
    transition: transform var(--tg-duration-fast, 150ms);
    box-shadow: 0 1px 3px color-mix(in oklab, black 20%, transparent);
  }

  .toggle.on .dot {
    transform: translateX(16px);
  }
</style>
