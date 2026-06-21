<script>
  // Shell dell'applicazione, in stile VS Code / Acrobat Pro:
  //   barra schede · toolbar documento · [activity bar | sidebar | centro] · barra di stato.
  // La vista centrale dipende dalla scheda attiva (PDF, editor, confronto o
  // "Nuovo da modello"); la sidebar mostra un pannello strumenti per volta.
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { schede } from "./lib/schede.svelte.js";

  // Scorciatoie da tastiera.
  $effect(() => {
    function onkey(e) {
      const tag = (e.target?.tagName || "").toLowerCase();
      const sto_scrivendo = tag === "input" || tag === "textarea" || tag === "select" || e.target?.isContentEditable;
      if (e.ctrlKey && e.key.toLowerCase() === "o") {
        e.preventDefault();
        schede.apriDaDialogo();
        return;
      }
      if (e.ctrlKey && e.key.toLowerCase() === "w") {
        e.preventDefault();
        if (schede.attiva) schede.chiudi(schede.attiva);
        return;
      }
      if (e.ctrlKey && e.key.toLowerCase() === "f") {
        e.preventDefault();
        if (schede.pdfAttivo) schede.mostraPannello("cerca");
        return;
      }
      if (sto_scrivendo) return;
      const s = schede.pdfAttivo;
      if (!s) return;
      if (e.key === "ArrowRight" || e.key === "PageDown") {
        schede.vaiAPagina(Math.min(s.pagina + 1, s.pagine - 1));
      } else if (e.key === "ArrowLeft" || e.key === "PageUp") {
        schede.vaiAPagina(Math.max(s.pagina - 1, 0));
      }
    }
    window.addEventListener("keydown", onkey);
    return () => window.removeEventListener("keydown", onkey);
  });

  // Apertura via trascinamento di file PDF nella finestra.
  $effect(() => {
    let stop;
    getCurrentWebview()
      .onDragDropEvent((e) => {
        if (e.payload.type === "drop") {
          for (const p of e.payload.paths) {
            if (p.toLowerCase().endsWith(".pdf")) schede.apri(p);
          }
        }
      })
      .then((u) => (stop = u))
      .catch(() => {});
    return () => stop && stop();
  });

  import BarraSchede from "./components/BarraSchede.svelte";
  import BarraStrumenti from "./components/BarraStrumenti.svelte";
  import BarraAttivita from "./components/BarraAttivita.svelte";
  import BarraStato from "./components/BarraStato.svelte";
  import Icona from "./components/Icona.svelte";
  import Visore from "./components/Visore.svelte";
  import Anteprime from "./components/Anteprime.svelte";
  import PannelloValidazione from "./components/PannelloValidazione.svelte";
  import PannelloTag from "./components/PannelloTag.svelte";
  import PannelloAutotag from "./components/PannelloAutotag.svelte";
  import PannelloSegnalibri from "./components/PannelloSegnalibri.svelte";
  import LettoreVocale from "./components/LettoreVocale.svelte";
  import PannelloCorrezione from "./components/PannelloCorrezione.svelte";
  import PannelloPagine from "./components/PannelloPagine.svelte";
  import PannelloRicerca from "./components/PannelloRicerca.svelte";
  import PannelloMetadati from "./components/PannelloMetadati.svelte";
  import PannelloModuli from "./components/PannelloModuli.svelte";
  import PannelloStrumenti from "./components/PannelloStrumenti.svelte";
  import PannelloAI from "./components/PannelloAI.svelte";
  import PannelloLibreria from "./components/PannelloLibreria.svelte";
  import Confronto from "./components/Confronto.svelte";
  import Creatore from "./components/Creatore.svelte";
  import Editor from "./components/Editor.svelte";

  // Pannelli laterali ammessi (chi richiede un PDF e chi no).
  const PANNELLI = ["anteprime", "pagine", "indice", "metadati", "cerca", "valida", "correggi", "tag", "autotag", "moduli", "leggi", "ai", "strumenti", "libreria"];
  const SENZA_PDF = ["libreria"];

  const sidebarVisibile = $derived(
    PANNELLI.includes(schede.pannello) && (SENZA_PDF.includes(schede.pannello) || !!schede.pdfAttivo),
  );
  const creatoreAttivo = $derived(schede.schedaAttiva?.tipo === "creatore");
</script>

<main>
  <BarraSchede />
  <BarraStrumenti />

  {#if schede.errore}
    <div class="banner-errore" role="alert">
      <span>{schede.errore}</span>
      <button onclick={() => (schede.errore = null)} aria-label="Chiudi avviso"><Icona nome="x" dim={15} /></button>
    </div>
  {/if}

  <div class="corpo">
    <BarraAttivita />

    {#if sidebarVisibile}
      <aside class="sidebar" class:stretta={schede.pannello === "anteprime"}>
        <button class="chiudi-sidebar" aria-label="Chiudi pannello" title="Chiudi" onclick={() => (schede.pannello = null)}>
          <Icona nome="x" dim={15} />
        </button>
        {#if schede.pannello === "anteprime"}<Anteprime />
        {:else if schede.pannello === "pagine"}<PannelloPagine />
        {:else if schede.pannello === "indice"}<PannelloSegnalibri />
        {:else if schede.pannello === "metadati"}<PannelloMetadati />
        {:else if schede.pannello === "cerca"}<PannelloRicerca />
        {:else if schede.pannello === "valida"}<PannelloValidazione />
        {:else if schede.pannello === "correggi"}<PannelloCorrezione />
        {:else if schede.pannello === "tag"}<PannelloTag />
        {:else if schede.pannello === "autotag"}<PannelloAutotag />
        {:else if schede.pannello === "moduli"}<PannelloModuli />
        {:else if schede.pannello === "leggi"}<LettoreVocale />
        {:else if schede.pannello === "ai"}<PannelloAI />
        {:else if schede.pannello === "strumenti"}<PannelloStrumenti />
        {:else if schede.pannello === "libreria"}<PannelloLibreria />{/if}
      </aside>
    {/if}

    <section class="centro">
      {#if creatoreAttivo}
        <Creatore tab={schede.schedaAttiva} />
      {:else if schede.modoCentro === "confronta" && schede.numeroPdf >= 2}
        <Confronto />
      {:else if schede.modoCentro === "editor" && schede.pdfAttivo}
        <Editor />
      {:else}
        <Visore />
      {/if}
    </section>
  </div>

  <BarraStato />
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }
  .corpo {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
  }
  .sidebar {
    position: relative;
    width: 360px;
    flex: none;
    border-right: 1px solid var(--bordo);
    background: var(--bg-sidebar);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .sidebar.stretta {
    width: 172px;
  }
  .chiudi-sidebar {
    position: absolute;
    top: 8px;
    right: 8px;
    z-index: 5;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    background: transparent;
    color: var(--testo-soft);
    border: none;
    border-radius: var(--r-1);
    cursor: pointer;
  }
  .chiudi-sidebar:hover {
    background: var(--hover);
    color: var(--testo);
  }
  .centro {
    flex: 1;
    display: flex;
    min-width: 0;
    overflow: hidden;
    background: var(--tela);
  }
  .banner-errore {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    background: var(--errore-sfondo);
    color: var(--errore);
    padding: 8px 14px;
    font-size: 13px;
  }
  .banner-errore button {
    display: flex;
    background: transparent;
    border: none;
    color: inherit;
    cursor: pointer;
    padding: 2px;
    border-radius: var(--r-1);
  }
</style>
