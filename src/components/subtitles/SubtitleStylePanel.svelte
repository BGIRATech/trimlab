<!--
  FICHIER : src/components/subtitles/SubtitleStylePanel.svelte
  ROLE    : Panneau de personnalisation des sous-titres — UI refaite
-->
<script lang="ts">
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

  export let style: SubtitleStyle;

  const fonts = [
    { value: "Arial", label: "Arial" },
    { value: "Impact", label: "Impact" },
    { value: "Georgia", label: "Georgia" },
    { value: "Verdana", label: "Verdana" },
    { value: "Trebuchet MS", label: "Trebuchet MS" },
    { value: "Inter", label: "Inter" },
    { value: "Syne", label: "Syne" },
    { value: "DM Mono", label: "DM Mono" },
  ];

  // ── Presets ──────────────────────────────────────────────
  function applyPreset(preset: Partial<SubtitleStyle>) {
    Object.assign(style, preset);
    style = style;
  }

  const presets: Array<{
    label: string;
    icon: string;
    s: Partial<SubtitleStyle>;
  }> = [
    {
      label: "YouTube",
      icon: "▶",
      s: {
        fontFamily: "Arial",
        fontSize: 36,
        color: "#FFFFFF",
        outlineColor: "#000000",
        outlineWidth: 2.5,
        bgColor: "#000000",
        bgOpacity: 0,
        bold: true,
        italic: false,
        animation: "none",
        position: "bottom",
        karaokeEnabled: false,
        maxWordsPerBlock: 5,
        maxCharsPerLine: 40,
      },
    },
    {
      label: "Cinéma",
      icon: "🎬",
      s: {
        fontFamily: "Georgia",
        fontSize: 30,
        color: "#FFFFFF",
        outlineColor: "#000000",
        outlineWidth: 1,
        bgColor: "#000000",
        bgOpacity: 0.6,
        bold: false,
        italic: false,
        animation: "fade",
        position: "bottom",
        karaokeEnabled: false,
        maxWordsPerBlock: 7,
        maxCharsPerLine: 50,
      },
    },
    {
      label: "TikTok",
      icon: "✦",
      s: {
        fontFamily: "Impact",
        fontSize: 46,
        color: "#FFFF00",
        outlineColor: "#000000",
        outlineWidth: 3,
        bgColor: "#000000",
        bgOpacity: 0,
        bold: false,
        italic: false,
        animation: "pop",
        position: "bottom",
        karaokeEnabled: false,
        maxWordsPerBlock: 3,
        maxCharsPerLine: 20,
      },
    },
    {
      label: "Minimal",
      icon: "○",
      s: {
        fontFamily: "Inter",
        fontSize: 28,
        color: "#FFFFFF",
        outlineColor: "#000000",
        outlineWidth: 0,
        bgColor: "#000000",
        bgOpacity: 0.45,
        bold: false,
        italic: false,
        animation: "fade",
        position: "bottom",
        karaokeEnabled: false,
        maxWordsPerBlock: 5,
        maxCharsPerLine: 36,
      },
    },
    {
      label: "Karaoke",
      icon: "🎤",
      s: {
        fontFamily: "Inter",
        fontSize: 34,
        color: "#FFFFFF",
        outlineColor: "#000000",
        outlineWidth: 2,
        bgColor: "#000000",
        bgOpacity: 0.45,
        bold: true,
        italic: false,
        animation: "fade",
        position: "bottom",
        karaokeEnabled: true,
        karaokeHighlight: "#B8FF3C",
        maxWordsPerBlock: 4,
        maxCharsPerLine: 28,
      },
    },
  ];

  function setPosition(id: string) {
    style.position = id as any;
    style = style;
  }
  function setAnimation(id: string) {
    style.animation = id as any;
    style = style;
  }

  // Helper pour convertir hex + opacity en rgba CSS pour le preview
  function hexToRgba(hex: string, opacity: number): string {
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);
    return `rgba(${r},${g},${b},${opacity})`;
  }
</script>

