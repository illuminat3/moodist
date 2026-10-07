<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';

  import Icon from '../components/Icon.svelte';
  import { app, catalog, setSettings } from '../lib/store.svelte';

  let { fullscreen = $bindable(false) }: { fullscreen?: boolean } = $props();

  const ic = catalog.ui;
  const st = $derived(app.s!.settings);
  const styles = [
    { id: 'digital', label: 'Digital' },
    { id: 'serif', label: 'Serif' },
    { id: 'analog', label: 'Analog' },
    { id: 'words', label: 'Words' },
    { id: 'night', label: 'Night' },
  ];

  let now = $state(new Date());

  // Tick once per second only when seconds are visible; otherwise once a minute.
  $effect(() => {
    const perSecond = st.clockSeconds;
    let timer: ReturnType<typeof setTimeout>;
    const tick = () => {
      now = new Date();
      const ms = perSecond ? 1000 - now.getMilliseconds() : 60_000 - (now.getSeconds() * 1000 + now.getMilliseconds());
      timer = setTimeout(tick, ms + 5);
    };
    tick();
    return () => clearTimeout(timer);
  });

  $effect(() => {
    const win = getCurrentWindow();
    win.setFullscreen(fullscreen).catch(() => {});
    return () => {
      if (fullscreen) win.setFullscreen(false).catch(() => {});
    };
  });

  const pad = (n: number) => String(n).padStart(2, '0');
  const hours = $derived(st.clock24h ? now.getHours() : now.getHours() % 12 || 12);
  const hh = $derived(st.clock24h ? pad(hours) : String(hours));
  const ampm = $derived(st.clock24h ? '' : now.getHours() < 12 ? 'AM' : 'PM');
  const date = $derived(now.toLocaleDateString([], { day: 'numeric', month: 'long', weekday: 'long' }));

  const numbers = ['twelve', 'one', 'two', 'three', 'four', 'five', 'six', 'seven', 'eight', 'nine', 'ten', 'eleven'];
  const words = $derived.by(() => {
    const m = Math.round((now.getMinutes() + now.getSeconds() / 60) / 5) * 5;
    let h = now.getHours();
    const next = (x: number) => numbers[(x + 1) % 12];
    const cur = (x: number) => numbers[x % 12];
    const phrases: Record<number, string> = {
      0: `${cur(h)} o’clock`,
      5: `five past ${cur(h)}`,
      10: `ten past ${cur(h)}`,
      15: `quarter past ${cur(h)}`,
      20: `twenty past ${cur(h)}`,
      25: `twenty-five past ${cur(h)}`,
      30: `half past ${cur(h)}`,
      35: `twenty-five to ${next(h)}`,
      40: `twenty to ${next(h)}`,
      45: `quarter to ${next(h)}`,
      50: `ten to ${next(h)}`,
      55: `five to ${next(h)}`,
      60: `${next(h)} o’clock`,
    };
    return `It’s ${phrases[m]}`;
  });

  const angles = $derived({
    h: ((now.getHours() % 12) + now.getMinutes() / 60) * 30,
    m: (now.getMinutes() + now.getSeconds() / 60) * 6,
    s: now.getSeconds() * 6,
  });
</script>

