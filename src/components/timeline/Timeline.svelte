<!--
  FICHIER : trimlab/src/components/timeline/Timeline.svelte
  ROLE    : Timeline visuelle drag-resize + liste segments éditables au clic
-->
<script lang="ts">
  import type { Segment } from "../../lib/store";
  import { playhead, isPlaying } from "../../lib/store";
  import { formatTimecode, formatDuration } from "../../lib/utils";

  export let segments: Segment[] = [];
  export let duration: number = 0;
  export let onToggle: (id: string) => void = () => {};
  export let onDelete: (id: string) => void = () => {};
  export let onTrim: (
    id: string,
    start: number,
    end: number,
  ) => void = () => {};

  // ── Stats ────────────────────────────────────────────────────────────────────
  $: kept = segments.filter((s) => s.seg_type === "keep");
  $: keptDuration = kept.reduce((a, s) => a + (s.end_time - s.start_time), 0);
  $: cutDuration = duration - keptDuration;
  $: savings = duration > 0 ? (cutDuration / duration) * 100 : 0;
  $: silenceCount = segments.filter((s) => s.seg_type === "silence").length;
  $: fillerCount = segments.filter((s) => s.seg_type === "filler").length;
  // [FIX] Rust ne produit jamais seg_type="cut" — Coupé = tout segment non-keep
  $: cutCount = segments.filter((s) => s.seg_type !== "keep").length;

  // ── Filtre ───────────────────────────────────────────────────────────────────
  type FilterType = "all" | "keep" | "silence" | "filler" | "cut";
  let filterType: FilterType = "all";
  $: visible =
    filterType === "all"
      ? segments
      : segments.filter((s) => s.seg_type === filterType);

  // ── Segment sélectionné (édition inline) ────────────────────────────────────
  let selectedId: string | null = null;
  $: selectedSeg = segments.find((s) => s.id === selectedId) ?? null;
  let editStart = "";
  let editEnd = "";

  function selectSeg(seg: Segment) {
    if (selectedId === seg.id) {
      selectedId = null;
      return;
    }
    selectedId = seg.id;
    const t = trimValues[seg.id] ?? {
      start: seg.start_time,
      end: seg.end_time,
    };
    editStart = t.start.toFixed(3);
    editEnd = t.end.toFixed(3);
  }

  // ── trimValues : synchro depuis store ────────────────────────────────────────
  let trimValues: Record<string, { start: number; end: number }> = {};
  $: {
    const next: Record<string, { start: number; end: number }> = {};
    for (const s of segments) {
      if (dragging && dragging.segId === s.id) {
        next[s.id] = trimValues[s.id] ?? {
          start: s.start_time,
          end: s.end_time,
        };
      } else {
        next[s.id] = { start: s.start_time, end: s.end_time };
      }
    }
    trimValues = next;
  }

  function tv(seg: Segment) {
    return trimValues[seg.id] ?? { start: seg.start_time, end: seg.end_time };
  }

  // ── Timeline visuelle : drag resize ─────────────────────────────────────────
  let visTimelineEl: HTMLElement | null = null;

  function pxToSec(px: number): number {
    if (!visTimelineEl || !duration) return 0;
    return (px / visTimelineEl.clientWidth) * duration;
  }

  let dragging: null | {
    segId: string;
    side: "start" | "end";
    origVal: number;
    startX: number;
  } = null;

  function startDrag(e: MouseEvent, seg: Segment, side: "start" | "end") {
    e.stopPropagation();
    e.preventDefault();
    const t = trimValues[seg.id] ?? {
      start: seg.start_time,
      end: seg.end_time,
    };
    dragging = {
      segId: seg.id,
      side,
      origVal: side === "start" ? t.start : t.end,
      startX: e.clientX,
    };
    window.addEventListener("mousemove", onDragMove);
    window.addEventListener("mouseup", onDragEnd);
  }

  function onDragMove(e: MouseEvent) {
    if (!dragging) return;
    const delta = pxToSec(e.clientX - dragging.startX);
    const idx = segments.findIndex((s) => s.id === dragging!.segId);
    if (idx === -1) return;
    const t = trimValues[dragging.segId];
    if (!t) return;
    const minDur = 0.1;
    const prevEnd = idx > 0 ? segments[idx - 1].end_time + 0.01 : 0;
    const nextStart =
      idx < segments.length - 1
        ? segments[idx + 1].start_time - 0.01
        : duration;

    if (dragging.side === "start") {
      trimValues[dragging.segId] = {
        ...t,
        start: Math.max(
          prevEnd,
          Math.min(t.end - minDur, dragging.origVal + delta),
        ),
      };
    } else {
      trimValues[dragging.segId] = {
        ...t,
        end: Math.max(
          t.start + minDur,
          Math.min(nextStart, dragging.origVal + delta),
        ),
      };
    }
    trimValues = { ...trimValues };
  }

  function onDragEnd() {
    if (!dragging) return;
    const t = trimValues[dragging.segId];
    onTrim(dragging.segId, t.start, t.end);
    dragging = null;
    window.removeEventListener("mousemove", onDragMove);
    window.removeEventListener("mouseup", onDragEnd);
  }

  // Clic sur la timeline → seek
  function onTimelineClick(e: MouseEvent) {
    if (dragging || !visTimelineEl || !duration) return;
    const rect = visTimelineEl.getBoundingClientRect();
    const t = ((e.clientX - rect.left) / rect.width) * duration;
    playhead.set(Math.max(0, Math.min(duration, t)));
  }

  $: playheadPct = duration > 0 ? ($playhead / duration) * 100 : 0;

  function onTimelineKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" || e.key === " ") {
      const target = e.currentTarget as HTMLElement;
      const rect = target.getBoundingClientRect();
      const cx = rect.left + rect.width / 2;
      playhead.set(
        Math.max(
          0,
          Math.min(duration, ((cx - rect.left) / rect.width) * duration),
        ),
      );
    }
  }

  function segLabel(type: string) {
    return (
      (
        {
          keep: "Parole",
          silence: "Silence",
          filler: "Filler",
          cut: "Coupé",
        } as any
      )[type] ?? type
    );
  }

  function setFilter(k: string | number) {
    filterType = String(k) as FilterType;
  }
