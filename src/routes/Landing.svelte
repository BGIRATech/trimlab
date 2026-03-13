<!--
  FICHIER : trimlab/src/routes/Landing.svelte
  ROLE    : Page marketing (hero, pricing, stack, roadmap)
  ACCÈS   : currentView.set('landing')
-->
<script lang="ts">
  import { currentView, showLicenceModal } from "../lib/store";

  let activeFeature = 0;
  const features = [
    {
      icon: "⚡",
      title: "Stream copy sans perte",
      desc: "Aucun réencodage. Vitesse native FFmpeg. Qualité préservée à 100%.",
    },
    {
      icon: "🤫",
      title: "100% Local & Privé",
      desc: "Zéro octet sur nos serveurs. Votre contenu reste sur votre machine.",
    },
    {
      icon: "⟺",
      title: "Split-screen avant/après",
      desc: "Visualisez le rush original et le résultat coupé côte à côte en lecture synchronisée.",
    },
    {
      icon: "⚡",
      title: "Multi-clips en parallèle",
      desc: "Analysez plusieurs fichiers simultanément. Export timeline unifiée pour Premiere, FCP, Resolve.",
    },
    {
      icon: "🧠",
      title: "IA Whisper embarquée",
      desc: "Détection fillers multilingue. Fonctionne offline, GPU optionnel.",
    },
    {
      icon: "📋",
      title: "Chapitres YouTube auto",
      desc: "Génère les timestamps (00:00 Intro, 02:34…) depuis la transcription. Un clic, un .txt à coller.",
    },
  ];

  const testimonials = [
    {
      name: "Marine L.",
      role: "YouTubeuse — 180K abonnés",
      quote:
        "Je gagne 3h par semaine sur chaque vidéo. TrimLab change vraiment le game.",
      avatar: "ML",
    },
    {
      name: "Thomas R.",
      role: "Podcasteur — Studio Libre",
      quote:
        "La détection de fillers sur mes interviews est bluffante. 86% hit rate.",
      avatar: "TR",
    },
    {
      name: "Camille B.",
      role: "Monteuse freelance",
      quote:
        "Enfin un outil desktop sans abonnement forcé. La licence à vie est folle.",
      avatar: "CB",
    },
  ];

  let licenceKey = "";
  let heroVisible = false;
  setTimeout(() => (heroVisible = true), 100);
</script>

