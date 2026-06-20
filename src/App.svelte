<script>
  // Shell dell'applicazione: barra schede, toolbar, visore, pannelli laterali
  // (validazione / tag / lettura) e vista di confronto.
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { schede } from "./lib/schede.svelte.js";

  // Scorciatoie da tastiera.
  $effect(() => {
    function onkey(e) {
      const tag = (e.target?.tagName || "").toLowerCase();
      const sto_scrivendo = tag === "input" || tag === "textarea" || tag === "select";
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
        if (schede.schedaAttiva) schede.mostraPannello("cerca");
        return;
      }
      if (sto_scrivendo) return;
      const s = schede.schedaAttiva;
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
  import RailStrumenti from "./components/RailStrumenti.svelte";
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
  import Confronto from "./components/Confronto.svelte";
  import Creatore from "./components/Creatore.svelte";
  import Editor from "./components/Editor.svelte";

  const lateralePannello = $derived(
    ["valida", "indice", "tag", "autotag", "leggi", "correggi", "pagine", "cerca", "metadati", "moduli", "strumenti"].includes(schede.pannello) ? schede.pannello : null,
  );
</script>

<main>
  <BarraSchede />
  <BarraStrumenti />

  {#if schede.errore}
    <div class="banner-errore" role="alert">
      {schede.errore}
      <button onclick={() => (schede.errore = null)} aria-label="Chiudi avviso">×</button>
    </div>
  {/if}

  <div class="corpo">
    <RailStrumenti />

    {#if schede.pannello === "confronta"}
      <Confronto />
    {:else if schede.pannello === "crea"}
      <Creatore />
    {:else if schede.pannello === "editor"}
      <Editor />
    {:else}
      <div class="area">
        {#if schede.anteprime && schede.schedaAttiva}
          <Anteprime />
        {/if}
        <Visore />
        {#if lateralePannello}
          <aside class="laterale">
            {#if lateralePannello === "valida"}<PannelloValidazione />
            {:else if lateralePannello === "indice"}<PannelloSegnalibri />
            {:else if lateralePannello === "tag"}<PannelloTag />
            {:else if lateralePannello === "autotag"}<PannelloAutotag />
            {:else if lateralePannello === "leggi"}<LettoreVocale />
            {:else if lateralePannello === "correggi"}<PannelloCorrezione />
            {:else if lateralePannello === "pagine"}<PannelloPagine />
            {:else if lateralePannello === "cerca"}<PannelloRicerca />
            {:else if lateralePannello === "metadati"}<PannelloMetadati />
            {:else if lateralePannello === "moduli"}<PannelloModuli />
            {:else if lateralePannello === "strumenti"}<PannelloStrumenti />{/if}
          </aside>
        {/if}
      </div>
    {/if}
  </div>
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
  .area {
    flex: 1;
    display: flex;
    min-width: 0;
    overflow: hidden;
  }
  .laterale {
    width: 380px;
    flex: none;
    border-left: 1px solid var(--bordo);
    background: var(--sfondo);
    overflow: hidden;
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
    background: transparent;
    border: none;
    color: inherit;
    font-size: 16px;
    cursor: pointer;
  }
</style>
