<!--
  FICHIER : src/components/ParamsPanel.svelte
  ROLE    : Panneau paramètres analyse silence
  ARCH    : ParamsPanel NE fait PAS l'analyse lui-même.
            Il dispatch un event "analyse" avec les paramètres → routes/App.svelte
            appelle analyseProject (seul chemin d'analyse, undo/redo intact).
-->
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { activeProject, isAnalysing } from "../lib/store";

  const dispatch = createEventDispatcher<{
    analyse: {
      thresholdDb: number;
      minDurationMs: number;
      paddingBefore: number;
      paddingAfter: number;
      minSpeechMs: number;
      aggressiveness: number;
    };
    fillerWordsChange: string[];
  }>();

  // ── Paramètres ───────────────────────────────────────────
  let thresholdMode: "auto" | "manual" = "auto";
  let thresholdDb = -35;
  let minDurationMs = 300;
  let paddingBefore = 0.05;
  let paddingAfter = 0.12;
  let minSpeechMs = 200;
  let aggressiveness = 3;

  // ── Sections collapsibles ────────────────────────────────
  let openSeuil = false;
  let openDurees = false;
  let openPadding = false;

  // ── Presets ──────────────────────────────────────────────
  const presets = [
    {
      id: "podcast",
      icon: "🎙",
      label: "Podcast",
      p: {
        thresholdMode: "auto" as const,
        minDurationMs: 300,
        paddingBefore: 0.05,
        paddingAfter: 0.12,
        minSpeechMs: 200,
        aggressiveness: 3,
      },
    },
    {
      id: "vlog",
      icon: "📱",
      label: "Vlog",
      p: {
        thresholdMode: "auto" as const,
        minDurationMs: 200,
        paddingBefore: 0.03,
        paddingAfter: 0.08,
        minSpeechMs: 150,
        aggressiveness: 3,
      },
    },
    {
      id: "interview",
      icon: "🎤",
      label: "Interview",
      p: {
        thresholdMode: "auto" as const,
        minDurationMs: 450,
        paddingBefore: 0.08,
        paddingAfter: 0.18,
        minSpeechMs: 300,
        aggressiveness: 2,
      },
    },
    {
      id: "tutoriel",
      icon: "💻",
      label: "Tutoriel",
      p: {
        thresholdMode: "auto" as const,
        minDurationMs: 220,
        paddingBefore: 0.03,
        paddingAfter: 0.08,
        minSpeechMs: 150,
        aggressiveness: 4,
      },
    },
    {
      id: "cinema",
      icon: "🎬",
      label: "Cinéma",
      p: {
        thresholdMode: "manual" as const,
        minDurationMs: 600,
        paddingBefore: 0.12,
        paddingAfter: 0.25,
        minSpeechMs: 500,
        aggressiveness: 1,
      },
    },
  ];

  let activePreset: string | null = "podcast";

  function applyPreset(preset: (typeof presets)[0]) {
    activePreset = preset.id;
    thresholdMode = preset.p.thresholdMode;
    minDurationMs = preset.p.minDurationMs;
    paddingBefore = preset.p.paddingBefore;
    paddingAfter = preset.p.paddingAfter;
    minSpeechMs = preset.p.minSpeechMs;
    aggressiveness = preset.p.aggressiveness;
  }

  function onManualChange() {
    activePreset = null;
  }

  // ── Aggressivité ─────────────────────────────────────────
  const aggrColors = [
    "",
    "#4bff7a",
    "#8bff4b",
    "#f0c040",
    "#ff9040",
    "#ff4b4b",
  ];
  const aggrLabels = [
    "",
    "Très conservateur",
    "Conservateur",
    "Équilibré",
    "Agressif",
    "Très agressif",
  ];

  // ── Mots fillers ─────────────────────────────────────────
  // Listes par défaut selon la langue du projet
  const DEFAULT_FILLERS: Record<string, string[]> = {
    fr: [
      "euh",
      "hm",
      "ben",
      "bah",
      "voilà",
      "quoi",
      "genre",
      "donc",
      "enfin",
      "du coup",
      "en fait",
    ],
    en: [
      "uh",
      "um",
      "like",
      "you know",
      "so",
      "basically",
      "actually",
      "right",
      "i mean",
    ],
    es: ["eh", "bueno", "o sea", "pues", "este", "verdad"],
    de: ["äh", "ähm", "also", "halt", "irgendwie", "eigentlich"],
  };

  // Utiliser la langue du projet ou français par défaut
  $: projectLang = ($activeProject?.settings?.language ?? "fr")
    .slice(0, 2)
    .toLowerCase();
  $: defaultFillers = DEFAULT_FILLERS[projectLang] ?? DEFAULT_FILLERS["fr"];

  let fillerWords: string[] = $activeProject?.settings?.filler_words?.length
    ? [...$activeProject.settings.filler_words]
    : [
        ...(DEFAULT_FILLERS[
          $activeProject?.settings?.language?.slice(0, 2)?.toLowerCase() ?? "fr"
        ] ?? DEFAULT_FILLERS["fr"]),
      ];

  let fillerInput = "";
  let openFillers = false;

  function addFiller() {
    const w = fillerInput.trim().toLowerCase();
    if (w && !fillerWords.includes(w)) {
      fillerWords = [...fillerWords, w];
      dispatch("fillerWordsChange", fillerWords);
    }
    fillerInput = "";
  }

  function removeFiller(w: string) {
    fillerWords = fillerWords.filter((f) => f !== w);
    dispatch("fillerWordsChange", fillerWords);
  }

  function resetFillers() {
    fillerWords = [...defaultFillers];
    dispatch("fillerWordsChange", fillerWords);
  }

  // Resync si le projet actif change
  let _lastProjectId: string | null = null;
  $: if ($activeProject && $activeProject.id !== _lastProjectId) {
    _lastProjectId = $activeProject.id;
    fillerWords = $activeProject.settings?.filler_words?.length
      ? [...$activeProject.settings.filler_words]
      : [...defaultFillers];
  }

  // ── Lancer l'analyse via event → routes/App.svelte ───────
  function runAnalyse() {
    dispatch("analyse", {
      thresholdDb: thresholdMode === "auto" ? 0 : thresholdDb,
      minDurationMs,
      paddingBefore,
      paddingAfter,
      minSpeechMs,
      aggressiveness,
    });
  }
</script>

<div class="panel">
  <!-- ══ ZONE SCROLLABLE ══════════════════════════════════════ -->
  <div class="scroll-area">
    <!-- PRESETS ────────────────────────────────────────────── -->
    <div class="section">
      <div class="section-label">Preset</div>
      <div class="preset-grid">
        {#each presets as p}
          <button
            class="preset-btn"
            class:active={activePreset === p.id}
            on:click={() => applyPreset(p)}
            title={p.id}
          >
            <span class="preset-icon">{p.icon}</span>
            <span class="preset-label">{p.label}</span>
          </button>
        {/each}
      </div>
    </div>

    <!-- AGGRESSIVITÉ ───────────────────────────────────────── -->
    <div class="section">
      <div class="section-label">
        Aggressivité
        <span
          style="color:{aggrColors[
            aggressiveness
          ]}; font-weight:600; font-size:9px"
        >
          {aggrLabels[aggressiveness]}
        </span>
      </div>
      <div class="aggr-row">
        {#each [1, 2, 3, 4, 5] as lvl}
          <button
            class="aggr-btn"
            class:on={aggressiveness === lvl}
            style={aggressiveness === lvl
              ? `background:${aggrColors[lvl]}18;border-color:${aggrColors[lvl]}55;color:${aggrColors[lvl]}`
              : ""}
            on:click={() => {
              aggressiveness = lvl;
              onManualChange();
            }}>{lvl}</button
          >
        {/each}
      </div>
    </div>

    <!-- SEUIL ──────────────────────────────────────────────── -->
    <div class="section collapsible">
      <button
        class="collapsible-header"
        on:click={() => (openSeuil = !openSeuil)}
      >
        <span class="section-label" style="margin:0">Seuil</span>
        <span class="badge"
          >{thresholdMode === "auto" ? "Auto" : thresholdDb + " dB"}</span
        >
        <span class="chevron" class:open={openSeuil}>›</span>
      </button>
      {#if openSeuil}
        <div class="collapsible-body">
          <div class="toggle-row">
            <button
              class="tog"
              class:on={thresholdMode === "auto"}
              on:click={() => {
                thresholdMode = "auto";
                onManualChange();
              }}>🤖 Auto</button
            >
            <button
              class="tog"
              class:on={thresholdMode === "manual"}
              on:click={() => {
                thresholdMode = "manual";
                onManualChange();
              }}>✎ Manuel</button
            >
          </div>
          {#if thresholdMode === "manual"}
            <div class="slider-row">
              <div class="slider-labels">
                <span>Seuil</span><span class="val">{thresholdDb} dB</span>
              </div>
              <input
                type="range"
                min="-60"
                max="-20"
                step="1"
                bind:value={thresholdDb}
                on:input={onManualChange}
              />
              <div class="scale-labels">
                <span>-60 dB</span><span>-20 dB</span>
              </div>
            </div>
          {:else}
            <p class="hint">
              FFmpeg mesure le volume moyen et calcule le seuil optimal.
            </p>
          {/if}
        </div>
      {/if}
    </div>

    <!-- DURÉES ─────────────────────────────────────────────── -->
    <div class="section collapsible">
      <button
        class="collapsible-header"
        on:click={() => (openDurees = !openDurees)}
      >
        <span class="section-label" style="margin:0">Durées</span>
        <span class="badge">{minDurationMs}ms · {minSpeechMs}ms</span>
        <span class="chevron" class:open={openDurees}>›</span>
      </button>
      {#if openDurees}
        <div class="collapsible-body">
          <div class="slider-row">
            <div class="slider-labels">
              <span>Silence min à couper</span><span class="val"
                >{minDurationMs} ms</span
              >
            </div>
            <input
              type="range"
              min="100"
              max="2000"
              step="50"
              bind:value={minDurationMs}
              on:input={onManualChange}
            />
            <div class="scale-labels"><span>100ms</span><span>2s</span></div>
          </div>
          <div class="slider-row" style="margin-top:8px">
            <div class="slider-labels">
              <span>Parole min conservée</span><span class="val"
                >{minSpeechMs} ms</span
              >
            </div>
            <input
              type="range"
              min="50"
              max="1000"
              step="25"
              bind:value={minSpeechMs}
              on:input={onManualChange}
            />
            <div class="scale-labels"><span>50ms</span><span>1s</span></div>
          </div>
        </div>
      {/if}
    </div>

    <!-- PADDING ────────────────────────────────────────────── -->
    <div class="section collapsible">
      <button
        class="collapsible-header"
        on:click={() => (openPadding = !openPadding)}
      >
        <span class="section-label" style="margin:0">Marges (Padding)</span>
        <span class="badge"
          >↤{(paddingBefore * 1000).toFixed(0)}ms · {(
            paddingAfter * 1000
          ).toFixed(0)}ms↦</span
        >
        <span class="chevron" class:open={openPadding}>›</span>
      </button>
      {#if openPadding}
        <div class="collapsible-body">
          <div class="padding-row">
            <div class="pad-item">
              <span class="pad-label">⬅ Avant</span>
              <div class="stepper">
                <button
                  class="step"
                  on:click={() => {
                    paddingBefore = Math.max(
                      0,
                      +(paddingBefore - 0.01).toFixed(2),
                    );
                    onManualChange();
                  }}>−</button
                >
                <span class="step-val"
                  >{(paddingBefore * 1000).toFixed(0)} ms</span
                >
                <button
                  class="step"
                  on:click={() => {
                    paddingBefore = Math.min(
                      0.5,
                      +(paddingBefore + 0.01).toFixed(2),
                    );
                    onManualChange();
                  }}>+</button
                >
              </div>
            </div>
            <div class="pad-item">
              <span class="pad-label">Après ➡</span>
              <div class="stepper">
                <button
                  class="step"
                  on:click={() => {
                    paddingAfter = Math.max(
                      0,
                      +(paddingAfter - 0.01).toFixed(2),
                    );
                    onManualChange();
                  }}>−</button
                >
                <span class="step-val"
                  >{(paddingAfter * 1000).toFixed(0)} ms</span
                >
                <button
                  class="step"
                  on:click={() => {
                    paddingAfter = Math.min(
                      0.5,
                      +(paddingAfter + 0.01).toFixed(2),
                    );
                    onManualChange();
                  }}>+</button
                >
              </div>
            </div>
          </div>
          <div class="padding-viz">
            <div class="viz-keep">Parole</div>
            <div
              class="viz-pad-after"
              style="width:{paddingAfter * 180}px"
            ></div>
            <div class="viz-cut">✂</div>
            <div
              class="viz-pad-before"
              style="width:{paddingBefore * 180}px"
            ></div>
            <div class="viz-keep">Parole</div>
          </div>
        </div>
      {/if}
    </div>
    <!-- FILLERS ─────────────────────────────────────────────── -->
    <div class="section collapsible">
      <button
        class="collapsible-header"
        on:click={() => (openFillers = !openFillers)}
      >
        <span class="chevron" class:open={openFillers}>›</span>
        <span class="section-label" style="margin:0">Mots fillers</span>
        {#if fillerWords.length > 0}
          <span class="badge"
            >{fillerWords.length} mot{fillerWords.length > 1 ? "s" : ""}</span
          >
        {/if}
      </button>
      {#if openFillers}
        <div class="collapsible-body">
          <p class="filler-hint">
            Ces mots seront marqués comme fillers lors de la transcription
            Whisper.
          </p>
          <div class="filler-chips">
            {#each fillerWords as w}
              <span class="filler-chip">
                {w}
                <button
                  class="chip-remove"
                  on:click={() => removeFiller(w)}
                  title="Retirer">×</button
                >
              </span>
            {/each}
          </div>
          <div class="filler-add-row">
            <input
              class="filler-input"
              type="text"
              placeholder="Ajouter un mot…"
              bind:value={fillerInput}
              on:keydown={(e) => e.key === "Enter" && addFiller()}
            />
            <button
              class="btn-add-filler"
              on:click={addFiller}
              disabled={!fillerInput.trim()}>+</button
            >
          </div>
          <button class="btn-reset-fillers" on:click={resetFillers}>
            ↺ Défauts ({projectLang.toUpperCase()})
          </button>
        </div>
      {/if}
    </div>
  </div>
  <!-- /scroll-area -->

  <!-- ══ BOUTON ANALYSER FIXE EN BAS — jamais masqué ══════════ -->
  <div class="action-bar">
    <button
      class="btn-analyse"
      class:loading={$isAnalysing}
      disabled={$isAnalysing ||
        !$activeProject ||
        $activeProject.files.length === 0}
      on:click={runAnalyse}
    >
      {#if $isAnalysing}
        <span class="spinner"></span> Analyse en cours…
      {:else}
        ⚡ Analyser les silences
      {/if}
    </button>
  </div>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .scroll-area {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
  }
  .scroll-area::-webkit-scrollbar {
    width: 4px;
  }
  .scroll-area::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 2px;
  }

  .section {
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
  }
  .section-label {
    font-size: 9px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-muted);
    margin-bottom: 7px;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .collapsible {
    padding: 0;
  }
  .collapsible-header {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 10px 12px;
    background: none;
    border: none;
    cursor: pointer;
    color: inherit;
    text-align: left;
  }
  .collapsible-header:hover {
    background: var(--bg-elevated);
  }
  .badge {
    margin-left: auto;
    font-size: 9px;
    font-family: var(--font-mono);
    color: var(--accent);
    background: var(--accent-subtle);
    padding: 1px 5px;
    border-radius: 3px;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .chevron {
    font-size: 14px;
    color: var(--text-muted);
    transform: rotate(0deg);
    transition: transform 0.15s;
    flex-shrink: 0;
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .collapsible-body {
    padding: 0 12px 10px;
    border-top: 1px solid var(--border);
    background: var(--bg-elevated);
  }

  .preset-grid {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 4px;
  }
  .preset-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 6px 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.12s;
  }
  .preset-btn:hover {
    border-color: var(--border-strong);
    color: var(--text-primary);
  }
  .preset-btn.active {
    border-color: rgba(184, 255, 60, 0.5);
    background: var(--accent-subtle);
    color: var(--accent);
  }
  .preset-icon {
    font-size: 14px;
  }
  .preset-label {
    font-size: 8px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .aggr-row {
    display: flex;
    gap: 4px;
  }
  .aggr-btn {
    flex: 1;
    height: 28px;
    border: 1.5px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.12s;
  }
  .aggr-btn:hover {
    border-color: var(--border-strong);
    color: var(--text-primary);
  }

  .slider-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding-top: 8px;
  }
  .slider-labels {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--text-secondary);
  }
  .val {
    font-family: var(--font-mono);
    color: var(--accent);
    font-size: 11px;
  }
  .slider-row input[type="range"] {
    width: 100%;
    accent-color: var(--accent);
    height: 3px;
  }
  .scale-labels {
    display: flex;
    justify-content: space-between;
    font-size: 9px;
    color: var(--text-muted);
  }

  .toggle-row {
    display: flex;
    gap: 4px;
    padding-top: 8px;
  }
  .tog {
    flex: 1;
    padding: 5px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
    transition: all 0.1s;
  }
  .tog.on {
    border-color: rgba(184, 255, 60, 0.4);
    background: var(--accent-subtle);
    color: var(--accent);
  }

  .padding-row {
    display: flex;
    gap: 8px;
    padding-top: 8px;
  }
  .pad-item {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
    align-items: center;
  }
  .pad-label {
    font-size: 10px;
    color: var(--text-secondary);
  }
  .stepper {
    display: flex;
    align-items: center;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-surface);
    overflow: hidden;
    width: 100%;
  }
  .step {
    width: 26px;
    height: 26px;
    border: none;
    background: none;
    color: var(--text-secondary);
    font-size: 15px;
    cursor: pointer;
    flex-shrink: 0;
  }
  .step:hover {
    background: var(--bg-overlay);
    color: var(--text-primary);
  }
  .step-val {
    flex: 1;
    text-align: center;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-primary);
    border-left: 1px solid var(--border);
    border-right: 1px solid var(--border);
  }
  .padding-viz {
    display: flex;
    align-items: center;
    margin-top: 8px;
    height: 20px;
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .viz-keep {
    flex: 1;
    height: 100%;
    background: rgba(184, 255, 60, 0.12);
    border: 1px solid rgba(184, 255, 60, 0.25);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 8px;
    color: var(--accent);
    font-weight: 700;
  }
  .viz-pad-after,
  .viz-pad-before {
    height: 100%;
    background: rgba(184, 255, 60, 0.05);
    border-top: 1px solid rgba(184, 255, 60, 0.15);
    border-bottom: 1px solid rgba(184, 255, 60, 0.15);
    min-width: 3px;
    transition: width 0.2s;
  }
  .viz-cut {
    height: 100%;
    min-width: 24px;
    background: rgba(255, 75, 75, 0.1);
    border: 1px solid rgba(255, 75, 75, 0.2);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 9px;
  }

  .hint {
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.4;
    padding-top: 6px;
  }

  .action-bar {
    padding: 10px 12px;
    border-top: 1px solid var(--border);
    background: var(--bg-surface);
    flex-shrink: 0;
  }
  .btn-analyse {
    width: 100%;
    padding: 9px;
    background: var(--accent);
    color: #000;
    border: none;
    border-radius: var(--radius-sm);
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    transition: all 0.15s;
  }
  .btn-analyse:hover:not(:disabled) {
    filter: brightness(1.08);
  }
  .btn-analyse:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .btn-analyse.loading {
    background: var(--bg-elevated);
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }
  .spinner {
    width: 13px;
    height: 13px;
    border: 2px solid var(--border-strong);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    flex-shrink: 0;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  /* ── Fillers ─────────────────────────────────── */
  .filler-hint {
    font-size: 10px;
    color: var(--text-muted);
    margin: 0 0 8px;
    line-height: 1.5;
  }
  .filler-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    margin-bottom: 8px;
    min-height: 24px;
  }
  .filler-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: rgba(255, 184, 77, 0.12);
    border: 1px solid rgba(255, 184, 77, 0.35);
    color: #f0c040;
    border-radius: 12px;
    font-size: 11px;
    font-family: var(--font-mono);
    padding: 2px 7px 2px 8px;
  }
  .chip-remove {
    background: none;
    border: none;
    color: rgba(255, 184, 77, 0.6);
    cursor: pointer;
    font-size: 13px;
    padding: 0;
    line-height: 1;
    display: flex;
    align-items: center;
  }
  .chip-remove:hover {
    color: #ff4b4b;
  }
  .filler-add-row {
    display: flex;
    gap: 5px;
    margin-bottom: 6px;
  }
  .filler-input {
    flex: 1;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 12px;
    padding: 4px 8px;
    outline: none;
    font-family: var(--font-mono);
  }
  .filler-input:focus {
    border-color: rgba(255, 184, 77, 0.5);
  }
  .btn-add-filler {
    width: 26px;
    height: 26px;
    border: 1px solid var(--border);
    background: var(--bg-overlay);
    color: var(--text-secondary);
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .btn-add-filler:not(:disabled):hover {
    border-color: rgba(255, 184, 77, 0.5);
    color: #f0c040;
  }
  .btn-add-filler:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .btn-reset-fillers {
    background: none;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    font-size: 10px;
    padding: 3px 8px;
    cursor: pointer;
    font-family: var(--font-mono);
  }
  .btn-reset-fillers:hover {
    border-color: rgba(255, 184, 77, 0.4);
    color: #f0c040;
  }
</style>