<div class="landing">
  <!-- Nav -->
  <nav class="nav">
    <div class="nav-logo">
      <svg width="28" height="28" viewBox="0 0 28 28" fill="none">
        <rect width="28" height="28" rx="7" fill="#B8FF3C" />
        <path
          d="M7 14 L11 10 L15 16 L19 8 L21 14"
          stroke="#0A0B0D"
          stroke-width="2.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <rect
          x="5"
          y="19"
          width="18"
          height="2"
          rx="1"
          fill="#0A0B0D"
          opacity="0.4"
        />
      </svg>
      <span>TrimLab</span>
      <span class="badge badge-accent">V1</span>
    </div>
    <div class="nav-links">
      <a href="#features">Fonctionnalités</a>
      <a href="#pricing">Tarifs</a>
      <a href="#stack">Stack</a>
    </div>
    <div class="nav-actions">
      <button
        class="btn btn-ghost btn-sm"
        on:click={() => currentView.set("dashboard")}>Dashboard</button
      >
      <button
        class="btn btn-primary btn-sm"
        on:click={() => currentView.set("app")}>Ouvrir l'app →</button
      >
    </div>
  </nav>

  <!-- Hero -->
  <section class="hero" class:visible={heroVisible}>
    <div class="hero-bg">
      <div class="hero-grid"></div>
      <div class="hero-glow"></div>
    </div>
    <div class="hero-content">
      <div class="hero-badge">
        <span class="badge badge-accent">✦ Disponible maintenant</span>
        <span class="badge badge-muted">Local First</span>
      </div>
      <h1 class="hero-title">
        Montez<br />
        <span class="hero-title-accent">sans les silences</span>
      </h1>
      <p class="hero-subtitle">
        TrimLab supprime automatiquement silences, hésitations et fillers
        verbaux. 100% local, 0 cloud, vitesse native. Réduisez votre temps de
        montage de <strong>60 à 85%</strong>.
      </p>
      <div class="hero-cta">
        <button
          class="btn btn-primary btn-lg"
          on:click={() => currentView.set("app")}
        >
          Essai 14 jours — Gratuit
          <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
            <path d="M3 8h10M9 4l4 4-4 4" />
            <path
              d="M9 4l4 4-4 4"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              fill="none"
            />
          </svg>
        </button>
        <button
          class="btn btn-ghost btn-lg"
          on:click={() => showLicenceModal.set(true)}
        >
          Acheter une licence
        </button>
      </div>
      <div class="hero-stats">
        <div class="hero-stat">
          <span class="hero-stat-value">92%</span>
          <span class="hero-stat-label">Précision silences</span>
        </div>
        <div class="hero-stat-divider"></div>
        <div class="hero-stat">
          <span class="hero-stat-value">86%</span>
          <span class="hero-stat-label">Détection fillers</span>
        </div>
        <div class="hero-stat-divider"></div>
        <div class="hero-stat">
          <span class="hero-stat-value">14j</span>
          <span class="hero-stat-label">Essai gratuit</span>
        </div>
        <div class="hero-stat-divider"></div>
        <div class="hero-stat">
          <span class="hero-stat-value">0 cloud</span>
          <span class="hero-stat-label">Tout en local</span>
        </div>
      </div>
    </div>

    <!-- Waveform preview -->
    <div class="hero-preview">
      <div class="hero-preview-bar">
        <span class="hero-preview-dot red"></span>
        <span class="hero-preview-dot yellow"></span>
        <span class="hero-preview-dot green"></span>
        <span
          class="font-mono"
          style="font-size:11px; color:var(--text-muted); margin-left:8px;"
          >interview_ep42.mp4 — 00:42:18</span
        >
      </div>
      <div class="mock-waveform">
        {#each Array(80) as _, i}
          <div
            class="mock-bar"
            class:silence={i >= 15 && i <= 19}
            class:filler={i >= 28 && i <= 29}
            style="--h: {20 +
              Math.sin(i * 0.4) * 30 +
              Math.random() * 25}%; --delay: {i * 20}ms"
          ></div>
        {/each}
        <div class="mock-playhead"></div>
      </div>
      <div class="mock-legend">
        <span><span class="dot keep"></span> Parole conservée</span>
        <span><span class="dot silence"></span> Silence supprimé</span>
        <span><span class="dot filler"></span> Filler détecté</span>
      </div>
      <div class="mock-stats-bar">
        <span class="text-success font-mono" style="font-size:11px;"
          >✓ Durée finale : 31:22 — économie : 10m 56s</span
        >
        <button
          class="btn btn-primary btn-sm"
          disabled
          style="opacity:0.5; cursor:not-allowed; font-size:11px;"
        >
          🔒 Exporter — Achetez une licence
        </button>
      </div>
    </div>
  </section>

  <!-- Features -->
  <section class="section" id="features">
    <div class="section-header">
      <span class="badge badge-muted">Fonctionnalités</span>
      <h2>Tout ce dont vous avez besoin,<br />rien de superflu</h2>
    </div>
    <div class="features-grid">
      {#each features as f, i}
        <button
          class="feature-card"
          class:active={activeFeature === i}
          on:click={() => (activeFeature = i)}
        >
          <div class="feature-icon">{f.icon}</div>
          <div class="feature-body">
            <div class="feature-title">{f.title}</div>
            <div class="feature-desc">{f.desc}</div>
          </div>
          {#if activeFeature === i}
            <div class="feature-indicator"></div>
          {/if}
        </button>
      {/each}
    </div>

    <!-- Feature details -->
    <div class="feature-detail animate-fade-in">
      {#if activeFeature === 0}
        <div class="feature-detail-inner">
          <div class="code-block">
            <span class="code-comment"># Stream copy natif</span><br />
            <span class="code-cmd">$ trimlab analyse</span>
            <span class="code-arg">interview.mp4</span><br />
            <span class="code-out"
              >⟶ Détection silences… <span class="text-success"
                >✓ 23 segments</span
              ></span
            ><br />
            <span class="code-out"
              >⟶ Stream copy export… <span class="text-success"
                >✓ Sans réencodage</span
              ></span
            ><br />
            <span class="code-out"
              >⟶ Durée : 42:18 → <span class="text-accent">31:22</span> (−25,9%)</span
            >
          </div>
        </div>
      {:else if activeFeature === 1}
        <div class="feature-detail-inner">
          <div class="privacy-visual">
            <div class="privacy-machine">💻 Votre machine</div>
            <div class="privacy-arrow">⟶ FFmpeg ⟶ Whisper ⟶ Export</div>
            <div class="privacy-no-cloud">✗ Aucune connexion externe</div>
          </div>
        </div>
      {:else if activeFeature === 2}
        <div class="feature-detail-inner">
          <div class="compare-visual">
            <div class="compare-pane before">
              <span class="compare-label">AVANT</span>
              <div class="compare-wave">
                {#each Array(20) as _, i}
                  <div
                    class="compare-bar"
                    style="height:{12 + Math.sin(i * 0.7) * 18}px"
                  ></div>
                {/each}
              </div>
              <span class="compare-hint">Rush original avec silences</span>
            </div>
            <div class="compare-divider">⟺</div>
            <div class="compare-pane after">
              <span class="compare-label after">APRÈS</span>
              <div class="compare-wave">
                {#each Array(14) as _, i}
                  <div
                    class="compare-bar after"
                    style="height:{14 + Math.sin(i * 0.9) * 16}px"
                  ></div>
                {/each}
              </div>
              <span class="compare-hint">Version coupée — lecture sync</span>
            </div>
          </div>
        </div>
      {:else if activeFeature === 3}
        <div class="feature-detail-inner">
          <div class="batch-visual">
            {#each ["ep40_interview.mp4", "ep41_intro.mp4", "ep42_guest.mp4"] as f, i}
              <div class="batch-row">
                <span
                  class="font-mono"
                  style="font-size:11px; color:var(--text-muted); width:160px"
                  >{f}</span
                >
                <div class="batch-bar-wrap">
                  <div
                    class="batch-bar-fill"
                    style="width:{[100, 78, 45][i]}%"
                  ></div>
                </div>
                <span class="font-mono text-success" style="font-size:10px"
                  >{["✓ Done", "Analyse…", "En attente"][i]}</span
                >
              </div>
            {/each}
            <div
              style="margin-top:12px; font-size:11px; color:var(--text-muted)"
            >
              → Export timeline unifiée : XML / FCPXML / EDL
            </div>
          </div>
        </div>
      {:else if activeFeature === 4}
        <div class="feature-detail-inner">
          <div class="ai-modes">
            {#each ["Lite (CPU only)", "Rapide (GPU opt.)", "Qualité (GPU req.)"] as mode, i}
              <div class="ai-mode" class:recommended={i === 1}>
                <span class="ai-mode-name font-mono">{mode}</span>
                {#if i === 1}<span
                    class="badge badge-accent"
                    style="font-size:9px;">Recommandé</span
                  >{/if}
              </div>
            {/each}
          </div>
        </div>
      {:else}
        <div class="feature-detail-inner">
          <div class="code-block">
            <span class="code-comment"
              ># Chapitres générés depuis la transcription</span
            ><br />
            <span class="code-out">00:00 Introduction</span><br />
            <span class="code-out"
              >02:34 <span class="text-accent">Présentation du projet</span
              ></span
            ><br />
            <span class="code-out">08:17 Démonstration live</span><br />
            <span class="code-out"
              >14:52 <span class="text-accent">Questions / Réponses</span></span
            ><br />
            <span class="code-out">21:06 Conclusion</span><br />
            <br />
            <span class="code-comment"
              ># → Copie en un clic dans la description YouTube</span
            >
          </div>
        </div>
      {/if}
    </div>
  </section>

  <!-- Pricing -->
  <section class="section" id="pricing">
    <div class="section-header">
      <span class="badge badge-muted">Tarifs</span>
      <h2>14 jours d'essai complet.<br />Puis choisissez votre formule.</h2>
      <p class="section-subtitle">
        Toutes les fonctionnalités disponibles pendant l'essai. Aucune carte
        bancaire requise. L'export se débloque avec une licence.
      </p>
    </div>
    <div class="pricing-grid pricing-grid-2">
      <!-- Trial -->
      <div class="pricing-card">
        <div class="pricing-plan">Essai</div>
        <div class="pricing-price">
          <span class="pricing-amount">$0</span>
          <span class="pricing-period">/ 14 jours</span>
        </div>
        <ul class="pricing-features">
          <li>✓ Toutes les fonctionnalités</li>
          <li>✓ Exports illimités</li>
          <li>✓ Split-screen, chapitres, batch</li>
          <li>✓ Sans carte bancaire</li>
          <li class="muted">✗ Expire après 14 jours</li>
        </ul>
        <button
          class="btn btn-ghost"
          style="width:100%"
          on:click={() => currentView.set("app")}
        >
          Commencer l'essai
        </button>
      </div>

      <!-- Lifetime -->
      <div class="pricing-card pricing-featured">
        <div class="pricing-badge">⭐ Offre de lancement</div>
        <div class="pricing-plan">Licence à vie</div>
        <div class="pricing-price">
          <span class="pricing-amount">$49</span>
          <span class="pricing-period">une fois</span>
        </div>
        <div class="pricing-savings">Pas d'abonnement. Pas de surprise.</div>
        <ul class="pricing-features">
          <li>✓ Exports illimités à vie</li>
          <li>✓ Machines illimitées</li>
          <li>✓ Prix définitif garanti</li>
          <li>✓ 14j satisfait ou remboursé</li>
        </ul>
        <button
          class="btn btn-primary"
          style="width:100%"
          on:click={() => showLicenceModal.set(true)}
        >
          Acheter — $49
        </button>
      </div>
    </div>
  </section>

  <!-- Stack / Roadmap -->
  <section class="section" id="stack">
    <div class="section-header">
      <span class="badge badge-muted">Stack & Roadmap</span>
      <h2>Construit pour durer,<br />architecturé avec soin</h2>
    </div>
    <div class="stack-grid">
      {#each [{ tech: "Rust 1.80+", role: "Moteur cœur", color: "#FF7B2C" }, { tech: "Tauri 2.x", role: "Framework Desktop", color: "#24C8DB" }, { tech: "Svelte 5", role: "Interface", color: "#FF3E00" }, { tech: "FFmpeg", role: "Traitement média", color: "#007808" }, { tech: "Whisper.cpp", role: "IA vocale", color: "#B8FF3C" }, { tech: "Custom Licence", role: "Activation offline", color: "#A78BFA" }] as s}
        <div class="stack-item">
          <div class="stack-dot" style="background: {s.color}"></div>
          <div>
            <div class="stack-tech font-mono">{s.tech}</div>
            <div class="stack-role">{s.role}</div>
          </div>
        </div>
      {/each}
    </div>

    <div class="roadmap">
      {#each [{ phase: "Phase 1", label: "PoC & Fondations", weeks: "S1–4", status: "done", desc: "CLI Rust, benchmarks multi-OS, CI GitHub Actions" }, { phase: "Phase 2", label: "MVP & UI", weeks: "S5–10", status: "done", desc: "Waveform interactive, stream copy, licence v1" }, { phase: "Phase 3", label: "IA & Premium", weeks: "S11–16", status: "done", desc: "Whisper local, détection fillers/répétitions, sync A/V externe, sous-titres karaoke" }, { phase: "Phase 4", label: "Polish & Bêta", weeks: "S17–21", status: "done", desc: "Export XML Premiere, split-screen avant/après, batch multi-clips, chapitres YouTube, fix play/pause" }, { phase: "Phase 5", label: "Release & Auto-update", weeks: "S22+", status: "active", desc: "Auto-update Tauri, notarization macOS, distribution publique" }] as r}
        <div
          class="roadmap-item"
          class:done={r.status === "done"}
          class:active={r.status === "active"}
        >
          <div class="roadmap-phase font-mono">{r.phase}</div>
          <div class="roadmap-connector"></div>
          <div class="roadmap-body">
            <div class="roadmap-title">
              {r.label}
              <span class="text-muted font-mono" style="font-size:11px;"
                >{r.weeks}</span
              >
            </div>
            <div class="roadmap-desc">{r.desc}</div>
          </div>
          <div class="roadmap-status">
            {#if r.status === "done"}
              <span class="badge badge-success">✓ Done</span>
            {:else if r.status === "active"}
              <span class="badge badge-warning animate-pulse">En cours</span>
            {:else}
              <span class="badge badge-muted">À venir</span>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  </section>

  <!-- Testimonials -->
  <section class="section">
    <div class="section-header">
      <span class="badge badge-muted">Bêta-testeurs</span>
      <h2>Ils ont testé TrimLab</h2>
    </div>
    <div class="testimonials">
      {#each testimonials as t}
        <div class="testimonial-card card">
          <div class="testimonial-avatar">{t.avatar}</div>
          <p class="testimonial-quote">"{t.quote}"</p>
          <div class="testimonial-author">
            <strong>{t.name}</strong>
            <span class="text-muted">{t.role}</span>
          </div>
        </div>
      {/each}
    </div>
  </section>

  <!-- Footer CTA -->
  <section class="footer-cta">
    <h2>Prêt à récupérer vos heures de montage ?</h2>
    <p>14 jours d'essai complet. Achat unique à $49. Aucun abonnement.</p>
    <button
      class="btn btn-primary btn-lg"
      on:click={() => currentView.set("app")}
    >
      Démarrer l'essai gratuit →
    </button>
  </section>

  <!-- Footer -->
  <footer class="footer">
    <div class="footer-logo">
      <svg width="20" height="20" viewBox="0 0 28 28" fill="none">
        <rect width="28" height="28" rx="7" fill="#B8FF3C" />
        <path
          d="M7 14 L11 10 L15 16 L19 8 L21 14"
          stroke="#0A0B0D"
          stroke-width="2.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
      TrimLab V1
    </div>
    <div class="footer-links">
      <span class="text-muted font-mono" style="font-size:11px;"
        >Version 1.3 — Mars 2026</span
      >
      <span class="text-muted">•</span>
      <span class="text-muted" style="font-size:12px;"
        >Local First. Privacy by design.</span
      >
    </div>
  </footer>
</div>

<style>
  .landing {
    overflow-y: auto;
    overflow-x: hidden;
    height: 100vh;
    scroll-behavior: smooth;
  }

  /* Nav */
  .nav {
    position: sticky;
    top: 0;
    z-index: 100;
    display: flex;
    align-items: center;
    gap: 32px;
    padding: 12px 32px;
    background: rgba(10, 11, 13, 0.85);
    backdrop-filter: blur(16px);
    border-bottom: 1px solid var(--border);
  }
  .nav-logo {
    display: flex;
    align-items: center;
    gap: 8px;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 16px;
    color: var(--text-primary);
  }
  .nav-links {
    display: flex;
    gap: 24px;
    flex: 1;
  }
  .nav-links a {
    color: var(--text-secondary);
    text-decoration: none;
    font-size: 13px;
    transition: color 0.15s;
  }
  .nav-links a:hover {
    color: var(--text-primary);
  }
  .nav-actions {
    display: flex;
    gap: 8px;
  }

  /* Hero */
  .hero {
    position: relative;
    padding: 80px 32px 60px;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 60px;
    align-items: center;
    min-height: 90vh;
    opacity: 0;
    transform: translateY(16px);
    transition:
      opacity 0.6s ease,
      transform 0.6s ease;
  }
  .hero.visible {
    opacity: 1;
    transform: translateY(0);
  }
  .hero-bg {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
  }
  .hero-grid {
    position: absolute;
    inset: 0;
    background-image: linear-gradient(var(--border) 1px, transparent 1px),
      linear-gradient(90deg, var(--border) 1px, transparent 1px);
    background-size: 48px 48px;
    opacity: 0.5;
  }
  .hero-glow {
    position: absolute;
    top: -100px;
    left: 30%;
    width: 600px;
    height: 600px;
    background: radial-gradient(
      circle,
      rgba(184, 255, 60, 0.08) 0%,
      transparent 70%
    );
    pointer-events: none;
  }
  .hero-content {
    position: relative;
    z-index: 1;
  }
  .hero-badge {
    display: flex;
    gap: 8px;
    margin-bottom: 24px;
  }
  .hero-title {
    font-family: var(--font-display);
    font-size: clamp(44px, 5vw, 72px);
    font-weight: 800;
    line-height: 1;
    letter-spacing: -2px;
    margin-bottom: 24px;
  }
  .hero-title-accent {
    color: var(--accent);
    display: block;
  }
  .hero-subtitle {
    font-size: 16px;
    line-height: 1.7;
    color: var(--text-secondary);
    max-width: 480px;
    margin-bottom: 32px;
  }
  .hero-subtitle strong {
    color: var(--text-primary);
  }
  .hero-cta {
    display: flex;
    gap: 12px;
    margin-bottom: 48px;
    flex-wrap: wrap;
  }
  .hero-stats {
    display: flex;
    gap: 0;
    align-items: center;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    overflow: hidden;
    width: fit-content;
  }
  .hero-stat {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 12px 20px;
    gap: 2px;
  }
  .hero-stat-value {
    font-family: var(--font-display);
    font-size: 22px;
    font-weight: 700;
    color: var(--accent);
  }
  .hero-stat-label {
    font-size: 10px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    white-space: nowrap;
  }
  .hero-stat-divider {
    width: 1px;
    height: 40px;
    background: var(--border);
  }

  /* Mock waveform */
  .hero-preview {
    position: relative;
    z-index: 1;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    overflow: hidden;
    box-shadow: var(--shadow-lg), var(--shadow-accent);
  }
  .hero-preview-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 10px 14px;
    background: var(--bg-elevated);
    border-bottom: 1px solid var(--border);
  }
  .hero-preview-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }
  .hero-preview-dot.red {
    background: #ff5f57;
  }
  .hero-preview-dot.yellow {
    background: #febc2e;
  }
  .hero-preview-dot.green {
    background: #28c840;
  }

  .mock-waveform {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 16px 12px;
    height: 100px;
    position: relative;
  }
  .mock-bar {
    flex: 1;
    height: var(--h, 40%);
    background: var(--accent);
    border-radius: 1px;
    opacity: 0.7;
    animation: waveform-pulse 2s ease-in-out infinite;
    animation-delay: var(--delay, 0ms);
  }
  .mock-bar.silence {
    background: rgba(255, 75, 75, 0.4);
  }
  .mock-bar.filler {
    background: rgba(255, 184, 77, 0.6);
  }
  .mock-playhead {
    position: absolute;
    left: 30%;
    top: 8px;
    bottom: 8px;
    width: 1.5px;
    background: white;
    opacity: 0.8;
  }
  .mock-legend {
    display: flex;
    gap: 16px;
    padding: 8px 12px;
    font-size: 10px;
    color: var(--text-muted);
    border-top: 1px solid var(--border);
  }
  .mock-legend span {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 2px;
  }
  .dot.keep {
    background: var(--accent);
  }
  .dot.silence {
    background: rgba(255, 75, 75, 0.7);
  }
  .dot.filler {
    background: rgba(255, 184, 77, 0.7);
  }
  .mock-stats-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    background: var(--bg-elevated);
    border-top: 1px solid var(--border);
  }

  /* Sections */
  .section {
    padding: 80px 32px;
    max-width: 1200px;
    margin: 0 auto;
  }
  .section-header {
    text-align: center;
    margin-bottom: 48px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
  }
  .section-header h2 {
    font-family: var(--font-display);
    font-size: clamp(28px, 3vw, 44px);
    font-weight: 700;
    line-height: 1.1;
    letter-spacing: -1px;
  }
  .section-subtitle {
    color: var(--text-secondary);
    font-size: 15px;
  }

  /* Features */
  .features-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
    margin-bottom: 24px;
  }
  .feature-card {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 20px;
    display: flex;
    gap: 16px;
    cursor: pointer;
    transition: all 0.15s;
    position: relative;
    text-align: left;
    width: 100%;
    overflow: hidden;
  }
  .feature-card.active {
    border-color: var(--border-active);
    background: var(--accent-subtle);
  }
  .feature-icon {
    font-size: 24px;
    flex-shrink: 0;
  }
  .feature-title {
    font-weight: 600;
    color: var(--accent);
    margin-bottom: 4px;
  }
  .feature-desc {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.5;
  }
  .feature-indicator {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 3px;
    background: var(--accent);
    border-radius: 0 2px 2px 0;
  }
  .feature-detail {
    background: var(--bg-surface);
    border: 1px solid var(--border-active);
    border-radius: var(--radius-lg);
    padding: 24px;
  }
  .code-block {
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.8;
    color: var(--text-secondary);
  }
  .code-comment {
    color: var(--text-muted);
  }
  .code-cmd {
    color: var(--accent);
  }
  .code-arg {
    color: var(--info);
  }
  .code-out {
    display: block;
    padding-left: 12px;
  }

  .privacy-visual {
    display: flex;
    align-items: center;
    gap: 24px;
    justify-content: center;
    font-family: var(--font-mono);
    font-size: 13px;
  }
  .privacy-machine {
    color: var(--text-primary);
  }
  .privacy-arrow {
    color: var(--accent);
  }
  .privacy-no-cloud {
    color: var(--danger);
  }

  .ai-modes {
    display: flex;
    gap: 12px;
  }
  .ai-mode {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 10px 16px;
    font-size: 12px;
  }
  .ai-mode.recommended {
    border-color: var(--border-active);
  }

  .accuracy-bars {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .acc-row {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 13px;
  }
  .acc-row span:first-child {
    width: 80px;
    color: var(--text-secondary);
  }

  /* Pricing */
  .pricing-grid-2 {
    grid-template-columns: repeat(2, 1fr);
    max-width: 640px;
    margin: 0 auto;
  }

  .pricing-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 16px;
  }
  .pricing-card {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    position: relative;
    transition: border-color 0.15s;
  }
  .pricing-card:hover {
    border-color: var(--border-strong);
  }
  .pricing-featured {
    border-color: var(--border-active);
    background: var(--accent-subtle);
    box-shadow: var(--shadow-accent);
  }
  .pricing-badge {
    position: absolute;
    top: -12px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--accent);
    color: var(--text-inverse);
    font-size: 10px;
    font-weight: 700;
    padding: 3px 10px;
    border-radius: 100px;
    white-space: nowrap;
  }
  .pricing-plan {
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 16px;
  }
  .pricing-price {
    display: flex;
    align-items: baseline;
    gap: 4px;
  }
  .pricing-amount {
    font-family: var(--font-display);
    font-size: 36px;
    font-weight: 800;
    color: var(--accent);
  }
  .pricing-period {
    color: var(--text-muted);
    font-size: 13px;
  }
  .pricing-savings {
    font-size: 11px;
    color: var(--success);
    margin-top: -8px;
  }
  .pricing-features {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 13px;
    flex: 1;
  }
  .pricing-features li {
    color: var(--text-secondary);
  }
  .pricing-features li.muted {
    color: var(--text-muted);
    text-decoration: line-through;
  }

  /* Stack */
  .stack-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
    margin-bottom: 40px;
  }
  .stack-item {
    display: flex;
    align-items: center;
    gap: 12px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 16px;
  }
  .stack-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .stack-tech {
    font-size: 13px;
    font-weight: 500;
  }
  .stack-role {
    font-size: 11px;
    color: var(--text-muted);
  }

  /* Roadmap */
  .roadmap {
    display: flex;
    flex-direction: column;
    gap: 0;
  }
  .roadmap-item {
    display: grid;
    grid-template-columns: 80px 32px 1fr 100px;
    align-items: center;
    gap: 16px;
    padding: 16px 0;
    border-bottom: 1px solid var(--border);
    opacity: 0.5;
    transition: opacity 0.15s;
  }
  .roadmap-item.done,
  .roadmap-item.active {
    opacity: 1;
  }
  .roadmap-phase {
    font-size: 11px;
    color: var(--text-muted);
  }
  .roadmap-connector {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--border-strong);
    border: 2px solid var(--bg-base);
    justify-self: center;
  }
  .roadmap-item.done .roadmap-connector {
    background: var(--success);
  }
  .roadmap-item.active .roadmap-connector {
    background: var(--warning);
    box-shadow: 0 0 10px var(--warning);
  }
  .roadmap-title {
    font-weight: 600;
    font-size: 14px;
    margin-bottom: 2px;
  }
  .roadmap-desc {
    font-size: 12px;
    color: var(--text-secondary);
  }
  .roadmap-status {
    display: flex;
    justify-content: flex-end;
  }

  /* Testimonials */
  .testimonials {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 16px;
  }
  .testimonial-card {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .testimonial-avatar {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: var(--bg-overlay);
    border: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--accent);
  }
  .testimonial-quote {
    font-size: 14px;
    color: var(--text-secondary);
    line-height: 1.6;
    flex: 1;
    font-style: italic;
  }
  .testimonial-author {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 13px;
  }

  /* Footer CTA */
  .footer-cta {
    text-align: center;
    padding: 80px 32px;
    background: var(--bg-surface);
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
  }
  .footer-cta h2 {
    font-family: var(--font-display);
    font-size: clamp(24px, 3vw, 40px);
    font-weight: 700;
    letter-spacing: -1px;
  }
  .footer-cta p {
    color: var(--text-secondary);
  }

  /* Footer */
  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 32px;
    border-top: 1px solid var(--border);
  }
  .footer-logo {
    display: flex;
    align-items: center;
    gap: 8px;
    font-family: var(--font-display);
    font-size: 13px;
    font-weight: 600;
  }
  .footer-links {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  /* Compare visual */
  .compare-visual {
    display: flex;
    align-items: center;
    gap: 16px;
    justify-content: center;
  }
  .compare-pane {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    background: var(--bg-elevated);
    border: 1px solid rgba(239, 68, 68, 0.35);
    border-radius: var(--radius);
    padding: 12px 16px;
    min-width: 160px;
  }
  .compare-pane.after {
    border-color: rgba(34, 197, 94, 0.35);
  }
  .compare-label {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: rgba(239, 68, 68, 0.9);
    background: rgba(239, 68, 68, 0.1);
    padding: 2px 8px;
    border-radius: 3px;
  }
  .compare-label.after {
    color: rgba(34, 197, 94, 0.9);
    background: rgba(34, 197, 94, 0.1);
  }
  .compare-wave {
    display: flex;
    align-items: center;
    gap: 2px;
    height: 40px;
  }
  .compare-bar {
    width: 5px;
    background: rgba(239, 68, 68, 0.5);
    border-radius: 2px;
  }
  .compare-bar.after {
    background: var(--accent);
    opacity: 0.75;
  }
  .compare-hint {
    font-size: 10px;
    color: var(--text-muted);
  }
  .compare-divider {
    font-size: 20px;
    color: var(--accent);
    flex-shrink: 0;
  }

  /* Batch visual */
  .batch-visual {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .batch-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .batch-bar-wrap {
    flex: 1;
    height: 4px;
    background: var(--border);
    border-radius: 2px;
    overflow: hidden;
  }
  .batch-bar-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 2px;
    transition: width 0.4s ease;
  }
</style>
