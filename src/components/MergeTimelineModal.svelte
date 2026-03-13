<!--
  FICHIER : trimlab/src/components/MergeTimelineModal.svelte
  ROLE    : Sélection de projets à fusionner en une seule timeline XML
-->
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { save } from "@tauri-apps/plugin-dialog";
  import { projects, notify } from "../lib/store";
  import { commands } from "../lib/commands";

  export let open = false;

  const dispatch = createEventDispatcher();

  let title = "Montage fusionné";
  let selected: string[] = []; // ids sélectionnés
  let order: string[] = []; // ids dans l'ordre de la timeline
  let exporting = false;

  $: if (open) {
    selected = $projects
      .filter((p) => p.status === "done" || p.status === "ready")
      .map((p) => p.id);
    order = [...selected];
  }

  function isSelected(id: string): boolean {
    return selected.includes(id);
  }

  function toggleProject(id: string) {
    if (selected.includes(id)) {
      selected = selected.filter((i) => i !== id);
      order = order.filter((i) => i !== id);
    } else {
      selected = [...selected, id];
      order = [...order, id];
    }
  }

  function moveUp(id: string) {
    const i = order.indexOf(id);
    if (i <= 0) return;
    const arr = [...order];
    const tmp = arr[i - 1];
    arr[i - 1] = arr[i];
    arr[i] = tmp;
    order = arr;
  }

  function moveDown(id: string) {
    const i = order.indexOf(id);
    if (i < 0 || i >= order.length - 1) return;
    const arr = [...order];
    const tmp = arr[i];
    arr[i] = arr[i + 1];
    arr[i + 1] = tmp;
    order = arr;
  }

  async function handleExport() {
    if (order.length < 2) {
      notify({
        type: "warning",
        title: "Sélection insuffisante",
        message: "Sélectionne au moins 2 projets.",
      });
      return;
    }

    const outputPath = await save({
      title: "Enregistrer la timeline fusionnée",
      defaultPath: `${title}.xml`,
      filters: [{ name: "FCP7 XML", extensions: ["xml"] }],
    });
    if (!outputPath) return;

    exporting = true;
    try {
      const result = await commands.exportMultiXml({
        projectIds: order,
        outputPath,
        title,
      });

      if (result.success) {
        notify({
          type: "success",
          title: "Timeline fusionnée",
          message: `${result.projectsMerged} projets · ${result.clipsExported} clips exportés`,
        });
        dispatch("close");
      } else {
        notify({
          type: "error",
          title: "Erreur export",
          message: result.error ?? "Inconnue",
        });
      }
    } catch (e: any) {
      notify({
        type: "error",
        title: "Erreur",
        message: e?.message ?? String(e),
      });
    } finally {
      exporting = false;
    }
  }

  function close() {
    if (!exporting) dispatch("close");
  }
</script>

