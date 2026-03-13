<!--
  FICHIER : trimlab/src/routes/Dashboard.svelte
  ROLE    : Dashboard - stats réelles + bouton fusion multi-projets
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { currentView, dashboardStats, projects } from "../lib/store";
  import type { DashboardStats } from "../lib/store";
  import { commands } from "../lib/commands";
  import { formatDuration } from "../lib/utils";
  import MergeTimelineModal from "../components/MergeTimelineModal.svelte";

  let loading = true;
  let showMerge = false;

  onMount(async () => {
    try {
      // Charger les projets si le store est vide (navigation directe vers Dashboard)
      if ($projects.length === 0) {
        const projs = await commands.listProjects();
        projects.set(projs);
      }
      const stats = await commands.getDashboardStats();
      dashboardStats.set(stats);
    } catch (e) {
      console.error("Dashboard stats error:", e);
    } finally {
      loading = false;
    }
  });

  $: doneProjects = $projects.filter(
    (p) => p.status === "done" || p.status === "ready",
  );

  function formatNumber(n: number): string {
    return n.toLocaleString("fr-FR");
  }

  function formatPercent(n: number): string {
    return (n * 100).toFixed(1) + "%";
  }
</script>

<MergeTimelineModal open={showMerge} on:close={() => (showMerge = false)} />

