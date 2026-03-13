<!--
  FICHIER : voicecut/src/components/licence/LicenceModal.svelte
  ROLE    : Modal activation licence (plans, saisie cle, validation)
  DECLENCHE PAR : showLicenceModal store
-->
<script lang="ts">
  import { showLicenceModal, licence, notify } from "../../lib/store";
  import { commands } from "../../lib/commands";

  let key = "";
  let loading = false;
  let error = "";
  let success = false;
  let machineId = "";

  async function loadMachineId() {
    machineId = await commands.getMachineId();
  }
  loadMachineId();

  async function activate() {
    if (!key.trim()) {
      error = "Entrez une cle de licence";
      return;
    }
    loading = true;
    error = "";
    try {
      const res = await commands.validateAndActivateLicence(key.trim());
      if (res.valid && res.plan) {
        licence.update((l) => ({
          ...l,
          status: "lifetime",
          key_hash: key.trim(),
          email: res.email,
          activated_at: new Date().toISOString(),
        }));
        success = true;
        notify({
          type: "success",
          title: "Licence activée !",
          message: "Export débloqué sur cette machine.",
        });
        setTimeout(() => showLicenceModal.set(false), 2000);
      } else {
        error = res.error ?? "Cle invalide";
      }
    } catch (e) {
      error = "Erreur de validation. Reessayez.";
    } finally {
      loading = false;
    }
  }
</script>

{#if $showLicenceModal}
  <div
    class="modal-backdrop"
    role="button"
    tabindex="0"
    on:click={() => showLicenceModal.set(false)}
    on:keydown={(e) => e.key === "Escape" && showLicenceModal.set(false)}
  >
    <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      on:click|stopPropagation
      on:keydown|stopPropagation
    >
      <div class="modal-header">
        <div>
          <h2 class="font-display">Activer une licence</h2>
          <p class="text-muted" style="font-size:13px; margin-top:4px;">
            Débloque tous les exports sur cette machine
          </p>
        </div>
        <button
          class="btn btn-ghost btn-sm"
          on:click={() => showLicenceModal.set(false)}>x</button
        >
      </div>

      {#if success}
        <div class="success-state">
          <div class="success-icon">OK</div>
          <div class="font-display" style="font-size:20px; font-weight:700;">
            Licence activee !
          </div>
          <p class="text-secondary">L'export est maintenant disponible.</p>
        </div>
      {:else}
        <!-- Licence à vie — seul plan disponible -->
        <div class="lifetime-card">
          <div class="lifetime-badge">⭐ Offre de lancement</div>
          <div class="lifetime-header">
            <div>
              <div class="lifetime-name font-display">Licence à vie</div>
              <div class="lifetime-tagline">
                Pas d'abonnement. Pas de surprise.
              </div>
            </div>
            <div class="lifetime-price font-display">
              47€<span class="lifetime-once"> une fois</span>
            </div>
          </div>
          <ul class="lifetime-features">
            <li>✓ Exports vidéo, audio, XML/EDL illimités</li>
            <li>✓ Machines illimitées</li>
            <li>✓ 14 jours satisfait ou remboursé</li>
          </ul>
        </div>

        <div class="key-section">
          <label class="key-label" for="licence-key">Cle de licence</label>
          <div class="key-input-row">
            <input
              id="licence-key"
              class="input font-mono"
              placeholder="TRIMLAB-XXXX-XXXX-XXXX"
              bind:value={key}
              on:keydown={(e) => e.key === "Enter" && activate()}
              autocomplete="off"
              spellcheck="false"
            />
            <button
              class="btn btn-primary"
              on:click={activate}
              disabled={loading}
            >
              {#if loading}
                <span class="animate-spin">o</span>
              {:else}
                Activer
              {/if}
            </button>
          </div>
          {#if error}
            <p class="key-error">{error}</p>
          {/if}
          <p class="key-hint text-muted">Cle recue par email apres achat.</p>
        </div>

        <div class="machine-row">
          <span class="text-muted" style="font-size:11px;">Machine ID :</span>
          <span class="font-mono text-muted" style="font-size:11px;"
            >{machineId}</span
          >
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: relative;
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(8px);
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    animation: fade-in 0.15s ease;
  }
  .modal {
    background: var(--bg-surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-xl);
    padding: 28px;
    width: 560px;
    max-width: calc(100vw - 32px);
    box-shadow: var(--shadow-lg);
    animation: slide-up 0.2s ease;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  .modal-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
  }
  .modal-header h2 {
    font-size: 20px;
    font-weight: 700;
  }
  .lifetime-card {
    background: var(--accent-subtle);
    border: 1.5px solid var(--border-active);
    border-radius: var(--radius-lg);
    padding: 18px 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    position: relative;
  }
  .lifetime-badge {
    position: absolute;
    top: -11px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--accent);
    color: var(--text-inverse);
    font-size: 10px;
    font-weight: 700;
    padding: 3px 12px;
    border-radius: 100px;
    white-space: nowrap;
  }
  .lifetime-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 4px;
  }
  .lifetime-name {
    font-size: 17px;
    font-weight: 700;
  }
  .lifetime-tagline {
    font-size: 12px;
    color: var(--text-muted);
    margin-top: 2px;
  }
  .lifetime-price {
    font-size: 32px;
    font-weight: 800;
    color: var(--accent);
  }
  .lifetime-once {
    font-size: 13px;
    font-weight: 400;
    color: var(--text-muted);
  }
  .lifetime-features {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .lifetime-features li {
    font-size: 12px;
    color: var(--text-secondary);
  }
  .key-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .key-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }
  .key-input-row {
    display: flex;
    gap: 8px;
  }
  .key-error {
    color: var(--danger);
    font-size: 12px;
  }
  .key-hint {
    font-size: 11px;
  }
  .machine-row {
    display: flex;
    gap: 8px;
    align-items: center;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 8px 12px;
  }

  .success-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 32px 0;
    text-align: center;
  }
  .success-icon {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: rgba(60, 255, 160, 0.12);
    border: 2px solid var(--success);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    font-weight: 700;
    color: var(--success);
  }
</style>
