<script>
  // Toolbar del documento: apertura, creazione da modello, navigazione pagine,
  // zoom e stampa. Opera sul PDF attivo (se presente).
  import { schede } from "../lib/schede.svelte.js";
  import { apriEsterno } from "../lib/api.js";
  import Icona from "./Icona.svelte";

  const s = $derived(schede.pdfAttivo);

  function vai(delta) {
    if (!s) return;
    s.pagina = Math.min(Math.max(s.pagina + delta, 0), s.pagine - 1);
  }
  function zoom(fattore) {
    if (!s) return;
    s.zoom = Math.min(Math.max(Math.round(s.zoom * fattore), 300), 3000);
  }
</script>

<div class="toolbar">
  <button class="primario" onclick={() => schede.apriDaDialogo()} title="Apri un PDF (Ctrl+O)">
    <Icona nome="folder-open" dim={16} /> <span>Apri</span>
  </button>
  <button onclick={() => schede.nuovoCreatore()} title="Crea un nuovo documento da modello">
    <Icona nome="file-plus" dim={16} /> <span>Nuovo</span>
  </button>

  <span class="sep"></span>

  <div class="gruppo">
    <button disabled={!s || s.pagina <= 0} onclick={() => vai(-1)} aria-label="Pagina precedente"><Icona nome="chevron-left" dim={16} /></button>
    <span class="conteggio">{#if s}{s.pagina + 1} / {s.pagine}{:else}— / —{/if}</span>
    <button disabled={!s || s.pagina >= s.pagine - 1} onclick={() => vai(1)} aria-label="Pagina successiva"><Icona nome="chevron-right" dim={16} /></button>
  </div>

  <span class="sep"></span>

  <div class="gruppo">
    <button disabled={!s} onclick={() => zoom(0.8)} aria-label="Riduci zoom"><Icona nome="minus" dim={16} /></button>
    <span class="conteggio">{s ? Math.round((s.zoom / 900) * 100) : 100}%</span>
    <button disabled={!s} onclick={() => zoom(1.25)} aria-label="Aumenta zoom"><Icona nome="plus" dim={16} /></button>
  </div>

  <span class="spazio"></span>

  <button disabled={!s} title="Apri nel programma di sistema (per stampare)" onclick={() => apriEsterno(s.percorso)}>
    <Icona nome="print" dim={16} /> <span>Stampa</span>
  </button>
</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 12px;
    background: var(--barra);
    border-bottom: 1px solid var(--bordo);
  }
  .toolbar button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    color: var(--testo);
    border: 1px solid transparent;
    border-radius: var(--r-2);
    padding: 6px 10px;
    cursor: pointer;
    font-size: 13px;
    min-width: 32px;
    justify-content: center;
    transition: background var(--transizione), border-color var(--transizione);
  }
  .toolbar button:hover:not(:disabled) {
    background: var(--hover);
  }
  .toolbar button.primario {
    background: var(--scheda);
    border-color: var(--bordo);
  }
  .toolbar button.primario:hover {
    border-color: var(--accento);
  }
  .toolbar button:disabled {
    opacity: 0.38;
    cursor: default;
  }
  .gruppo {
    display: flex;
    align-items: center;
    gap: 2px;
    background: var(--scheda);
    border: 1px solid var(--bordo-soft);
    border-radius: var(--r-2);
    padding: 2px;
  }
  .gruppo button {
    padding: 4px 8px;
  }
  .conteggio {
    min-width: 62px;
    text-align: center;
    color: var(--testo-soft);
    font-variant-numeric: tabular-nums;
    font-size: 12.5px;
  }
  .sep {
    width: 1px;
    height: 22px;
    background: var(--bordo);
    margin: 0 4px;
  }
  .spazio {
    flex: 1;
  }
</style>
