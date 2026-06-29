<script>
  // Activity bar verticale (stile VS Code): sole icone, raggruppate per
  // categoria. Ogni voce apre/chiude un pannello laterale, attiva una vista
  // centrale (editor/confronta) o commuta un interruttore (misura). L'etichetta
  // compare come tooltip al passaggio del mouse.
  import { schede } from "../lib/schede.svelte.js";
  import { applicaTema } from "../lib/tema.js";
  import Icona from "./Icona.svelte";

  const s = $derived(schede.pdfAttivo);

  // tipo: "pannello" (sidebar) | "centro" (vista centrale) | "misura" (toggle)
  const gruppi = [
    {
      nome: "Naviga",
      voci: [
        { id: "anteprime", icona: "anteprime", testo: "Anteprime", tipo: "pannello", pdf: true },
        { id: "pagine", icona: "file", testo: "Pagine", tipo: "pannello", pdf: true },
        { id: "indice", icona: "bookmark", testo: "Indice / segnalibri", tipo: "pannello", pdf: true },
        { id: "metadati", icona: "info", testo: "Metadati", tipo: "pannello", pdf: true },
        { id: "cerca", icona: "cerca", testo: "Cerca", tipo: "pannello", pdf: true },
      ],
    },
    {
      nome: "Accessibilità",
      voci: [
        { id: "valida", icona: "valida", testo: "Valida (PDF/UA)", tipo: "pannello", pdf: true },
        { id: "correggi", icona: "wrench", testo: "Correggi", tipo: "pannello", pdf: true },
        { id: "tag", icona: "tag", testo: "Tag", tipo: "pannello", pdf: true },
        { id: "autotag", icona: "wand", testo: "Auto-tag", tipo: "pannello", pdf: true },
        { id: "ordine", icona: "list", testo: "Anteprima lettura (screen reader)", tipo: "pannello", pdf: true },
        { id: "moduli", icona: "moduli", testo: "Moduli", tipo: "pannello", pdf: true },
        { id: "leggi", icona: "volume", testo: "Leggi ad alta voce", tipo: "pannello", pdf: true },
        { id: "ai", icona: "ai", testo: "AI documento", tipo: "pannello", pdf: true },
      ],
    },
    {
      nome: "Strumenti",
      voci: [
        { id: "strumenti", icona: "settings", testo: "Strumenti PDF", tipo: "pannello", pdf: true },
        { id: "libreria", icona: "folder", testo: "Libreria", tipo: "pannello", pdf: false },
        { id: "misura", icona: "ruler", testo: "Misura", tipo: "misura", pdf: true },
        { id: "editor", icona: "edit", testo: "Editor PDF", tipo: "centro", pdf: true },
        { id: "confronta", icona: "confronta", testo: "Confronta", tipo: "centro", confronto: true },
      ],
    },
  ];

  function attivo(v) {
    if (v.tipo === "pannello") return schede.pannello === v.id;
    if (v.tipo === "centro") return schede.modoCentro === v.id && !!schede.pdfAttivo;
    if (v.tipo === "misura") return schede.misura;
    return false;
  }

  function disabilitato(v) {
    if (v.confronto) return schede.numeroPdf < 2;
    if (v.pdf) return !s;
    return false;
  }

  function aziona(v) {
    if (disabilitato(v)) return;
    if (v.tipo === "pannello") schede.mostraPannello(v.id);
    else if (v.tipo === "centro") schede.mostraCentro(v.id);
    else if (v.tipo === "misura") schede.misura = !schede.misura;
  }

  let tema = $state(document.documentElement.dataset.tema || "scuro");
  function cambiaTema() {
    tema = tema === "scuro" ? "chiaro" : "scuro";
    applicaTema(tema);
  }
</script>

<nav class="attivita" aria-label="Strumenti">
  <div class="voci">
    {#each gruppi as g, i}
      {#if i > 0}<div class="sep" aria-hidden="true"></div>{/if}
      {#each g.voci as v (v.id)}
        <button
          class="voce"
          class:on={attivo(v)}
          disabled={disabilitato(v)}
          aria-pressed={attivo(v)}
          onclick={() => aziona(v)}>
          <Icona nome={v.icona} dim={20} />
          <span class="tip">{v.testo}</span>
        </button>
      {/each}
    {/each}
  </div>

  <div class="fondo">
    <button class="voce" onclick={cambiaTema}>
      <Icona nome={tema === "scuro" ? "sun" : "moon"} dim={20} />
      <span class="tip">Tema {tema === "scuro" ? "chiaro" : "scuro"}</span>
    </button>
  </div>
</nav>

<style>
  .attivita {
    width: 52px;
    flex: none;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    background: var(--bg-attivita);
    border-right: 1px solid var(--bordo-soft);
    padding: 6px 0;
  }
  .voci {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    overflow-y: auto;
    overflow-x: visible;
  }
  .voci::-webkit-scrollbar {
    width: 0;
  }
  .fondo {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-top: 4px;
  }
  .sep {
    width: 24px;
    height: 1px;
    background: var(--bordo);
    margin: 6px 0;
  }
  .voce {
    position: relative;
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    color: var(--testo-soft);
    border: none;
    border-radius: var(--r-2);
    cursor: pointer;
    transition: color var(--transizione), background var(--transizione);
  }
  /* barretta accento a sinistra quando attivo (come VS Code) */
  .voce.on::before {
    content: "";
    position: absolute;
    left: -6px;
    top: 50%;
    transform: translateY(-50%);
    width: 3px;
    height: 22px;
    border-radius: 0 3px 3px 0;
    background: var(--accento);
  }
  .voce:hover:not(:disabled) {
    color: var(--testo);
    background: var(--hover);
  }
  .voce.on {
    color: var(--accento-testo);
    background: var(--accento-tenue);
  }
  .voce:disabled {
    opacity: 0.32;
    cursor: default;
  }
  /* tooltip a comparsa sulla destra */
  .tip {
    position: absolute;
    left: calc(100% + 8px);
    top: 50%;
    transform: translateY(-50%);
    white-space: nowrap;
    background: var(--barra);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: var(--r-1);
    padding: 4px 9px;
    font-size: 12px;
    box-shadow: var(--ombra);
    opacity: 0;
    pointer-events: none;
    z-index: 50;
    transition: opacity var(--transizione);
  }
  .voce:hover .tip {
    opacity: 1;
  }
</style>
