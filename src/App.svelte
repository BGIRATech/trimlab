<!--
  FICHIER : trimlab/src/App.svelte
  ROLE    : Composant racine — router entre Landing / App / Dashboard
  MONTÉ PAR : src/main.ts
-->
<script lang="ts">
  import { currentView, showLicenceModal, showExportModal } from "./lib/store";
  import Landing from "./routes/Landing.svelte";
  import AppEditor from "./routes/App.svelte";
  import Dashboard from "./routes/Dashboard.svelte";
  import LicenceModal from "./components/licence/LicenceModal.svelte";
  import ExportModal from "./components/ExportModal.svelte";
  import Notifications from "./components/Notifications.svelte";
</script>

<!-- Router -->
{#if $currentView === "landing"}
  <Landing />
{:else if $currentView === "app"}
  <AppEditor />
{:else if $currentView === "dashboard"}
  <Dashboard />
{/if}

<!-- Global modals -->
<LicenceModal />
<ExportModal />

<!-- Notifications -->
<Notifications />