</script>

<div class="timeline">
  <!-- ── STATS ── -->
  <div class="summary">
    <div class="summary-item">
      <span class="summary-label">Original</span>
      <span class="summary-value font-mono">{formatDuration(duration)}</span>
    </div>
    <span class="summary-arrow">→</span>
    <div class="summary-item">
      <span class="summary-label">Final</span>
      <span class="summary-value font-mono text-accent"
        >{formatDuration(keptDuration)}</span
      >
    </div>
    <div class="summary-divider"></div>
    <div class="summary-item">
      <span class="summary-label">Économie</span>
      <span class="summary-value font-mono text-success"
        >−{savings.toFixed(1)}%</span
      >
    </div>
    <div class="summary-divider"></div>
    <div class="summary-item">
      <span class="summary-label">Coupé</span>
      <span class="summary-value font-mono">{formatDuration(cutDuration)}</span>
    </div>
  </div>

  <!-- ── TIMELINE VISUELLE ── -->
  <div class="vis-timeline-wrap">
    <!-- Légende -->
    <div class="vis-legend">
      <span class="vis-legend-hint"
        >← Glisse les bords pour ajuster · Clique pour sélectionner</span
      >
    </div>

    <div
      class="vis-timeline"
      bind:this={visTimelineEl}
      on:click={onTimelineClick}
      on:keydown={onTimelineKeydown}
      role="presentation"
    >
      {#each segments as seg (seg.id)}
        {#if duration > 0}
          {@const t = tv(seg)}
          {@const left = (t.start / duration) * 100}
          {@const width = Math.max(0.3, ((t.end - t.start) / duration) * 100)}
          {@const selected = selectedId === seg.id}
          {@const isDragging = dragging?.segId === seg.id}

          <div
            class="vis-seg {seg.seg_type}"
            class:selected
            class:is-dragging={isDragging}
            style="left:{left}%; width:{width}%;"
            title="{segLabel(seg.seg_type)} — {formatTimecode(
              t.start,
            )} → {formatTimecode(t.end)}"
            on:click|stopPropagation={() => selectSeg(seg)}
            on:keydown={(e) => e.key === "Enter" && selectSeg(seg)}
            role="button"
            tabindex="0"
          >
            <!-- Handle gauche -->
            <div
              class="vis-handle vis-handle-left"
              on:mousedown|stopPropagation={(e) => startDrag(e, seg, "start")}
              role="presentation"
            ></div>

            {#if width > 3}
              <span class="vis-seg-label">{segLabel(seg.seg_type)}</span>
            {/if}

            <!-- Handle droit -->
            <div
              class="vis-handle vis-handle-right"
              on:mousedown|stopPropagation={(e) => startDrag(e, seg, "end")}
              role="presentation"
            ></div>
          </div>
        {/if}
      {/each}

      <!-- Playhead -->
      <div class="vis-playhead" style="left:{playheadPct}%;">
        <div class="vis-playhead-head"></div>
      </div>
    </div>

    <!-- Axe timecodes -->
    <div class="vis-axis">
      {#each [0, 0.25, 0.5, 0.75, 1] as pct}
        <span class="vis-tick font-mono" style="left:{pct * 100}%">
          {formatTimecode(pct * duration)}
        </span>
      {/each}
    </div>
  </div>

  <!-- ── PANNEAU ÉDITION INLINE ── -->
  {#if selectedSeg}
    {@const t = tv(selectedSeg)}
    <div class="edit-panel">
      <span class="edit-panel-label {selectedSeg.seg_type}"
        >{segLabel(selectedSeg.seg_type)}</span
      >

      <div class="edit-fields">
        <label class="edit-field">
          <span>Début (s)</span>
          <input
            type="number"
            step="0.01"
            min="0"
            bind:value={editStart}
            on:change={() => {
              const s = parseFloat(editStart);
              if (!isNaN(s) && s < t.end) onTrim(selectedSeg.id, s, t.end);
            }}
          />
        </label>
        <span class="edit-arrow">→</span>
        <label class="edit-field">
          <span>Fin (s)</span>
          <input
            type="number"
            step="0.01"
            bind:value={editEnd}
            on:change={() => {
              const e = parseFloat(editEnd);
              if (!isNaN(e) && e > t.start) onTrim(selectedSeg.id, t.start, e);
            }}
          />
        </label>
        <span class="edit-dur font-mono"
          >= {formatDuration(t.end - t.start)}</span
        >
      </div>

      <div class="edit-actions">
        <button
          class="btn btn-ghost btn-xs"
          on:click={() => {
            playhead.set(t.start);
            isPlaying.set(true);
          }}>▶ Écouter</button
        >

        {#if selectedSeg.seg_type !== "keep"}
          <button
            class="btn btn-ghost btn-xs btn-restore"
            on:click={() => {
              onToggle(selectedSeg.id);
              selectedId = null;
            }}>✓ Garder</button
          >
        {:else}
          <button
            class="btn btn-ghost btn-xs btn-cut"
            on:click={() => {
              onToggle(selectedSeg.id);
              selectedId = null;
            }}>✂ Couper</button
          >
        {/if}

        <button
          class="btn btn-ghost btn-xs btn-delete"
          on:click={() => {
            onDelete(selectedSeg.id);
            selectedId = null;
          }}>🗑</button
        >

        <button
          class="btn btn-ghost btn-xs"
          on:click={() => (selectedId = null)}>✕</button
        >
      </div>
    </div>
  {/if}

  <!-- ── FILTRES ── -->
  <div class="filter-tabs">
    {#each [["all", "Tous", segments.length], ["keep", "Parole", kept.length], ["silence", "Silence", silenceCount], ["filler", "Filler", fillerCount], ["cut", "Coupé", cutCount]] as [k, label, count]}
      <button
        class="filter-tab"
        class:active={filterType === k}
        on:click={() => setFilter(k)}
      >
        {label}
        <span class="filter-count">{count}</span>
      </button>
    {/each}
  </div>

  <!-- ── LISTE SEGMENTS ── -->
  <div class="seg-list">
    {#if visible.length === 0}
      <div class="seg-empty">
        {segments.length === 0
          ? "Aucun segment — lance l'analyse d'abord"
          : `Aucun segment "${filterType}"`}
      </div>
    {:else}
      {#each visible as seg (seg.id)}
        {@const t = tv(seg)}
        {@const selected = selectedId === seg.id}
        <div
          class="seg-row {seg.seg_type}"
          class:selected
          class:is-cut={seg.seg_type === "cut"}
          on:click={() => selectSeg(seg)}
          role="button"
          tabindex="0"
          on:keydown={(e) => e.key === "Enter" && selectSeg(seg)}
        >
          <div class="seg-indicator {seg.seg_type}"></div>
          <span class="seg-type-badge {seg.seg_type}"
            >{segLabel(seg.seg_type)}</span
          >
          <span class="seg-timecodes font-mono">
            {formatTimecode(t.start)} → {formatTimecode(t.end)}
          </span>
          <span class="seg-duration text-muted font-mono"
            >{formatDuration(t.end - t.start)}</span
          >
          {#if seg.label}
            <span class="seg-label-chip" title={seg.label}>{seg.label}</span>
          {/if}

          <div
            class="seg-actions"
            on:click|stopPropagation
            on:keydown|stopPropagation
            role="presentation"
          >
            <button
              class="btn-seg-action"
              title="Écouter"
              on:click={() => {
                playhead.set(t.start);
                isPlaying.set(true);
              }}>▶</button
            >
            {#if seg.seg_type !== "keep"}
              <button
                class="btn-seg-action btn-restore"
                title="Garder"
                on:click={() => onToggle(seg.id)}>✓</button
              >
            {:else}
              <button
                class="btn-seg-action btn-cut"
                title="Couper"
                on:click={() => onToggle(seg.id)}>✂</button
              >
            {/if}
            <button
              class="btn-seg-action btn-delete"
              title="Supprimer"
              on:click={() => onDelete(seg.id)}>🗑</button
            >
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .timeline {
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: 100%;
    overflow: hidden;
  }

  /* ── STATS ── */
  .summary {
    display: flex;
    align-items: center;
    gap: 14px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 8px 14px;
    flex-wrap: wrap;
    flex-shrink: 0;
  }
  .summary-item {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .summary-label {
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
  }
  .summary-value {
    font-size: 15px;
    font-weight: 600;
  }
  .summary-arrow {
    color: var(--text-muted);
    font-size: 12px;
  }
  .summary-divider {
    width: 1px;
    height: 28px;
    background: var(--border);
  }

  /* ── TIMELINE VISUELLE ── */
  .vis-timeline-wrap {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .vis-legend {
    display: flex;
    justify-content: flex-end;
  }
  .vis-legend-hint {
    font-size: 9px;
    color: var(--text-muted);
    letter-spacing: 0.03em;
  }

  .vis-timeline {
    position: relative;
    height: 52px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
    cursor: crosshair;
    user-select: none;
  }

  /* Segments */
  .vis-seg {
    position: absolute;
    top: 5px;
    height: calc(100% - 10px);
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: visible;
    cursor: pointer;
    transition:
      filter 0.1s,
      box-shadow 0.1s;
    box-sizing: border-box;
  }
  .vis-seg:hover {
    filter: brightness(1.15);
  }
  .vis-seg.selected {
    box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.8);
    z-index: 3;
  }
  .vis-seg.is-dragging {
    z-index: 10;
    opacity: 0.9;
  }

  .vis-seg.keep {
    background: rgba(184, 255, 60, 0.5);
    border: 1px solid rgba(184, 255, 60, 0.8);
  }
  .vis-seg.silence {
    background: rgba(255, 75, 75, 0.4);
    border: 1px solid rgba(255, 75, 75, 0.7);
  }
  .vis-seg.filler {
    background: rgba(255, 184, 77, 0.4);
    border: 1px solid rgba(255, 184, 77, 0.7);
  }
  .vis-seg.cut {
    background: rgba(120, 120, 120, 0.2);
    border: 1px solid rgba(120, 120, 120, 0.4);
  }

  .vis-seg-label {
    font-size: 9px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: rgba(255, 255, 255, 0.65);
    pointer-events: none;
    white-space: nowrap;
    overflow: hidden;
  }

  /* Handles */
  .vis-handle {
    position: absolute;
    top: 0;
    width: 10px;
    height: 100%;
    cursor: ew-resize;
    z-index: 5;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition:
      opacity 0.12s,
      background 0.12s;
  }
  .vis-handle::after {
    content: "";
    width: 2px;
    height: 55%;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.85);
  }
  .vis-handle-left {
    left: 0;
    border-radius: 4px 0 0 4px;
    background: rgba(0, 0, 0, 0.2);
  }
  .vis-handle-right {
    right: 0;
    border-radius: 0 4px 4px 0;
    background: rgba(0, 0, 0, 0.2);
  }
  .vis-seg:hover .vis-handle,
  .vis-seg.selected .vis-handle {
    opacity: 1;
  }
  .vis-handle:hover {
    background: rgba(0, 0, 0, 0.45) !important;
    opacity: 1 !important;
  }

  /* Playhead */
  .vis-playhead {
    position: absolute;
    top: 0;
    width: 1px;
    height: 100%;
    background: rgba(255, 255, 255, 0.9);
    pointer-events: none;
    z-index: 20;
    transform: translateX(-50%);
  }
  .vis-playhead-head {
    position: absolute;
    top: -1px;
    left: 50%;
    transform: translateX(-50%);
    width: 7px;
    height: 7px;
    background: white;
    border-radius: 50%;
  }

  /* Axe */
  .vis-axis {
    position: relative;
    height: 14px;
  }
  .vis-tick {
    position: absolute;
    font-size: 9px;
    color: var(--text-muted);
    transform: translateX(-50%);
  }
  .vis-tick:first-child {
    transform: translateX(0);
  }
  .vis-tick:last-child {
    transform: translateX(-100%);
  }

  /* ── PANNEAU ÉDITION INLINE ── */
  .edit-panel {
    flex-shrink: 0;
    background: var(--bg-elevated);
    border: 1px solid var(--border-active);
    border-radius: var(--radius);
    padding: 10px 14px;
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    animation: slideIn 0.15s ease;
  }
  @keyframes slideIn {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  .edit-panel-label {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: 2px 7px;
    border-radius: 3px;
    flex-shrink: 0;
  }
  .edit-panel-label.keep {
    background: rgba(184, 255, 60, 0.1);
    color: var(--accent);
  }
  .edit-panel-label.silence {
    background: rgba(255, 75, 75, 0.1);
    color: var(--danger);
  }
  .edit-panel-label.filler {
    background: rgba(255, 184, 77, 0.1);
    color: var(--warning);
  }
  .edit-panel-label.cut {
    background: rgba(120, 120, 120, 0.1);
    color: var(--text-muted);
  }

  .edit-fields {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .edit-field {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .edit-field span {
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }
  .edit-field input {
    width: 82px;
    background: var(--bg-overlay);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 3px 6px;
    outline: none;
    transition: border-color 0.1s;
  }
  .edit-field input:focus {
    border-color: var(--accent);
  }
  .edit-arrow {
    color: var(--text-muted);
    font-size: 12px;
  }
  .edit-dur {
    font-size: 11px;
    color: var(--text-muted);
  }
  .edit-actions {
    display: flex;
    gap: 4px;
    margin-left: auto;
  }

  .btn-xs {
    padding: 3px 8px !important;
    font-size: 11px !important;
    height: 24px;
  }
  .btn-restore:hover {
    color: var(--accent) !important;
    border-color: var(--accent) !important;
  }
  .btn-cut:hover {
    color: var(--danger) !important;
    border-color: var(--danger) !important;
  }
  .btn-delete:hover {
    color: var(--danger) !important;
    border-color: var(--danger) !important;
  }

  /* ── FILTRES ── */
  .filter-tabs {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
    flex-wrap: wrap;
  }
  .filter-tab {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 4px 9px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
    transition: all 0.12s;
  }
  .filter-tab:hover {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }
  .filter-tab.active {
    background: var(--bg-elevated);
    border-color: var(--border);
    color: var(--text-primary);
  }
  .filter-count {
    background: var(--bg-overlay);
    border-radius: 100px;
    padding: 1px 5px;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-muted);
  }

  /* ── LISTE ── */
  .seg-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .seg-empty {
    color: var(--text-muted);
    font-size: 12px;
    text-align: center;
    padding: 24px;
  }

  .seg-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    transition:
      background 0.1s,
      border-color 0.1s;
    font-size: 12px;
    cursor: pointer;
  }
  .seg-row:hover {
    background: var(--bg-elevated);
    border-color: var(--border);
  }
  .seg-row.selected {
    background: var(--bg-elevated);
    border-color: var(--border-active);
  }
  .seg-row.is-cut {
    opacity: 0.5;
  }

  .seg-indicator {
    width: 3px;
    height: 22px;
    border-radius: 2px;
    flex-shrink: 0;
  }
  .seg-indicator.keep {
    background: var(--accent);
  }
  .seg-indicator.silence {
    background: var(--danger);
  }
  .seg-indicator.filler {
    background: var(--warning);
  }
  .seg-indicator.cut {
    background: var(--text-muted);
  }

  .seg-type-badge {
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: 2px 5px;
    border-radius: 3px;
    flex-shrink: 0;
    min-width: 50px;
    text-align: center;
  }
  .seg-type-badge.keep {
    background: rgba(184, 255, 60, 0.1);
    color: var(--accent);
  }
  .seg-type-badge.silence {
    background: rgba(255, 75, 75, 0.1);
    color: var(--danger);
  }
  .seg-type-badge.filler {
    background: rgba(255, 184, 77, 0.1);
    color: var(--warning);
  }
  .seg-type-badge.cut {
    background: rgba(120, 120, 120, 0.1);
    color: var(--text-muted);
  }

  .seg-timecodes {
    font-size: 11px;
    color: var(--text-secondary);
  }
  .seg-duration {
    font-size: 10px;
    margin-left: auto;
  }

  .seg-label-chip {
    font-size: 9px;
    padding: 1px 5px;
    border-radius: 3px;
    background: rgba(255, 184, 77, 0.15);
    color: var(--warning);
    border: 1px solid rgba(255, 184, 77, 0.3);
    max-width: 80px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .seg-actions {
    display: flex;
    gap: 2px;
  }
  .btn-seg-action {
    width: 26px;
    height: 26px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    cursor: pointer;
    font-size: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.1s;
    color: var(--text-secondary);
  }
  .btn-seg-action:hover {
    border-color: var(--border-strong);
    color: var(--text-primary);
  }
  .btn-seg-action.btn-cut:hover {
    background: rgba(255, 75, 75, 0.1);
    border-color: var(--danger);
    color: var(--danger);
  }
  .btn-seg-action.btn-restore:hover {
    background: rgba(184, 255, 60, 0.1);
    border-color: var(--accent);
    color: var(--accent);
  }
  .btn-seg-action.btn-delete:hover {
    background: rgba(255, 75, 75, 0.1);
    border-color: var(--danger);
    color: var(--danger);
  }
</style>
