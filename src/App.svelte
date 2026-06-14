<script>
  // Shell dell'applicazione: barra schede, toolbar, visore, pannelli laterali
  // (validazione / tag / lettura) e vista di confronto.
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { schede } from "./lib/schede.svelte.js";

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
  import Visore from "./components/Visore.svelte";
  import Anteprime from "./components/Anteprime.svelte";
  import PannelloValidazione from "./components/PannelloValidazione.svelte";
  import PannelloTag from "./components/PannelloTag.svelte";
  import PannelloSegnalibri from "./components/PannelloSegnalibri.svelte";
  import LettoreVocale from "./components/LettoreVocale.svelte";
  import PannelloCorrezione from "./components/PannelloCorrezione.svelte";
  import PannelloPagine from "./components/PannelloPagine.svelte";
  import Confronto from "./components/Confronto.svelte";

  const lateralePannello = $derived(
    ["valida", "indice", "tag", "leggi", "correggi", "pagine"].includes(schede.pannello) ? schede.pannello : null,
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

  {#if schede.pannello === "confronta"}
    <Confronto />
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
          {:else if lateralePannello === "leggi"}<LettoreVocale />
          {:else if lateralePannello === "correggi"}<PannelloCorrezione />
          {:else if lateralePannello === "pagine"}<PannelloPagine />{/if}
        </aside>
      {/if}
    </div>
  {/if}
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }
  .area {
    flex: 1;
    display: flex;
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
