<!--
  FICHIER : src/components/subtitles/SubtitleEditor.svelte
  ROLE    : Éditeur de sous-titres avec preview live, style, export SRT/ASS/burn
  VERSION : 1.3 (Fix burn-in edited blocks)
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    playhead,
    isPlaying,
    canExport,
    showLicenceModal,
  } from "../../lib/store";
  import { get } from "svelte/store";
  import { commands } from "../../lib/commands";
  import { formatTimecode } from "../../lib/utils";
  import { open } from "@tauri-apps/plugin-shell";
  // @ts-ignore
  import SubtitleStylePanel from "./SubtitleStylePanel.svelte";

  export let projectId: string;
  export let videoPath: string;
  export let onClose: () => void = () => {};

  interface Segment {
    start_time: number;
    end_time: number;
    seg_type: string;
  }

  export let segments: Segment[] = [];

  function remapTime(t: number, keeps: Segment[]): number | null {
    let offset = 0;
    for (const seg of keeps) {
      if (t <= seg.end_time) {
        return offset + (Math.max(t, seg.start_time) - seg.start_time);
      }
      offset += seg.end_time - seg.start_time;
    }
    return offset;
  }

  function remapBlock(b: SubtitleBlock, keeps: Segment[]) {
    if (keeps.length === 0)
      return {
        start: b.start,
        end: b.end,
        text: b.text,
        words: b.words,
        edited: b.edited,
      };
    const newStart = remapTime(b.start, keeps);
    const newEnd = remapTime(b.end, keeps);
    if (newStart === null && newEnd === null) return null;
    const start = newStart ?? 0;
    const end = newEnd ?? start + (b.end - b.start);
    const words = b.words
      .map((w) => {
        const ws = remapTime(w.start, keeps);
        const we = remapTime(w.end, keeps);
        if (ws === null) return null;
        return { ...w, start: ws, end: we ?? ws + (w.end - w.start) };
      })
      .filter((w): w is Word & { start: number; end: number } => w !== null);
    return { start, end, text: b.text, words, edited: b.edited };
  }

  function applyStyleToSrtText(text: string): string {
    let t = text;
    if (style.bold) t = `<b>${t}</b>`;
    if (style.italic) t = `<i>${t}</i>`;
    if (style.color && style.color.toLowerCase() !== "#ffffff") {
      t = `<font color="${style.color}">${t}</font>`;
    }
    return t;
  }

  interface Word {
    word: string;
    start: number;
    end: number;
    confidence: number;
  }

  interface SubtitleBlock {
    id: string;
    words: Word[];
    text: string;
    start: number;
    end: number;
    edited: boolean;
  }

  interface SubtitleStyle {
    fontFamily: string;
    fontSize: number;
    color: string;
    outlineColor: string;
    outlineWidth: number;
    bgColor: string;
    bgOpacity: number;
    position: "bottom" | "top" | "middle";
    bold: boolean;
    italic: boolean;
    karaokeHighlight: string;
    karaokeEnabled: boolean;
    animation: "none" | "fade" | "pop";
    maxWordsPerBlock: number;
    maxCharsPerLine: number;
  }

  // ── State ────────────────────────────────────────────────────────────────────
  let words: Word[] = [];
  let blocks: SubtitleBlock[] = [];
  let style: SubtitleStyle = {
    fontFamily: "Inter",
    fontSize: 32,
    color: "#FFFFFF",
    outlineColor: "#000000",
    outlineWidth: 2,
    bgColor: "#000000",
    bgOpacity: 0.45,
    position: "bottom",
    bold: true,
    italic: false,
    karaokeHighlight: "#B8FF3C",
    karaokeEnabled: true,
    animation: "fade",
    maxWordsPerBlock: 4,
    maxCharsPerLine: 32,
  };

  let loading = true;
  let exporting = false;
  let exportStep = "";
  let lastExportPath = "";
  let selectedBlock: string | null = null;
  let editingBlock: string | null = null;
  let editText = "";
  let showStyle = true;
  let previewCollapsed = false;

  // Canvas overlay
  let canvasEl: HTMLCanvasElement;
  let canvasCtx: CanvasRenderingContext2D | null = null;
  let containerEl: HTMLDivElement;
  let rafId: number | null = null;
  let resizeObserver: ResizeObserver | null = null;

  // Drag & drop
  let dragSrcId: string | null = null;
  let dragOverId: string | null = null;
  let isDragging = false;

  function onBlockMouseDown(blockId: string) {
    dragSrcId = blockId;
    isDragging = false;
  }

  function onBlockMouseEnter(blockId: string) {
    if (!dragSrcId || dragSrcId === blockId) return;
    isDragging = true;
    dragOverId = blockId;
  }

  function onBlockMouseLeave(blockId: string) {
    if (dragOverId === blockId) dragOverId = null;
  }

  function onBlockMouseUp(blockId: string) {
    if (isDragging && dragSrcId && dragSrcId !== blockId) {
      const srcIdx = blocks.findIndex((b) => b.id === dragSrcId);
      const destIdx = blocks.findIndex((b) => b.id === blockId);
      if (srcIdx !== -1 && destIdx !== -1) {
        const updated = [...blocks];
        const [moved] = updated.splice(srcIdx, 1);
        updated.splice(destIdx, 0, moved);
        blocks = updated;
        saveBlocksToBackend();
      }
    }
    dragSrcId = null;
    dragOverId = null;
    isDragging = false;
  }

  function onWindowMouseUp() {
    if (dragSrcId) {
      dragSrcId = null;
      dragOverId = null;
      isDragging = false;
    }
  }

  function focusOnMount(el: HTMLElement) {
    requestAnimationFrame(() => el.focus());
    return {};
  }

  // ── Chargement & Persistence ─────────────────────────────────────────────────

  async function saveBlocksToBackend() {
    if (!projectId) return;
    try {
      await commands.saveSubtitleBlocks(projectId, JSON.stringify(blocks));
    } catch (e) {
      console.error("[SubtitleEditor] Erreur sauvegarde:", e);
    }
  }

  onMount(async () => {
    window.addEventListener("mouseup", onWindowMouseUp);
    loading = true;

    try {
      // 1. Charger les blocs sauvegardés depuis la DB
      const savedJson = await commands.getSubtitleBlocks(projectId);

      if (savedJson) {
        try {
          const parsed = JSON.parse(savedJson);
          if (Array.isArray(parsed) && parsed.length > 0) {
            blocks = parsed;
            words = blocks.flatMap((b) => b.words || []);
          }
        } catch (e) {
          console.error("Erreur parsing saved blocks", e);
        }
      }

      // 2. Si aucun bloc sauvegardé, générer depuis Whisper
      if (blocks.length === 0) {
        try {
          words = await commands.getTranscriptWords(projectId);
        } catch (_) {
          // La table transcript_words n'existe pas encore (pas de transcription effectuée)
          words = [];
        }
        if (words.length > 0) {
          blocks = groupWordsIntoBlocks(
            words,
            style.maxWordsPerBlock,
            style.maxCharsPerLine,
          );
          await saveBlocksToBackend();
        }
      }
    } catch (e) {
      console.error("[SubtitleEditor] Erreur chargement:", e);
    }

    loading = false;
    startRenderLoop();

    if (containerEl) {
      resizeObserver = new ResizeObserver(() => {
        if (canvasEl && containerEl) {
          canvasEl.width = containerEl.clientWidth;
          canvasEl.height = containerEl.clientHeight;
        }
      });
      resizeObserver.observe(containerEl);
    }
  });

  onDestroy(() => {
    window.removeEventListener("mouseup", onWindowMouseUp);
    if (rafId !== null) cancelAnimationFrame(rafId);
    resizeObserver?.disconnect();
  });

  // ── Regroupement mots → blocs ────────────────────────────────────────────────
  function groupWordsIntoBlocks(
    ws: Word[],
    maxWords: number,
    maxChars: number,
  ): SubtitleBlock[] {
    const result: SubtitleBlock[] = [];
    let current: Word[] = [];
    let charCount = 0;

    const flush = () => {
      if (current.length === 0) return;
      const text = current.map((w) => w.word.trim()).join(" ");
      result.push({
        id: `sub_${result.length}_${Date.now()}`,
        words: [...current],
        text,
        start: current[0].start,
        end: current[current.length - 1].end,
        edited: false,
      });
      current = [];
      charCount = 0;
    };

    for (const w of ws) {
      const wLen = w.word.trim().length + 1;
      const gap =
        current.length > 0 ? w.start - current[current.length - 1].end : 0;

      if (
        current.length >= maxWords ||
        charCount + wLen > maxChars ||
        gap > 0.8
      ) {
        flush();
      }
      current.push(w);
      charCount += wLen;
    }
    flush();
    return result;
  }

  // ── Reset ─────────────────────────────────────────────────────────────────────
  // [FIX] Modale de confirmation Svelte native — aucune dépendance plugin-dialog,
  // aucun risque de resolve immédiat. Les blocs NE sont effacés QUE si l'utilisateur
  // clique explicitement le bouton de confirmation dans la modale.
  let showResetConfirm = false;

  async function confirmReset() {
    showResetConfirm = false;
    try {
      await commands.deleteProjectWords(projectId);
      await commands.saveSubtitleBlocks(projectId, "[]");
    } catch (e) {
      console.error("[SubtitleEditor] Erreur reset:", e);
    }
    words = [];
    blocks = [];
    exportStep = "";
    lastExportPath = "";
  }

  // ── Edition d'un bloc ────────────────────────────────────────────────────────
  function startEdit(block: SubtitleBlock) {
    const fresh = blocks.find((b) => b.id === block.id) ?? block;
    editingBlock = fresh.id;
    editText = fresh.text;
    selectedBlock = fresh.id;
  }

  async function commitEdit(block: SubtitleBlock) {
    blocks = blocks.map((b) =>
      b.id === block.id ? { ...b, text: editText, edited: true } : b,
    );
    editingBlock = null;
    await saveBlocksToBackend();
  }

  async function deleteBlock(id: string) {
    blocks = blocks.filter((b) => b.id !== id);
    await saveBlocksToBackend();
  }

  async function splitBlock(block: SubtitleBlock) {
    const mid = Math.floor(block.words.length / 2);
    if (mid === 0) return;
    const a = block.words.slice(0, mid);
    const b = block.words.slice(mid);
    const newBlocks = blocks.flatMap((bl) => {
      if (bl.id !== block.id) return [bl];
      return [
        {
          id: `${bl.id}_a_${Date.now()}`,
          words: a,
          text: a.map((w) => w.word.trim()).join(" "),
          start: a[0].start,
          end: a[a.length - 1].end,
          edited: false,
        },
        {
          id: `${bl.id}_b_${Date.now()}`,
          words: b,
          text: b.map((w) => w.word.trim()).join(" "),
          start: b[0].start,
          end: b[b.length - 1].end,
          edited: false,
        },
      ];
    });
    blocks = newBlocks;
    await saveBlocksToBackend();
  }

  async function mergeWithNext(block: SubtitleBlock) {
    const idx = blocks.findIndex((b) => b.id === block.id);
    if (idx < 0 || idx >= blocks.length - 1) return;
    const next = blocks[idx + 1];
    const merged: SubtitleBlock = {
      id: block.id,
      words: [...block.words, ...next.words],
      text: block.text + " " + next.text,
      start: block.start,
      end: next.end,
      edited: true,
    };
    blocks = [...blocks.slice(0, idx), merged, ...blocks.slice(idx + 2)];
    await saveBlocksToBackend();
  }

  // ── Canvas overlay ───────────────────────────────────────────────────────────
  function getActiveBlock(t: number): SubtitleBlock | null {
    return blocks.find((b) => t >= b.start && t <= b.end) ?? null;
  }

  function getActiveWordIndex(block: SubtitleBlock, t: number): number {
    for (let i = 0; i < block.words.length; i++) {
      if (t >= block.words[i].start && t <= block.words[i].end) return i;
    }
    for (let i = 0; i < block.words.length; i++) {
      if (t < block.words[i].start) return i - 1;
    }
    return block.words.length - 1;
  }

  function renderSubtitle(ctx: CanvasRenderingContext2D, t: number) {
    const W = ctx.canvas.width;
    const H = ctx.canvas.height;
    ctx.clearRect(0, 0, W, H);

    const block = getActiveBlock(t);
    if (!block) return;

    const blockAge = t - block.start;
    const blockDur = block.end - block.start;
    let alpha = 1;
    if (style.animation === "fade") {
      if (blockAge < 0.15) alpha = blockAge / 0.15;
      else if (blockAge > blockDur - 0.15) alpha = (blockDur - blockAge) / 0.15;
    } else if (style.animation === "pop") {
      alpha = blockAge < 0.1 ? blockAge / 0.1 : 1;
    }
    alpha = Math.max(0, Math.min(1, alpha));

    const fontSize = style.fontSize * (W / 640);
    ctx.font = `${style.bold ? "bold" : "normal"} ${style.italic ? "italic" : "normal"} ${fontSize}px ${style.fontFamily}, sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "alphabetic";

    const useKaraoke = style.karaokeEnabled && !block.edited;

    if (!useKaraoke) {
      const text = block.text;
      const textW = ctx.measureText(text).width;
      const padH = fontSize * 0.3;
      const padV = fontSize * 0.2;
      const yBase =
        style.position === "bottom"
          ? H * 0.88
          : style.position === "top"
            ? H * 0.12
            : H * 0.5;

      if (style.bgOpacity > 0) {
        ctx.globalAlpha = alpha * style.bgOpacity;
        ctx.fillStyle = style.bgColor;
        ctx.beginPath();
        ctx.roundRect(
          W / 2 - textW / 2 - padH,
          yBase - fontSize - padV,
          textW + padH * 2,
          fontSize + padV * 2,
          6,
        );
        ctx.fill();
      }
      ctx.globalAlpha = alpha;
      ctx.strokeStyle = style.outlineColor;
      ctx.lineWidth = style.outlineWidth * 2;
      ctx.lineJoin = "round";
      ctx.strokeText(text, W / 2, yBase);
      ctx.fillStyle = style.color;
      ctx.fillText(text, W / 2, yBase);
    } else {
      const activeWordIdx = getActiveWordIndex(block, t);
      const words = block.words;
      const wordWidths = words.map(
        (w) => ctx.measureText(w.word.trim() + " ").width,
      );
      const totalW = wordWidths.reduce((a, b) => a + b, 0);
      const yBase =
        style.position === "bottom"
          ? H * 0.88
          : style.position === "top"
            ? H * 0.12
            : H * 0.5;
      const padH = fontSize * 0.3;
      const padV = fontSize * 0.2;

      if (style.bgOpacity > 0) {
        ctx.globalAlpha = alpha * style.bgOpacity;
        ctx.fillStyle = style.bgColor;
        ctx.beginPath();
        ctx.roundRect(
          W / 2 - totalW / 2 - padH,
          yBase - fontSize - padV,
          totalW + padH * 2,
          fontSize + padV * 2,
          6,
        );
        ctx.fill();
      }

      ctx.globalAlpha = alpha;
      let x = W / 2 - totalW / 2;

      for (let i = 0; i < words.length; i++) {
        const wText = words[i].word.trim() + " ";
        const wW = wordWidths[i];
        const isActive = i === activeWordIdx;
        const isPast = i < activeWordIdx;

        if (isActive && style.karaokeEnabled) {
          const glowAlpha = 0.25;
          ctx.globalAlpha = alpha * glowAlpha;
          ctx.fillStyle = style.karaokeHighlight;
          ctx.beginPath();
          ctx.roundRect(x - 2, yBase - fontSize - 2, wW + 4, fontSize + 8, 4);
          ctx.fill();
          ctx.globalAlpha = alpha;
        }

        const wordColor = isActive
          ? style.karaokeHighlight
          : isPast
            ? style.color + "BB"
            : style.color;

        ctx.strokeStyle = style.outlineColor;
        ctx.lineWidth = style.outlineWidth * 2;
        ctx.lineJoin = "round";
        ctx.textAlign = "left";
        ctx.strokeText(wText, x, yBase);
        ctx.fillStyle = wordColor;
        ctx.fillText(wText, x, yBase);
        x += wW;
      }
      ctx.textAlign = "center";
    }

    ctx.globalAlpha = 1;
  }

  function startRenderLoop() {
    const loop = () => {
      if (canvasCtx && containerEl) {
        const W = containerEl.clientWidth;
        const H = containerEl.clientHeight;
        if (canvasEl.width !== W || canvasEl.height !== H) {
          canvasEl.width = W;
          canvasEl.height = H;
        }
        renderSubtitle(canvasCtx, get(playhead));
      }
      rafId = requestAnimationFrame(loop);
    };
    if (canvasEl && containerEl) {
      canvasEl.width = containerEl.clientWidth;
      canvasEl.height = containerEl.clientHeight;
    }
    rafId = requestAnimationFrame(loop);
  }

  function seekToBlock(block: SubtitleBlock) {
    selectedBlock = block.id;
    const t = block.start + 0.01;
    playhead.set(t);
    if (canvasCtx) renderSubtitle(canvasCtx, t);
  }

  $: if (canvasEl) {
    canvasCtx = canvasEl.getContext("2d");
  }

  // [FIX] Redécouper les blocs quand maxWordsPerBlock ou maxCharsPerLine change
  // (slider OU preset). Ne rebuilde que si les mots sources sont disponibles.
  // ── Regroupement réactif au changement de preset / sliders ──────────────────
  //
  // DOUBLE BUG ORIGINAL :
  //   1. _prevMaxWords = -1 → bloc $: déclenché immédiatement au chargement
  //      (car -1 !== style.maxWordsPerBlock) → blocs DB écrasés
  //   2. Logique "overlap" pour préserver les blocs édités était incorrecte :
  //      un bloc édité [1.0, 3.0] chevauche deux blocs frais [1.0,2.0] et [2.0,3.0]
  //      → dupliqué ou raté selon les timings → ancien texte ré-apparaît
  //
  // FIX CORRECT :
  //   - Initialiser _prevMaxWords à la valeur COURANTE (pas -1)
  //   - Exclure les mots des blocs édités du pool de régénération
  //   - Régénérer uniquement les mots libres avec les nouveaux paramètres
  //   - Réinsérer les blocs édités à leur position chronologique
  //
  let _prevMaxWords = style.maxWordsPerBlock;
  let _prevMaxChars = style.maxCharsPerLine;

  $: if (
    !loading &&
    words.length > 0 &&
    (style.maxWordsPerBlock !== _prevMaxWords ||
      style.maxCharsPerLine !== _prevMaxChars)
  ) {
    _prevMaxWords = style.maxWordsPerBlock;
    _prevMaxChars = style.maxCharsPerLine;

    const edited = blocks.filter((b) => b.edited);

    if (edited.length === 0) {
      // Aucune correction manuelle — régénération totale propre
      blocks = groupWordsIntoBlocks(
        words,
        style.maxWordsPerBlock,
        style.maxCharsPerLine,
      );
    } else {
      // Des blocs ont été édités — exclure leurs mots du pool,
      // régénérer le reste, puis réinsérer les blocs édités en ordre chronologique.
      const editedRanges = edited.map((b) => ({ start: b.start, end: b.end }));
      const freeWords = words.filter(
        (w) =>
          !editedRanges.some(
            (r) => w.start >= r.start - 0.01 && w.end <= r.end + 0.01,
          ),
      );
      const freshFree = groupWordsIntoBlocks(
        freeWords,
        style.maxWordsPerBlock,
        style.maxCharsPerLine,
      );
      // Fusion chronologique : blocs frais + blocs édités intacts
      blocks = [...freshFree, ...edited].sort((a, b) => a.start - b.start);
    }

    saveBlocksToBackend().catch(() => {});
  }

  // ── Export ───────────────────────────────────────────────────────────────────
  async function handleExportSrt() {
    if (!get(canExport)) {
      showLicenceModal.set(true);
      return;
    }
    exporting = true;
    exportStep = "Génération SRT…";
    try {
      const keeps = segments.filter((s) => s.seg_type === "keep");
      const srtBlocks = blocks
        .map((b) => remapBlock(b, keeps))
        .filter((b): b is NonNullable<typeof b> => b !== null)
        .map((b) => ({
          start: b.start,
          end: b.end,
          text: applyStyleToSrtText(b.text),
        }));
      const res = await commands.exportSrt({ projectId, blocks: srtBlocks });
      lastExportPath = res.outputPath;
      exportStep = "✅ SRT exporté !";
    } catch (e) {
      exportStep = `❌ ${e}`;
    }
    exporting = false;
  }

  async function handleExportAss() {
    if (!get(canExport)) {
      showLicenceModal.set(true);
      return;
    }
    exporting = true;
    exportStep = "Génération ASS…";
    try {
      const keeps = segments.filter((s) => s.seg_type === "keep");
      const assBlocks = blocks
        .map((b) => remapBlock(b, keeps))
        .filter((b): b is NonNullable<typeof b> => b !== null)
        .map((b) => ({
          start: b.start,
          end: b.end,
          text: b.text,
          words: b.words,
          edited: b.edited,
        }));
      const res = await commands.exportAss({
        projectId,
        blocks: assBlocks,
        style,
      });
      lastExportPath = res.outputPath;
      exportStep = "✅ ASS exporté !";
    } catch (e) {
      exportStep = `❌ ${e}`;
    }
    exporting = false;
  }

  async function handleBurnIn() {
    if (!get(canExport)) {
      showLicenceModal.set(true);
      return;
    }
    exporting = true;
    exportStep = "Burn-in en cours (FFmpeg)… Cela peut prendre du temps.";
    try {
      const keepSegs = segments
        .filter((s) => s.seg_type === "keep")
        .map((s) => [s.start_time, s.end_time] as [number, number]);
      const result = await commands.burnSubtitles({
        projectId,
        videoPath,
        blocks: blocks.map((b) => ({
          start: b.start,
          end: b.end,
          text: b.text,
          words: b.words,
          edited: b.edited,
        })),
        style,
        keepSegments: keepSegs,
      });
      lastExportPath = result.outputPath;
      exportStep = `✅ Vidéo générée !`;
    } catch (e) {
      exportStep = `❌ Erreur: ${e}`;
    }
    exporting = false;
  }

  function openExportFolder() {
    if (lastExportPath) open(lastExportPath);
  }

  $: activeBlock = getActiveBlock($playhead);
</script>

<div class="subtitle-editor">
  <div class="editor-header">
    <div class="header-left">
      <span class="editor-title font-display">✦ Sous-titres</span>
      <span class="block-count">{blocks.length} blocs</span>
    </div>
    <div class="header-actions">
      <button
        class="btn btn-danger btn-sm"
        on:click={() => (showResetConfirm = true)}
        disabled={exporting}>↺ Réinitialiser</button
      >
      <button
        class="btn btn-ghost btn-sm"
        on:click={() => (showStyle = !showStyle)}
      >
        {showStyle ? "Masquer styles" : "Styles"}
      </button>
      <div class="export-group">
        <button
          class="btn btn-ghost btn-sm"
          on:click={handleExportSrt}
          disabled={exporting || blocks.length === 0}
          title={$canExport ? "Exporter SRT" : "Licence requise"}
          >{$canExport ? "↓ SRT" : "🔒 SRT"}</button
        >
        <button
          class="btn btn-ghost btn-sm"
          on:click={handleExportAss}
          disabled={exporting || blocks.length === 0}
          title={$canExport ? "Exporter ASS" : "Licence requise"}
          >{$canExport ? "↓ ASS" : "🔒 ASS"}</button
        >
        <button
          class="btn btn-primary btn-sm"
          on:click={handleBurnIn}
          disabled={exporting || blocks.length === 0}
          title={$canExport
            ? "Incruster les sous-titres dans la vidéo"
            : "Licence requise"}
          >{$canExport ? "🔥 Burn-in" : "🔒 Burn-in"}</button
        >
      </div>
      <button class="btn btn-ghost btn-sm" on:click={onClose}>✕</button>
    </div>
  </div>

  {#if exportStep}
    <div
      class="export-status"
      class:success={exportStep.startsWith("✅")}
      class:error={exportStep.startsWith("❌")}
    >
      <span>{exportStep}</span>
      {#if exportStep.startsWith("✅") && lastExportPath}
        <button class="btn btn-ghost btn-sm" on:click={openExportFolder}
          >Ouvrir</button
        >
      {/if}
    </div>
  {/if}

  <div class="editor-body">
    <div class="blocks-panel">
      {#if loading}
        <div class="loading-state">
          <div class="spinner"></div>
          <span>Chargement...</span>
        </div>
      {:else if blocks.length === 0}
        <div class="empty-state">
          <div class="empty-icon">🎙</div>
          <div>Aucun sous-titre</div>
        </div>
      {:else}
        <div class="blocks-list">
          {#each blocks as block (block.id)}
            {@const isActive = activeBlock?.id === block.id}
            {@const isSelected = selectedBlock === block.id}
            <button
              class="block-row"
              class:active={isActive}
              class:selected={isSelected}
              class:edited={block.edited}
              class:drag-over={dragOverId === block.id}
              class:dragging={dragSrcId === block.id && isDragging}
              on:click={() => {
                if (!isDragging) seekToBlock(block);
              }}
              on:mousedown={() => onBlockMouseDown(block.id)}
              on:mouseenter={() => onBlockMouseEnter(block.id)}
              on:mouseleave={() => onBlockMouseLeave(block.id)}
              on:mouseup={() => onBlockMouseUp(block.id)}
            >
              <div class="block-timecodes">
                <span class="font-mono">{formatTimecode(block.start)}</span>
                <span class="text-muted">→</span>
                <span class="font-mono">{formatTimecode(block.end)}</span>
                {#if block.edited}<span class="edited-badge">✎</span>{/if}
              </div>

              {#if editingBlock === block.id}
                <textarea
                  class="block-edit-input"
                  bind:value={editText}
                  rows="2"
                  on:mousedown|stopPropagation
                  on:click|stopPropagation
                  on:blur={() => commitEdit(block)}
                  on:keydown={(e) => {
                    if (e.key === "Enter" && !e.shiftKey) {
                      e.preventDefault();
                      commitEdit(block);
                    }
                  }}
                  use:focusOnMount
                ></textarea>
              {:else}
                <div
                  class="block-text"
                  role="button"
                  tabindex="0"
                  on:dblclick={() => startEdit(block)}
                  on:keydown={(e) => e.key === "Enter" && startEdit(block)}
                >
                  {#if isActive && style.karaokeEnabled && !block.edited}
                    {#each block.words as w, i}
                      {@const activeIdx = getActiveWordIndex(block, $playhead)}
                      <span
                        class="word-token"
                        class:word-active={i === activeIdx}
                        class:word-past={i < activeIdx}>{w.word}</span
                      >
                    {/each}
                  {:else}
                    {block.text}
                  {/if}
                </div>
              {/if}

              <div class="block-actions">
                <button
                  class="block-btn"
                  title="Éditer"
                  on:click|stopPropagation={() => startEdit(block)}>✎</button
                >
                <button
                  class="block-btn"
                  title="Couper en deux"
                  on:click|stopPropagation={() => splitBlock(block)}>✂</button
                >
                <button
                  class="block-btn"
                  title="Fusionner avec le suivant"
                  on:click|stopPropagation={() => mergeWithNext(block)}
                  >⊕</button
                >
                <button
                  class="block-btn btn-danger"
                  title="Supprimer"
                  on:click|stopPropagation={() => deleteBlock(block.id)}
                  >×</button
                >
              </div>
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <div class="preview-panel">
      <div class="preview-bar">
        <span class="preview-bar-label">Aperçu</span>
        <button
          class="preview-toggle"
          title={previewCollapsed ? "Agrandir" : "Réduire"}
          on:click={() => (previewCollapsed = !previewCollapsed)}
          >{previewCollapsed ? "▸ Agrandir" : "▾ Réduire"}</button
        >
      </div>
      <div
        class="video-preview-wrap"
        class:collapsed={previewCollapsed}
        bind:this={containerEl}
      >
        <canvas bind:this={canvasEl} class="subtitle-canvas"></canvas>
      </div>
      {#if showStyle}
        <div class="style-panel-wrap">
          <SubtitleStylePanel bind:style />
        </div>
      {/if}
    </div>
  </div>
  <!-- ── Modale de confirmation reset ──────────────────────────────── -->
  {#if showResetConfirm}
    <div class="confirm-overlay" role="dialog" aria-modal="true">
      <div class="confirm-box">
        <div class="confirm-icon">⚠</div>
        <div class="confirm-title">Réinitialiser les sous-titres ?</div>
        <div class="confirm-body">
          Tous les sous-titres et la transcription seront effacés.<br />
          <strong>Cette action est irréversible.</strong>
        </div>
        <div class="confirm-actions">
          <button
            class="btn btn-ghost btn-sm"
            on:click={() => (showResetConfirm = false)}
          >
            Annuler
          </button>
          <button class="btn btn-danger btn-sm" on:click={confirmReset}>
            ↺ Réinitialiser
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .subtitle-editor {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-base);
    overflow: hidden;
    position: relative; /* ancre la modale de confirmation */
  }
  .editor-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    background: var(--bg-surface);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    gap: 12px;
  }
  .header-left {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .editor-title {
    font-size: 15px;
    font-weight: 700;
    color: var(--accent);
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .export-group {
    display: flex;
    gap: 4px;
  }
  .export-status {
    padding: 6px 16px;
    font-size: 12px;
    font-family: var(--font-mono);
    background: var(--bg-elevated);
    border-bottom: 1px solid var(--border);
    color: var(--text-secondary);
    flex-shrink: 0;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .export-status.success {
    color: var(--success);
    background: rgba(60, 255, 160, 0.06);
  }
  .export-status.error {
    color: var(--danger);
    background: rgba(255, 75, 75, 0.06);
  }
  .editor-body {
    display: flex;
    flex: 1;
    overflow: hidden;
  }
  .blocks-panel {
    width: 340px;
    min-width: 280px;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--border);
    overflow: hidden;
  }
  .blocks-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px;
  }
  .loading-state,
  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--text-muted);
    font-size: 13px;
  }
  .spinner {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .block-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 7px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    cursor: grab;
    transition:
      background 0.1s,
      border-color 0.1s,
      opacity 0.1s;
    position: relative;
    text-align: left;
    width: 100%;
    background: none;
  }
  .block-row:hover {
    background: var(--bg-elevated);
  }
  .block-row.active {
    border-color: rgba(184, 255, 60, 0.3);
    background: rgba(184, 255, 60, 0.04);
  }
  .block-row.edited {
    border-left: 2px solid var(--info);
  }
  .block-row.dragging {
    opacity: 0.35;
    border-style: dashed;
  }
  .block-row.drag-over {
    border-color: var(--accent);
    background: rgba(184, 255, 60, 0.08);
  }
  .block-timecodes {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 10px;
    color: var(--text-muted);
  }
  .edited-badge {
    margin-left: auto;
    font-size: 9px;
    color: var(--info);
  }
  .block-text {
    font-size: 13px;
    color: var(--text-primary);
    line-height: 1.5;
    padding: 2px 0;
    min-height: 20px;
  }
  .block-edit-input {
    width: 100%;
    resize: vertical;
    background: var(--bg-overlay);
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 13px;
    padding: 4px 6px;
    outline: none;
  }
  .word-token {
    transition: color 0.1s;
  }
  .word-active {
    color: var(--accent);
    font-weight: 600;
  }
  .word-past {
    color: var(--text-muted);
  }
  .block-actions {
    display: flex;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.1s;
  }
  .block-row:hover .block-actions {
    opacity: 1;
  }
  .block-btn {
    width: 22px;
    height: 22px;
    border: 1px solid var(--border);
    background: var(--bg-overlay);
    color: var(--text-secondary);
    border-radius: 3px;
    cursor: pointer;
    font-size: 11px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .block-btn:hover {
    border-color: var(--border-strong);
    color: var(--text-primary);
  }
  .preview-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }
  .preview-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 10px;
    background: var(--bg-elevated);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .preview-bar-label {
    font-size: 9px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-muted);
  }
  .preview-toggle {
    background: none;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: 10px;
    padding: 2px 8px;
    cursor: pointer;
    font-family: var(--font-mono);
  }
  .preview-toggle:hover {
    border-color: var(--border-strong);
    color: var(--text-primary);
    background: var(--bg-overlay);
  }
  .video-preview-wrap {
    position: relative;
    flex-shrink: 0;
    background: #000;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    height: clamp(140px, 25vh, 240px);
    transition: height 0.25s ease;
  }
  .video-preview-wrap.collapsed {
    height: 0;
  }
  .subtitle-canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    z-index: 1;
  }
  .style-panel-wrap {
    flex: 1;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
  }
  /* ── Modale de confirmation ────────────────────────── */
  .confirm-overlay {
    position: absolute;
    inset: 0;
    z-index: 100;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    backdrop-filter: blur(2px);
  }
  .confirm-box {
    background: var(--bg-surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    padding: 24px 28px;
    max-width: 340px;
    width: 90%;
    display: flex;
    flex-direction: column;
    gap: 10px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }
  .confirm-icon {
    font-size: 24px;
    text-align: center;
    color: var(--danger);
  }
  .confirm-title {
    font-size: 15px;
    font-weight: 700;
    color: var(--text-primary);
    text-align: center;
  }
  .confirm-body {
    font-size: 12px;
    color: var(--text-secondary);
    text-align: center;
    line-height: 1.6;
  }
  .confirm-actions {
    display: flex;
    gap: 8px;
    justify-content: center;
    margin-top: 6px;
  }
</style>