<div class="panel">
  <!-- ═══ PRESETS ═══════════════════════════════════════════ -->
  <div class="block">
    <div class="block-title">Presets rapides</div>
    <div class="presets">
      {#each presets as p}
        <button class="preset" on:click={() => applyPreset(p.s)}>
          <span class="preset-icon">{p.icon}</span>
          <span>{p.label}</span>
        </button>
      {/each}
    </div>
  </div>

  <!-- ═══ TEXTE ═══════════════════════════════════════════════ -->
  <div class="block">
    <div class="block-title">Texte</div>

    <!-- Police + taille -->
    <div class="row gap8">
      <div class="field" style="flex:1">
        <label class="lbl" for="font-sel">Police</label>
        <select id="font-sel" class="sel" bind:value={style.fontFamily}>
          {#each fonts as f}<option value={f.value}>{f.label}</option>{/each}
        </select>
      </div>
      <div class="field" style="width:88px">
        <span class="lbl">Taille</span>
        <div class="stepper">
          <button
            class="step"
            on:click={() => {
              style.fontSize = Math.max(12, style.fontSize - 2);
              style = style;
            }}>−</button
          >
          <span class="step-num">{style.fontSize}</span>
          <button
            class="step"
            on:click={() => {
              style.fontSize = Math.min(96, style.fontSize + 2);
              style = style;
            }}>+</button
          >
        </div>
      </div>
    </div>

    <!-- Gras · Italique + couleurs -->
    <div class="row gap8" style="margin-top:6px; align-items:flex-end">
      <!-- Gras / italique -->
      <div class="fmt-group">
        <button
          class="fmt"
          class:on={style.bold}
          on:click={() => {
            style.bold = !style.bold;
            style = style;
          }}><b>G</b></button
        >
        <button
          class="fmt"
          class:on={style.italic}
          on:click={() => {
            style.italic = !style.italic;
            style = style;
          }}><i>I</i></button
        >
      </div>

      <!-- Couleur texte -->
      <label class="swatch-wrap" title="Couleur du texte">
        <span class="swatch" style="background:{style.color}"></span>
        <span class="swatch-lbl">Texte</span>
        <input type="color" class="hidden-pick" bind:value={style.color} />
      </label>

      <!-- Couleur contour + épaisseur -->
      <label class="swatch-wrap" title="Couleur du contour">
        <span
          class="swatch outline-swatch"
          style="background:{style.outlineColor}"
        ></span>
        <span class="swatch-lbl">Contour</span>
        <input
          type="color"
          class="hidden-pick"
          bind:value={style.outlineColor}
        />
      </label>

      <div class="field" style="flex:1">
        <label class="lbl" for="outline-w"
          >Épaisseur <span class="chip">{style.outlineWidth.toFixed(1)}</span
          ></label
        >
        <input
          id="outline-w"
          type="range"
          min="0"
          max="6"
          step="0.5"
          bind:value={style.outlineWidth}
          class="slider"
        />
      </div>
    </div>
  </div>

  <!-- ═══ FOND ════════════════════════════════════════════════ -->
  <div class="block">
    <div class="block-title">Fond / Boîte</div>

    <div class="row gap8" style="align-items:flex-end">
      <label class="swatch-wrap" title="Couleur du fond">
        <span
          class="swatch lg"
          style="background:{style.bgColor}; opacity:{style.bgOpacity > 0
            ? 1
            : 0.3}"
        ></span>
        <span class="swatch-lbl">Couleur</span>
        <input type="color" class="hidden-pick" bind:value={style.bgColor} />
      </label>
      <div class="field" style="flex:1">
        <label class="lbl" for="bg-opacity2"
          >Opacité <span class="chip">{Math.round(style.bgOpacity * 100)}%</span
          ></label
        >
        <input
          id="bg-opacity2"
          type="range"
          min="0"
          max="1"
          step="0.05"
          bind:value={style.bgOpacity}
          class="slider"
        />
      </div>
    </div>

    <!-- Preview fond -->
    <div
      class="bg-preview"
      style="background:{hexToRgba(style.bgColor, style.bgOpacity)};
             color:{style.color};
             font-family:{style.fontFamily};
             font-weight:{style.bold ? 700 : 400};
             font-style:{style.italic ? 'italic' : 'normal'};
             -webkit-text-stroke:{style.outlineWidth > 0
        ? style.outlineWidth * 0.4 + 'px ' + style.outlineColor
        : 'none'}"
    >
      Aperçu du texte ici
    </div>
  </div>

  <!-- ═══ POSITION · ANIMATION ════════════════════════════════ -->
  <div class="block">
    <div class="block-title">Position & Animation</div>
    <div class="row gap6">
      <!-- Position -->
      <div class="seg-ctrl" style="flex:1">
        {#each [["top", "⬆", "Haut"], ["middle", "⬤", "Milieu"], ["bottom", "⬇", "Bas"]] as [id, ico, lbl]}
          <button
            class="seg"
            class:on={style.position === id}
            on:click={() => setPosition(id)}
          >
            <span style="font-size:11px">{ico}</span>
            <span>{lbl}</span>
          </button>
        {/each}
      </div>
      <!-- Animation -->
      <div class="seg-ctrl" style="flex:1">
        {#each [["none", "—", "Fixe"], ["fade", "◑", "Fondu"], ["pop", "✦", "Pop"]] as [id, ico, lbl]}
          <button
            class="seg"
            class:on={style.animation === id}
            on:click={() => setAnimation(id)}
          >
            <span style="font-size:11px">{ico}</span>
            <span>{lbl}</span>
          </button>
        {/each}
      </div>
    </div>
  </div>

  <!-- ═══ KARAOKÉ ══════════════════════════════════════════════ -->
  <div class="block">
    <div class="block-title">Karaoké</div>
    <button
      class="toggle-btn"
      class:on={style.karaokeEnabled}
      on:click={() => {
        style.karaokeEnabled = !style.karaokeEnabled;
        style = style;
      }}
    >
      <span class="tog-track"><span class="tog-thumb"></span></span>
      <span
        >{style.karaokeEnabled
          ? "Activé — mot actif mis en valeur"
          : "Désactivé"}</span
      >
    </button>
    {#if style.karaokeEnabled}
      <div class="row gap8" style="margin-top:8px; align-items:center">
        <label class="swatch-wrap">
          <span class="swatch lg" style="background:{style.karaokeHighlight}"
          ></span>
          <span class="swatch-lbl">Surlignage</span>
          <input
            type="color"
            class="hidden-pick"
            bind:value={style.karaokeHighlight}
          />
        </label>
        <div class="karaoke-demo">
          <span style="color:var(--text-muted)">Bonjour</span>
          <span style="color:{style.karaokeHighlight};font-weight:700">
            tout</span
          >
          <span style="color:var(--text-secondary)"> le monde</span>
        </div>
      </div>
    {/if}
  </div>

  <!-- ═══ DÉCOUPAGE ════════════════════════════════════════════ -->
  <div class="block" style="border-bottom:none">
    <div class="block-title">Découpage des blocs</div>
    <div class="field">
      <label class="lbl" for="max-words">
        Mots par bloc
        <span class="chip">{style.maxWordsPerBlock} mots</span>
      </label>
      <input
        id="max-words"
        type="range"
        min="2"
        max="8"
        step="1"
        bind:value={style.maxWordsPerBlock}
        class="slider"
      />
      <div class="tick-row">
        {#each [2, 3, 4, 5, 6, 7, 8] as v}
          <span class="tick" class:active={style.maxWordsPerBlock === v}
            >{v}</span
          >
        {/each}
      </div>
    </div>
    <div class="field" style="margin-top:8px">
      <label class="lbl" for="max-chars">
        Largeur max
        <span class="chip">{style.maxCharsPerLine} car.</span>
      </label>
      <input
        id="max-chars"
        type="range"
        min="16"
        max="60"
        step="2"
        bind:value={style.maxCharsPerLine}
        class="slider"
      />
    </div>
  </div>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    background: var(--bg-surface);
    border-top: 1px solid var(--border);
  }

  /* ── Blocs ──────────────────────────────────────── */
  .block {
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
  }
  .block-title {
    font-size: 9px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-muted);
    margin-bottom: 8px;
  }

  /* ── Layout ─────────────────────────────────────── */
  .row {
    display: flex;
  }
  .gap8 {
    gap: 8px;
  }
  .gap6 {
    gap: 6px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  /* ── Labels ─────────────────────────────────────── */
  .lbl {
    font-size: 10px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .chip {
    font-family: var(--font-mono);
    font-size: 9px;
    color: var(--accent);
    background: var(--accent-subtle);
    padding: 1px 5px;
    border-radius: 3px;
  }

  /* ── Select ─────────────────────────────────────── */
  .sel {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 12px;
    padding: 5px 7px;
    width: 100%;
    cursor: pointer;
    outline: none;
  }
  .sel:focus {
    border-color: var(--border-active);
  }

  /* ── Stepper taille ─────────────────────────────── */
  .stepper {
    display: flex;
    height: 29px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .step {
    width: 28px;
    flex-shrink: 0;
    border: none;
    background: none;
    color: var(--text-secondary);
    font-size: 17px;
    cursor: pointer;
    transition: background 0.1s;
    line-height: 1;
  }
  .step:hover {
    background: var(--bg-overlay);
    color: var(--text-primary);
  }
  .step-num {
    flex: 1;
    text-align: center;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-primary);
    border-left: 1px solid var(--border);
    border-right: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  /* ── Slider ─────────────────────────────────────── */
  .slider {
    width: 100%;
    accent-color: var(--accent);
    cursor: pointer;
    height: 3px;
  }

  /* ── Gras / Italique ────────────────────────────── */
  .fmt-group {
    display: flex;
    gap: 3px;
    align-items: flex-end;
  }
  .fmt {
    width: 30px;
    height: 29px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    font-size: 13px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.1s;
  }
  .fmt:hover {
    border-color: var(--border-strong);
    color: var(--text-primary);
  }
  .fmt.on {
    background: var(--accent-subtle);
    border-color: rgba(184, 255, 60, 0.4);
    color: var(--accent);
  }

  /* ── Swatches couleur ───────────────────────────── */
  .swatch-wrap {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    cursor: pointer;
    position: relative;
    flex-shrink: 0;
    isolation: isolate;
  }
  .swatch {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    border: 1.5px solid var(--border-strong);
    transition:
      transform 0.12s,
      border-color 0.12s;
    display: block;
  }
  .swatch.lg {
    width: 32px;
    height: 32px;
  }
  .outline-swatch {
    box-shadow: inset 0 0 0 2px var(--bg-elevated);
  }
  .swatch-wrap:hover .swatch {
    transform: scale(1.1);
    border-color: var(--accent);
  }
  .swatch-lbl {
    font-size: 9px;
    color: var(--text-muted);
    white-space: nowrap;
  }
  .hidden-pick {
    position: absolute;
    inset: 0;
    opacity: 0;
    width: 100%;
    height: 100%;
    cursor: pointer;
    border: none;
    padding: 0;
  }

  /* ── Preview fond ───────────────────────────────── */
  .bg-preview {
    margin-top: 8px;
    padding: 7px 14px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    text-align: center;
    font-size: 13px;
    transition: all 0.2s;
  }

  /* ── Segmented controls ─────────────────────────── */
  .seg-ctrl {
    display: flex;
    gap: 2px;
    padding: 2px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .seg {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    padding: 5px 3px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-muted);
    font-size: 9px;
    cursor: pointer;
    transition: all 0.1s;
  }
  .seg:hover {
    background: var(--bg-overlay);
    color: var(--text-secondary);
  }
  .seg.on {
    background: var(--accent);
    color: #000;
    font-weight: 700;
  }

  /* ── Toggle karaoké ─────────────────────────────── */
  .toggle-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 7px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
    transition: all 0.15s;
    text-align: left;
  }
  .toggle-btn.on {
    border-color: rgba(184, 255, 60, 0.35);
    background: var(--accent-subtle);
    color: var(--accent);
  }
  .tog-track {
    width: 28px;
    height: 16px;
    border-radius: 8px;
    background: var(--bg-overlay);
    border: 1px solid var(--border-strong);
    position: relative;
    flex-shrink: 0;
    transition: background 0.2s;
  }
  .tog-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--text-muted);
    transition:
      transform 0.2s,
      background 0.2s;
  }
  .toggle-btn.on .tog-track {
    background: rgba(184, 255, 60, 0.2);
    border-color: rgba(184, 255, 60, 0.4);
  }
  .toggle-btn.on .tog-thumb {
    transform: translateX(12px);
    background: var(--accent);
  }
  .karaoke-demo {
    flex: 1;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 5px 10px;
    font-size: 13px;
    font-weight: 500;
  }

  /* ── Presets ────────────────────────────────────── */
  .presets {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 5px;
  }
  .preset {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    padding: 7px 4px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    cursor: pointer;
    transition: all 0.12s;
  }
  .preset:hover {
    border-color: var(--border-active);
    background: var(--accent-subtle);
    color: var(--accent);
  }
  .preset-icon {
    font-size: 15px;
  }

  /* ── Ticks découpage ────────────────────────────── */
  .tick-row {
    display: flex;
    justify-content: space-between;
    padding: 0 1px;
  }
  .tick {
    font-size: 9px;
    font-family: var(--font-mono);
    color: var(--text-muted);
  }
  .tick.active {
    color: var(--accent);
    font-weight: 700;
  }
</style>