{#if open}
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
  <div class="overlay" on:click|self={close}>
    <div class="modal">
      <div class="modal-header">
        <h2 class="font-display">Fusionner en timeline</h2>
        <button
          class="btn btn-ghost btn-sm"
          on:click={close}
          disabled={exporting}>✕</button
        >
      </div>

      <div class="modal-body">
        <!-- Titre du projet fusionné -->
        <div class="field">
          <label for="merge-title" class="field-label"
            >Titre de la séquence</label
          >
          <input
            id="merge-title"
            class="input"
            bind:value={title}
            placeholder="Montage fusionné"
          />
        </div>

        <!-- Liste des projets avec checkbox + ordre -->
        <div class="field">
          <div class="field-label">
            Projets à inclure <span class="text-muted"
              >(dans l'ordre de la timeline)</span
            >
          </div>
          <div class="project-list">
            {#each $projects as proj (proj.id)}
              <div
                class="proj-row"
                class:selected={isSelected(proj.id)}
                class:disabled={proj.status !== "done" &&
                  proj.status !== "ready"}
              >
                <input
                  type="checkbox"
                  checked={isSelected(proj.id)}
                  disabled={proj.status !== "done" && proj.status !== "ready"}
                  on:change={() => toggleProject(proj.id)}
                />
                <div class="proj-info">
                  <span class="proj-name">{proj.name}</span>
                  <span class="proj-status text-muted">
                    {proj.status === "done" || proj.status === "ready"
                      ? "✓ analysé"
                      : proj.status === "idle"
                        ? "non analysé"
                        : proj.status}
                  </span>
                </div>
                {#if isSelected(proj.id)}
                  <div class="order-badge">{order.indexOf(proj.id) + 1}</div>
                  <div class="reorder-btns">
                    <button
                      class="btn btn-ghost btn-xs"
                      on:click={() => moveUp(proj.id)}
                      disabled={order.indexOf(proj.id) === 0}>↑</button
                    >
                    <button
                      class="btn btn-ghost btn-xs"
                      on:click={() => moveDown(proj.id)}
                      disabled={order.indexOf(proj.id) === order.length - 1}
                      >↓</button
                    >
                  </div>
                {/if}
              </div>
            {/each}

            {#if $projects.length === 0}
              <p class="text-muted" style="padding: 16px; text-align:center;">
                Aucun projet disponible.
              </p>
            {/if}
          </div>
        </div>

        <!-- Résumé -->
        {#if order.length >= 2}
          <div class="summary">
            <span>🎬 {order.length} projets dans l'ordre :</span>
            <span class="text-muted">
              {order
                .map((id) => $projects.find((p) => p.id === id)?.name ?? id)
                .join(" → ")}
            </span>
          </div>
        {:else}
          <div class="summary warning">
            Sélectionne au moins 2 projets analysés pour fusionner.
          </div>
        {/if}
      </div>

      <div class="modal-footer">
        <button class="btn btn-ghost" on:click={close} disabled={exporting}
          >Annuler</button
        >
        <button
          class="btn btn-primary"
          on:click={handleExport}
          disabled={exporting || order.length < 2}
        >
          {#if exporting}
            <span class="spinner">⟳</span> Export en cours…
          {:else}
            Exporter la timeline XML
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(4px);
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    width: 540px;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.5);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 24px 0;
  }
  .modal-header h2 {
    font-size: 18px;
    font-weight: 700;
  }

  .modal-body {
    padding: 20px 24px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    overflow-y: auto;
    flex: 1;
  }

  .modal-footer {
    padding: 16px 24px;
    border-top: 1px solid var(--border);
    display: flex;
    gap: 10px;
    justify-content: flex-end;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .field-label {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }
  .input {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 8px 12px;
    color: var(--text-primary);
    font-size: 14px;
    width: 100%;
  }
  .input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .project-list {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    max-height: 280px;
    overflow-y: auto;
  }

  .proj-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
    transition: background 0.1s;
  }
  .proj-row:last-child {
    border-bottom: none;
  }
  .proj-row:hover {
    background: var(--bg-surface);
  }
  .proj-row.selected {
    background: var(--accent-subtle);
  }
  .proj-row.disabled {
    opacity: 0.45;
  }

  .proj-row input[type="checkbox"] {
    cursor: pointer;
    accent-color: var(--accent);
    flex-shrink: 0;
  }

  .proj-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .proj-name {
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .proj-status {
    font-size: 11px;
  }

  .order-badge {
    background: var(--accent);
    color: #000;
    border-radius: 50%;
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 700;
    flex-shrink: 0;
  }

  .reorder-btns {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .btn-xs {
    padding: 1px 5px;
    font-size: 11px;
    line-height: 1.4;
  }

  .summary {
    font-size: 12px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 10px 14px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    line-height: 1.5;
  }
  .summary.warning {
    border-color: rgba(255, 180, 50, 0.4);
    color: #ffb432;
  }

  .spinner {
    display: inline-block;
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
