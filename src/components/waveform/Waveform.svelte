<!--
  FICHIER : trimlab/src/components/waveform/Waveform.svelte
  ROLE    : Canvas waveform interactif — rendu sur demande uniquement (pas de RAF infini)
            Double canvas : staticCanvas (barres + segments, recalculé si données changent)
                           dynamicCanvas (playhead + hover, recalculé si playhead/hover change)
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { playhead, zoom, isPlaying } from "../../lib/store";
  import type { Segment } from "../../lib/store";
  import { formatTimecode } from "../../lib/utils";

  export let segments: Segment[] = [];
  export let duration: number = 0;
  export let waveformPoints: number[] = [];
  export let onSeek: (t: number) => void = () => {};

  // Double canvas : static (barres, segments) + dynamic (playhead, hover)
  let staticCanvas: HTMLCanvasElement;
  let dynamicCanvas: HTMLCanvasElement;
  let container: HTMLDivElement;
  let sCtx: CanvasRenderingContext2D | null = null;
  let dCtx: CanvasRenderingContext2D | null = null;

  let isDragging = false;
  let hoverTime = -1;
  let raf: number | null = null;

  // Dernier état rendu — évite les redraws inutiles
  let lastStaticKey = "";
  let lastDynamicKey = "";

  const C = {
    bg: "#111318",
    keep: "#B8FF3C",
    silence: "rgba(255,75,75,0.5)",
    filler: "rgba(255,184,77,0.65)",
    playhead: "#FFFFFF",
    hover: "rgba(255,255,255,0.15)",
    grid: "rgba(255,255,255,0.04)",
    text: "rgba(255,255,255,0.3)",
  };

  let dpr = 1;
  let W = 0;
  let H = 0;
  let scrollOffset = 0;

  function resize() {
    if (!staticCanvas || !dynamicCanvas || !container) return;
    dpr = window.devicePixelRatio || 1;
    W = container.clientWidth;
    H = container.clientHeight;
    for (const c of [staticCanvas, dynamicCanvas]) {
      c.width = W * dpr;
      c.height = H * dpr;
      c.style.width = W + "px";
      c.style.height = H + "px";
    }
    sCtx = staticCanvas.getContext("2d");
    dCtx = dynamicCanvas.getContext("2d");
    sCtx?.scale(dpr, dpr);
    dCtx?.scale(dpr, dpr);
    // Forcer un redraw complet
    lastStaticKey = "";
    lastDynamicKey = "";
    scheduleRedraw();
  }

  function timeToX(t: number): number {
    return (t / duration) * W * $zoom - scrollOffset;
  }
  function xToTime(x: number): number {
    return ((x + scrollOffset) / (W * $zoom)) * duration;
  }

  // ── Canvas statique : barres + segments ───────────────────────────────────
  function drawStatic() {
    if (!sCtx || W === 0) return;
    sCtx.clearRect(0, 0, W, H);
    sCtx.fillStyle = C.bg;
    sCtx.fillRect(0, 0, W, H);

    // Grid
    sCtx.strokeStyle = C.grid;
    sCtx.lineWidth = 1;
    const step = duration > 600 ? 60 : duration > 60 ? 10 : 5;
    for (let t = 0; t <= duration; t += step) {
      const x = timeToX(t);
      if (x < 0 || x > W) continue;
      sCtx.beginPath();
      sCtx.moveTo(x, 0);
      sCtx.lineTo(x, H - 20);
      sCtx.stroke();
      sCtx.fillStyle = C.text;
      sCtx.font = "9px DM Mono, monospace";
      sCtx.textAlign = "center";
      sCtx.fillText(formatTimecode(t), x, H - 6);
    }

    // Segment backgrounds
    for (const seg of segments) {
      const x1 = timeToX(seg.start_time);
      const x2 = timeToX(seg.end_time);
      if (x2 < 0 || x1 > W) continue;
      const col =
        seg.seg_type === "silence"
          ? C.silence
          : seg.seg_type === "filler"
            ? C.filler
            : null;
      if (col) {
        sCtx.fillStyle = col;
        sCtx.fillRect(x1, 0, x2 - x1, H - 20);
      }
    }

    // Waveform bars
    const mid = (H - 20) / 2;

    // [FIX] Normalisation des points : le backend peut renvoyer des valeurs brutes
    // (PCM int16 ±32768) ou toutes nulles (mock). On calcule la valeur max pour
    // ramener chaque amplitude dans [0, 1] et garantir des barres visibles.
    const rawMax = waveformPoints.reduce((m, v) => Math.max(m, Math.abs(v)), 0);
    const hasRealData = waveformPoints.length > 0 && rawMax > 0;
    const normFactor = rawMax > 0 ? 1 / rawMax : 1;

    if (hasRealData) {
      const barW = Math.max(1, (W * $zoom) / waveformPoints.length);
      const maxAmp = mid * 0.9;
      for (let i = 0; i < waveformPoints.length; i++) {
        const x = (i / waveformPoints.length) * W * $zoom - scrollOffset;
        if (x < -barW || x > W) continue;
        // amplitude normalisée entre 0 et maxAmp — toujours visible
        const amp = Math.abs(waveformPoints[i]) * normFactor * maxAmp;
        const t = (i / waveformPoints.length) * duration;
        const seg = segments.find((s) => t >= s.start_time && t <= s.end_time);
        sCtx.fillStyle =
          seg?.seg_type === "silence"
            ? "rgba(255,75,75,0.7)"
            : seg?.seg_type === "filler"
              ? "rgba(255,184,77,0.8)"
              : C.keep;
        sCtx.fillRect(x, mid - amp, barW * 0.7, amp * 2);
      }
    } else {
      const count = Math.floor(W / 3);
      for (let i = 0; i < count; i++) {
        const x = i * 3;
        const h = 8 + Math.sin(i * 0.3) * 16 + Math.sin(i * 1.1) * 10;
        const t = (x / W) * duration;
        const seg = segments.find((s) => t >= s.start_time && t <= s.end_time);
        sCtx.fillStyle =
          seg?.seg_type === "silence"
            ? "rgba(255,75,75,0.45)"
            : seg?.seg_type === "filler"
              ? "rgba(255,184,77,0.6)"
              : "rgba(184,255,60,0.6)";
        sCtx.fillRect(x, mid - h, 2, h * 2);
      }
    }
  }

  // ── Canvas dynamique : playhead + hover (ultra-léger) ─────────────────────
  function drawDynamic(ph: number) {
    if (!dCtx || W === 0) return;
    dCtx.clearRect(0, 0, W, H);

    // Hover
    if (hoverTime >= 0) {
      const hx = timeToX(hoverTime);
      dCtx.fillStyle = C.hover;
      dCtx.fillRect(hx - 1, 0, 2, H - 20);
      dCtx.fillStyle = "rgba(255,255,255,0.7)";
      dCtx.font = "9px DM Mono, monospace";
      dCtx.textAlign = hx > W / 2 ? "right" : "left";
      dCtx.fillText(formatTimecode(hoverTime), hx + (hx > W / 2 ? -4 : 4), 12);
    }

    // Playhead
    const px = timeToX(ph);
    if (px >= 0 && px <= W) {
      dCtx.strokeStyle = C.playhead;
      dCtx.lineWidth = 1.5;
      dCtx.shadowColor = "rgba(255,255,255,0.5)";
      dCtx.shadowBlur = 4;
      dCtx.beginPath();
      dCtx.moveTo(px, 0);
      dCtx.lineTo(px, H - 20);
      dCtx.stroke();
      dCtx.shadowBlur = 0;
      dCtx.fillStyle = C.playhead;
      dCtx.beginPath();
      dCtx.moveTo(px - 6, 0);
      dCtx.lineTo(px + 6, 0);
      dCtx.lineTo(px, 10);
      dCtx.closePath();
      dCtx.fill();
    }
  }

  // ── Scheduler : ne redessine que si l'état a changé ───────────────────────
  function scheduleRedraw() {
    if (raf !== null) return;
    raf = requestAnimationFrame(() => {
      raf = null;

      // Static : recalcule si zoom, scroll, segments ou waveform ont changé
      const staticKey = `${$zoom}|${scrollOffset}|${segments.length}|${waveformPoints.length}|${W}|${H}`;
      if (staticKey !== lastStaticKey) {
        drawStatic();
        lastStaticKey = staticKey;
      }

      // Dynamic : recalcule si playhead ou hover a changé
      const ph = $playhead;
      const dynamicKey = `${ph.toFixed(3)}|${hoverTime.toFixed(3)}`;
      if (dynamicKey !== lastDynamicKey) {
        drawDynamic(ph);
        lastDynamicKey = dynamicKey;
      }
    });
  }

  // ── RAF continu seulement pendant la lecture ───────────────────────────────
  let playingRaf: number | null = null;
  function startPlayingLoop() {
    if (playingRaf !== null) return;
    const loop = () => {
      const ph = $playhead;
      const dynamicKey = `${ph.toFixed(3)}|${hoverTime.toFixed(3)}`;
      if (dynamicKey !== lastDynamicKey) {
        drawDynamic(ph);
        lastDynamicKey = dynamicKey;
      }
      if ($isPlaying) {
        playingRaf = requestAnimationFrame(loop);
      } else {
        playingRaf = null;
      }
    };
    playingRaf = requestAnimationFrame(loop);
  }

  // Réagir aux changements de store
  import { derived } from "svelte/store";
  const triggerStatic = derived([zoom], () => ({}));
  $: {
    $triggerStatic;
    scheduleRedraw();
  }
  $: {
    segments;
    waveformPoints;
    duration;
    scheduleRedraw();
  }
  $: {
    if ($isPlaying) startPlayingLoop();
    else scheduleRedraw();
  }

  function handleMouseMove(e: MouseEvent) {
    const rect = dynamicCanvas.getBoundingClientRect();
    hoverTime = Math.max(0, Math.min(duration, xToTime(e.clientX - rect.left)));
    scheduleRedraw();
  }
  function handleMouseLeave() {
    hoverTime = -1;
    scheduleRedraw();
  }

  function handleClick(e: MouseEvent) {
    const rect = dynamicCanvas.getBoundingClientRect();
    const t = Math.max(0, Math.min(duration, xToTime(e.clientX - rect.left)));
    playhead.set(t);
    onSeek(t);
    scheduleRedraw();
  }

  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    if (e.ctrlKey || e.metaKey) {
      zoom.update((z) =>
        Math.max(1, Math.min(20, z * (e.deltaY > 0 ? 0.9 : 1.1))),
      );
    } else {
      scrollOffset = Math.max(0, scrollOffset + e.deltaX + e.deltaY * 0.5);
    }
    scheduleRedraw();
  }

  let ro: ResizeObserver;
  onMount(() => {
    resize();
    ro = new ResizeObserver(resize);
    ro.observe(container);
  });
  onDestroy(() => {
    if (raf !== null) cancelAnimationFrame(raf);
    if (playingRaf !== null) cancelAnimationFrame(playingRaf);
    ro?.disconnect();
  });
