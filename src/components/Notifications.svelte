<!--
  FICHIER : trimlab/src/components/Notifications.svelte
  ROLE    : Toasts de notification (succès, erreur, info)
  DÉCLENCHÉ PAR : notify() depuis n'importe quel fichier
-->
<script lang="ts">
  import { notifications } from "../lib/store";

  // [FIX BUG F] Permettre la fermeture manuelle d'une notification
  // Indispensable pour les durée=0 (notifications permanentes sans dismiss)
  function dismiss(id: string) {
    notifications.update((ns) => ns.filter((n) => n.id !== id));
  }
</script>

<div class="notif-container">
  {#each $notifications as n (n.id)}
    <div class="notif {n.type} animate-slide-up">
      <div class="notif-icon">
        {n.type === "success"
          ? "✓"
          : n.type === "error"
            ? "✕"
            : n.type === "warning"
              ? "⚠"
              : "ℹ"}
      </div>
      <div class="notif-body">
        <div class="notif-title">{n.title}</div>
        {#if n.message}
          <div class="notif-message">{n.message}</div>
        {/if}
      </div>
      {#if n.action}
        <button class="btn btn-ghost btn-sm" on:click={n.action.fn}
          >{n.action.label}</button
        >
      {/if}
      <!-- [FIX BUG F] Bouton de fermeture manuelle -->
      <button
        class="notif-close"
        on:click={() => dismiss(n.id)}
        aria-label="Fermer la notification">✕</button
      >
    </div>
  {/each}
</div>

<style>
  .notif-container {
    position: fixed;
    bottom: 20px;
    right: 20px;
    z-index: 9999;
    display: flex;
    flex-direction: column;
    gap: 8px;
    pointer-events: none;
  }

  .notif {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    padding: 12px 16px;
    min-width: 280px;
    max-width: 380px;
    box-shadow: var(--shadow-lg);
    pointer-events: all;
  }
  .notif.success {
    border-color: rgba(60, 255, 160, 0.3);
  }
  .notif.error {
    border-color: rgba(255, 75, 75, 0.3);
  }
  .notif.warning {
    border-color: rgba(255, 184, 77, 0.3);
  }

  .notif-icon {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 700;
    flex-shrink: 0;
  }
  .success .notif-icon {
    background: rgba(60, 255, 160, 0.15);
    color: var(--success);
  }
  .error .notif-icon {
    background: rgba(255, 75, 75, 0.15);
    color: var(--danger);
  }
  .warning .notif-icon {
    background: rgba(255, 184, 77, 0.15);
    color: var(--warning);
  }
  .info .notif-icon {
    background: rgba(77, 184, 255, 0.15);
    color: var(--info);
  }

  .notif-body {
    flex: 1;
    min-width: 0;
  }
  .notif-title {
    font-size: 13px;
    font-weight: 600;
  }
  .notif-message {
    font-size: 11px;
    color: var(--text-secondary);
    margin-top: 2px;
  }

  /* [FIX BUG F] Bouton de fermeture manuelle */
  .notif-close {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 11px;
    padding: 0 2px;
    line-height: 1;
    flex-shrink: 0;
    align-self: center;
    opacity: 0.6;
    transition: opacity 0.15s;
  }
  .notif-close:hover {
    opacity: 1;
    color: var(--text-primary);
  }
</style>