<div class="clock-page" class:full={fullscreen} class:night={st.clockStyle === 'night'}>
  {#if !fullscreen}
    <div class="options">
      <div class="styles" role="radiogroup" aria-label="Clock style">
        {#each styles as s (s.id)}
          <button
            aria-checked={st.clockStyle === s.id}
            class:on={st.clockStyle === s.id}
            onclick={() => setSettings({ clockStyle: s.id })}
            role="radio"
          >
            {s.label}
          </button>
        {/each}
      </div>
      <div class="toggles">
        <label><input checked={st.clock24h} class="switch" onchange={e => setSettings({ clock24h: e.currentTarget.checked })} type="checkbox" /> 24-hour</label>
        <label><input checked={st.clockSeconds} class="switch" onchange={e => setSettings({ clockSeconds: e.currentTarget.checked })} type="checkbox" /> Seconds</label>
        <label><input checked={st.clockDate} class="switch" onchange={e => setSettings({ clockDate: e.currentTarget.checked })} type="checkbox" /> Date</label>
        <button class="btn ghost small" onclick={() => (fullscreen = true)}>Fullscreen</button>
      </div>
    </div>
  {/if}

  <button
    aria-label={fullscreen ? 'Exit fullscreen' : 'Enter fullscreen'}
    class="face {st.clockStyle}"
    ondblclick={() => (fullscreen = !fullscreen)}
    title="Double-click for fullscreen"
  >
    {#if st.clockStyle === 'analog'}
      <svg class="dial" viewBox="0 0 200 200">
        <circle class="rim" cx="100" cy="100" r="96" />
        {#each Array(60) as _, i (i)}
          <line
            class:major={i % 5 === 0}
            transform="rotate({i * 6} 100 100)"
            x1="100"
            x2="100"
            y1="8"
            y2={i % 5 === 0 ? 20 : 13}
          />
        {/each}
        <line class="hand hour" transform="rotate({angles.h} 100 100)" x1="100" x2="100" y1="110" y2="52" />
        <line class="hand minute" transform="rotate({angles.m} 100 100)" x1="100" x2="100" y1="114" y2="26" />
        {#if st.clockSeconds}
          <line class="hand second" transform="rotate({angles.s} 100 100)" x1="100" x2="100" y1="120" y2="20" />
        {/if}
        <circle class="pin" cx="100" cy="100" r="3.5" />
      </svg>
    {:else if st.clockStyle === 'words'}
      <span class="words">{words}</span>
    {:else}
      <span class="time">
        {hh}<span class="colon">:</span>{pad(now.getMinutes())}{#if st.clockSeconds}<span class="secs">{pad(now.getSeconds())}</span>{/if}
        {#if ampm}<span class="ampm">{ampm}</span>{/if}
      </span>
    {/if}
    {#if st.clockDate}
      <span class="date">{date}</span>
    {/if}
  </button>

  {#if fullscreen}
    <button aria-label="Exit fullscreen" class="icon-btn exit" onclick={() => (fullscreen = false)}>
      <Icon svg={ic.close} />
    </button>
  {/if}
</div>

<style>
  .clock-page {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 420px;

    &.full {
      background-color: var(--color-background);
    }

    &.night.full {
      background-color: #000;
    }
  }

  .options {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
    padding: 16px 24px;
    border-bottom: 1px solid var(--color-border);
  }

  .styles {
    display: flex;
    border: 1px solid var(--color-border-strong);

    & button {
      height: 32px;
      padding: 0 14px;
      font-size: var(--font-xsm);
      color: var(--color-foreground-subtle);
      cursor: pointer;
      background: none;
      border: none;

      &:hover {
        color: var(--color-foreground);
      }

      &.on {
        color: var(--color-inverted-foreground);
        background-color: var(--color-inverted-background);
      }
    }
  }

  .toggles {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    align-items: center;
    font-size: var(--font-xsm);
    color: var(--color-foreground-subtle);

    & label {
      display: flex;
      gap: 8px;
      align-items: center;
      cursor: pointer;
    }
  }

  .face {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 18px;
    align-items: center;
    justify-content: center;
    padding: 24px;
    color: var(--color-foreground);
    cursor: default;
    background: none;
    border: none;
    outline: none;
  }

  .time {
    display: flex;
    align-items: baseline;
    font-family: var(--font-heading);
    font-size: clamp(64px, 16vw, 220px);
    font-weight: 600;
    line-height: 1;
    letter-spacing: -0.03em;
    font-variant-numeric: tabular-nums;
  }

  .colon {
    margin: 0 0.02em;
    color: var(--color-foreground-subtler);
  }

  .secs {
    margin-left: 0.25em;
    font-size: 0.32em;
    color: var(--color-foreground-subtle);
  }

  .ampm {
    margin-left: 0.3em;
    font-size: 0.22em;
    font-weight: 500;
    color: var(--color-foreground-subtle);
    letter-spacing: 0.05em;
  }

  .date {
    font-size: clamp(14px, 2vw, 22px);
    color: var(--color-foreground-subtle);
  }

  .serif {
    & .time {
      font-family: var(--font-display);
      font-weight: 400;
      letter-spacing: -0.01em;
    }

    & .date {
      font-family: var(--font-display);
      font-style: italic;
    }
  }

  .night {
    & .time {
      font-weight: 500;
      color: #7f1d1d;
    }

    & .colon,
    & .secs,
    & .ampm,
    & .date {
      color: #571515;
    }
  }

  .words {
    max-width: 12ch;
    font-family: var(--font-display);
    font-size: clamp(40px, 8vw, 104px);
    font-weight: 500;
    line-height: 1.1;
    text-align: center;
  }

  .dial {
    width: min(60vh, 70vw, 440px);
    height: auto;

    & .rim {
      fill: var(--color-surface);
      stroke: var(--color-border-strong);
      stroke-width: 1;
    }

    & line {
      stroke: var(--color-foreground-subtler);
      stroke-width: 1;

      &.major {
        stroke: var(--color-foreground-subtle);
        stroke-width: 2;
      }
    }

    & .hand {
      stroke: var(--color-foreground);
      stroke-linecap: round;
    }

    & .hour {
      stroke-width: 5;
    }

    & .minute {
      stroke-width: 3;
    }

    & .second {
      stroke: var(--color-foreground-subtle);
      stroke-width: 1.2;
    }

    & .pin {
      fill: var(--color-foreground);
    }
  }

  .exit {
    position: absolute;
    top: 16px;
    right: 16px;
    opacity: 0.3;

    &:hover {
      opacity: 1;
    }
  }
</style>