<div class="dashboard">
  <div class="dash-header">
    <div>
      <h1 class="font-display">Dashboard</h1>
      <p class="text-muted" style="font-size:13px; margin-top:4px;">
        Statistiques de vos projets TrimLab
      </p>
    </div>
    <div class="header-actions">
      {#if doneProjects.length >= 2}
        <button
          class="btn btn-primary btn-sm"
          on:click={() => (showMerge = true)}
          title="Fusionner plusieurs projets en une seule timeline XML"
        >
          ⊕ Fusionner en timeline
        </button>
      {/if}
      <button
        class="btn btn-ghost btn-sm"
        on:click={() => currentView.set("app")}
      >
        Retour à l'éditeur
      </button>
    </div>
  </div>

  {#if loading}
    <div class="loading-state">
      <div class="spinner">o</div>
      <span class="text-muted">Chargement des statistiques...</span>
    </div>
  {:else if $dashboardStats}
    {@const s = $dashboardStats}
    <div class="stats-grid">
      <div class="stat-card">
        <div class="stat-card-icon projects">P</div>
        <div class="stat-card-body">
          <div class="stat-card-value font-display">
            {formatNumber(s.total_projects)}
          </div>
          <div class="stat-card-label">Projets créés</div>
        </div>
      </div>

      <div class="stat-card">
        <div class="stat-card-icon files">F</div>
        <div class="stat-card-body">
          <div class="stat-card-value font-display">
            {formatNumber(s.total_files)}
          </div>
          <div class="stat-card-label">Fichiers importés</div>
        </div>
      </div>

      <div class="stat-card accent">
        <div class="stat-card-icon time">T</div>
        <div class="stat-card-body">
          <div class="stat-card-value font-display">
            {formatDuration(s.total_time_saved)}
          </div>
          <div class="stat-card-label">Temps économisé</div>
        </div>
      </div>

      <div class="stat-card">
        <div class="stat-card-icon exports">E</div>
        <div class="stat-card-body">
          <div class="stat-card-value font-display">
            {formatNumber(s.total_exports)}
          </div>
          <div class="stat-card-label">Exports réalisés</div>
        </div>
      </div>

      <div class="stat-card">
        <div class="stat-card-icon accuracy">%</div>
        <div class="stat-card-body">
          <div class="stat-card-value font-display">
            {s.avg_accuracy > 0 ? formatPercent(s.avg_accuracy) : "--"}
          </div>
          <div class="stat-card-label">Précision moyenne</div>
        </div>
      </div>
    </div>

    <!-- Bandeau fusion si projets analysés disponibles -->
    {#if doneProjects.length >= 2}
      <div class="merge-banner">
        <div class="merge-banner-left">
          <span class="merge-icon">⊕</span>
          <div>
            <div class="merge-title">Fusion multi-projets disponible</div>
            <div class="text-muted" style="font-size:12px;">
              {doneProjects.length} projets analysés — assemblez-les en une seule
              timeline Premiere Pro
            </div>
          </div>
        </div>
        <button
          class="btn btn-primary btn-sm"
          on:click={() => (showMerge = true)}
        >
          Fusionner en timeline
        </button>
      </div>
    {/if}

    {#if s.total_projects === 0}
      <div class="empty-state">
        <div class="empty-icon">A</div>
        <h3 class="font-display">Aucun projet encore</h3>
        <p class="text-secondary">
          Crée ton premier projet dans l'éditeur pour voir les statistiques ici.
        </p>
        <button class="btn btn-primary" on:click={() => currentView.set("app")}>
          Ouvrir l'éditeur
        </button>
      </div>
    {/if}
  {:else}
    <div class="empty-state">
      <div class="empty-icon">!</div>
      <h3 class="font-display">Erreur de chargement</h3>
      <p class="text-secondary">
        Impossible de lire les statistiques depuis la base de données.
      </p>
    </div>
  {/if}
</div>

<style>
  .dashboard {
    min-height: 100vh;
    padding: 40px;
    background: var(--bg-base);
    display: flex;
    flex-direction: column;
    gap: 32px;
  }

  .dash-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
  }
  .dash-header h1 {
    font-size: 28px;
    font-weight: 700;
  }

  .header-actions {
    display: flex;
    gap: 10px;
    align-items: center;
  }

  .loading-state {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 60px;
    justify-content: center;
  }
  .spinner {
    animation: spin 1s linear infinite;
    font-size: 20px;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 16px;
  }

  .stat-card {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    padding: 20px;
    display: flex;
    align-items: center;
    gap: 16px;
    transition: border-color 0.15s;
  }
  .stat-card:hover {
    border-color: var(--border-strong);
  }
  .stat-card.accent {
    border-color: rgba(184, 255, 60, 0.3);
    background: var(--accent-subtle);
  }

  .stat-card-icon {
    width: 40px;
    height: 40px;
    border-radius: var(--radius);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 18px;
    font-weight: 700;
    flex-shrink: 0;
  }
  .stat-card-icon.projects {
    background: rgba(100, 120, 255, 0.15);
    color: #6478ff;
  }
  .stat-card-icon.files {
    background: rgba(100, 200, 255, 0.15);
    color: #64c8ff;
  }
  .stat-card-icon.time {
    background: rgba(184, 255, 60, 0.15);
    color: var(--accent);
  }
  .stat-card-icon.exports {
    background: rgba(255, 180, 50, 0.15);
    color: #ffb432;
  }
  .stat-card-icon.accuracy {
    background: rgba(80, 220, 140, 0.15);
    color: var(--success);
  }

  .stat-card-value {
    font-size: 26px;
    font-weight: 700;
    line-height: 1;
  }
  .stat-card-label {
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 4px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  /* ── Bandeau fusion ── */
  .merge-banner {
    background: var(--bg-surface);
    border: 1px solid rgba(184, 255, 60, 0.25);
    border-radius: var(--radius-xl);
    padding: 18px 24px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .merge-banner-left {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .merge-icon {
    font-size: 24px;
  }
  .merge-title {
    font-size: 14px;
    font-weight: 600;
    margin-bottom: 2px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 80px 40px;
    text-align: center;
  }
  .empty-icon {
    width: 64px;
    height: 64px;
    background: var(--bg-elevated);
    border-radius: var(--radius-xl);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 28px;
  }
  .empty-state h3 {
    font-size: 22px;
    font-weight: 700;
  }
  .empty-state p {
    font-size: 14px;
    max-width: 360px;
    line-height: 1.6;
  }
</style>
