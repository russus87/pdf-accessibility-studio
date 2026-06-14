<script>
  // Toolbar: apertura, navigazione pagine e zoom. Opera sulla scheda attiva.
  import { schede } from "../lib/schede.svelte.js";
  import { apriEsterno } from "../lib/api.js";
  import { applicaTema } from "../lib/tema.js";

  const s = $derived(schede.schedaAttiva);

  let tema = $state(document.documentElement.dataset.tema || "scuro");
  function cambiaTema() {
    tema = tema === "scuro" ? "chiaro" : "scuro";
    applicaTema(tema);
  }

  function vai(delta) {
    if (!s) return;
    const p = Math.min(Math.max(s.pagina + delta, 0), s.pagine - 1);
    s.pagina = p;
  }
  function zoom(fattore) {
    if (!s) return;
    s.zoom = Math.min(Math.max(Math.round(s.zoom * fattore), 300), 3000);
  }
</script>

<div class="toolbar">
  <button onclick={() => schede.apriDaDialogo()}>Apri PDF</button>

  <span class="sep"></span>

  <button disabled={!s || s.pagina <= 0} onclick={() => vai(-1)} aria-label="Pagina precedente">‹</button>
  <span class="conteggio">
    {#if s}{s.pagina + 1} / {s.pagine}{:else}— / —{/if}
  </span>
  <button disabled={!s || s.pagina >= s.pagine - 1} onclick={() => vai(1)} aria-label="Pagina successiva">›</button>

  <span class="sep"></span>

  <button disabled={!s} onclick={() => zoom(0.8)} aria-label="Riduci zoom">−</button>
  <span class="conteggio">{s ? Math.round((s.zoom / 900) * 100) : 100}%</span>
  <button disabled={!s} onclick={() => zoom(1.25)} aria-label="Aumenta zoom">+</button>

  <span class="spazio"></span>

  <button class="strumento" class:on={schede.anteprime} disabled={!s}
    onclick={() => (schede.anteprime = !schede.anteprime)}>Anteprime</button>
  <button class="strumento" class:on={schede.pannello === "pagine"} disabled={!s}
    onclick={() => schede.mostraPannello("pagine")}>Pagine</button>
  <button class="strumento" class:on={schede.pannello === "valida"} disabled={!s}
    onclick={() => schede.mostraPannello("valida")}>Valida</button>
  <button class="strumento" class:on={schede.pannello === "cerca"} disabled={!s}
    onclick={() => schede.mostraPannello("cerca")}>Cerca</button>
  <button class="strumento" class:on={schede.pannello === "indice"} disabled={!s}
    onclick={() => schede.mostraPannello("indice")}>Indice</button>
  <button class="strumento" class:on={schede.pannello === "tag"} disabled={!s}
    onclick={() => schede.mostraPannello("tag")}>Tag</button>
  <button class="strumento" class:on={schede.pannello === "leggi"} disabled={!s}
    onclick={() => schede.mostraPannello("leggi")}>Leggi</button>
  <button class="strumento" class:on={schede.pannello === "correggi"} disabled={!s}
    onclick={() => schede.mostraPannello("correggi")}>Correggi</button>
  <button class="strumento" class:on={schede.pannello === "confronta"} disabled={schede.schede.length < 2}
    onclick={() => schede.mostraPannello("confronta")}>Confronta</button>

  <span class="sep"></span>

  <button class="strumento" disabled={!s} title="Apri nel programma di sistema (per stampare)"
    onclick={() => apriEsterno(s.percorso)}>Stampa/Esterno</button>
  <button class="strumento" title="Cambia tema" onclick={cambiaTema}>{tema === "scuro" ? "☀" : "🌙"}</button>
</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    background: var(--barra);
    border-bottom: 1px solid var(--bordo);
  }
  .toolbar button {
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 6px 12px;
    cursor: pointer;
    min-width: 36px;
  }
  .toolbar button:hover:not(:disabled) {
    border-color: var(--accento);
  }
  .toolbar button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .conteggio {
    min-width: 64px;
    text-align: center;
    color: var(--testo-soft);
    font-variant-numeric: tabular-nums;
  }
  .sep {
    width: 1px;
    height: 22px;
    background: var(--bordo);
    margin: 0 6px;
  }
  .spazio {
    flex: 1;
  }
  .toolbar button.strumento.on {
    background: var(--accento);
    color: #fff;
    border-color: var(--accento);
  }
</style>