</script>

<div class="waveform-wrap" bind:this={container}>
  <!-- Canvas statique (barres, segments) -->
  <canvas
    bind:this={staticCanvas}
    style="position:absolute;inset:0;display:block;"
  ></canvas>
  <!-- Canvas dynamique (playhead, hover) — capte les events souris -->
  <canvas
    bind:this={dynamicCanvas}
    style="position:absolute;inset:0;display:block;cursor:crosshair;"
    on:mousemove={handleMouseMove}
    on:mouseleave={handleMouseLeave}
    on:click={handleClick}
    on:wheel={handleWheel}
  ></canvas>

  {#if hoverTime >= 0}
    {@const seg = segments.find(
      (s) => hoverTime >= s.start_time && hoverTime <= s.end_time,
    )}
    {#if seg && seg.seg_type !== "keep"}
      <div class="seg-tooltip" style="left:{timeToX(hoverTime)}px">
        <span class="seg-type {seg.seg_type}">
          {seg.seg_type === "silence" ? "Silence" : `Filler: "${seg.label}"`}
        </span>
        <span class="seg-conf">{Math.round(seg.confidence * 100)}%</span>
      </div>
    {/if}
  {/if}
</div>

<style>
  .waveform-wrap {
    position: relative;
    width: 100%;
    height: var(--waveform-height, 96px);
    background: var(--bg-surface);
    border-radius: var(--radius);
    overflow: hidden;
    border: 1px solid var(--border);
  }
  .seg-tooltip {
    position: absolute;
    top: 4px;
    transform: translateX(-50%);
    background: var(--bg-overlay);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    padding: 3px 8px;
    font-size: 10px;
    display: flex;
    gap: 6px;
    align-items: center;
    pointer-events: none;
    z-index: 10;
    white-space: nowrap;
  }
  .seg-type.silence {
    color: var(--danger);
  }
  .seg-type.filler {
    color: var(--warning);
  }
  .seg-conf {
    color: var(--text-muted);
    font-family: var(--font-mono);
  }
</style>
