<script>
  // Visore della pagina corrente: chiede al backend il rendering PNG e lo mostra.
  import { schede } from "../lib/schede.svelte.js";
  import { renderPagina, fileRecenti, dimensioniPagina } from "../lib/api.js";
  import StrumentoMisura from "./StrumentoMisura.svelte";

  const s = $derived(schede.schedaAttiva);

  // Strumento di misura: riferimento all'immagine (per i pixel reali) e
  // dimensioni della pagina in punti, richieste al backend quando è attivo.
  let imgEl = $state(null);
  let dimPagina = $state(null);
  $effect(() => {
    if (!s || !schede.misura) {
      dimPagina = null;
      return;
    }
    const id = s.id;
    const pagina = s.pagina;
    let annullato = false;
    dimensioniPagina(id, pagina)
      .then((d) => {
        if (!annullato) dimPagina = d;
      })
      .catch(() => {
        if (!annullato) dimPagina = null;
      });
    return () => {
      annullato = true;
    };
  });

  // File recenti, mostrati nello stato vuoto.
  let recenti = $state([]);
  const nomeFile = (p) => p.split(/[\\/]/).pop();
  $effect(() => {
    if (!s) fileRecenti().then((r) => (recenti = r)).catch(() => {});
  });

  let src = $state(null);
  let caricamento = $state(false);
  let erroreRender = $state(null);

  // Evidenziazioni (ricerca/tag) valide solo per la scheda e pagina correnti.
  const evid = $derived(
    schede.evidenziazioni && s && schede.evidenziazioni.id === s.id && schede.evidenziazioni.pagina === s.pagina
      ? schede.evidenziazioni
      : null,
  );

  // Quando compaiono dei riquadri sulla pagina corrente, porta il primo in vista.
  let overlayEl = $state(null);
  $effect(() => {
    if (src && evid && evid.rettangoli.length && overlayEl) {
      overlayEl.firstElementChild?.scrollIntoView({ block: "center", inline: "center", behavior: "smooth" });
    }
  });

  // Ad ogni cambio di scheda/pagina/zoom richiede il rendering. La closure di
  // cleanup annulla i risultati di richieste ormai superate (evita flicker).
  $effect(() => {
    if (!s) {
      src = null;
      return;
    }
    const id = s.id;
    const pagina = s.pagina;
    const zoom = s.zoom;

    let annullato = false;
    caricamento = true;
    erroreRender = null;

    renderPagina(id, pagina, zoom)
      .then((url) => {
        if (!annullato) src = url;
      })
      .catch((e) => {
        if (!annullato) erroreRender = String(e);
      })
      .finally(() => {
        if (!annullato) caricamento = false;
      });

    return () => {
      annullato = true;
    };
  });
</script>

<div class="visore">
  {#if !s}
    <div class="vuoto">
      <h2>PDF Accessibility Studio</h2>
      <p>Apri un PDF per iniziare, o trascinalo nella finestra.</p>
      <button onclick={() => schede.apriDaDialogo()}>Apri PDF</button>
      {#if recenti.length}
        <div class="recenti">
          <div class="rec-tit">Recenti</div>
          {#each recenti as r}
            <button class="rec" title={r} onclick={() => schede.apri(r)}>{nomeFile(r)}</button>
          {/each}
        </div>
      {/if}
    </div>
  {:else if erroreRender}
    <div class="errore-render">
      <p>Impossibile renderizzare la pagina.</p>
      <code>{erroreRender}</code>
    </div>
  {:else if src}
    <div class="pagina-wrap">
      <img class="pagina" bind:this={imgEl} {src} alt={`Pagina ${s.pagina + 1} di ${s.nome}`} />
      {#if schede.misura && dimPagina}
        <StrumentoMisura {imgEl} larghezzaPt={dimPagina.larghezza} altezzaPt={dimPagina.altezza} />
      {/if}
      {#if evid}
        <div class="overlay" class:tag={evid.tipo === "tag"} bind:this={overlayEl}>
          {#each evid.rettangoli as r}
            <div
              class="box"
              style={`left:${r.x0 * 100}%;top:${r.y0 * 100}%;width:${(r.x1 - r.x0) * 100}%;height:${(r.y1 - r.y0) * 100}%`}
            ></div>
          {/each}
        </div>
      {/if}
    </div>
    {#if caricamento}<div class="spinner">…</div>{/if}
  {:else}
    <div class="spinner grande">Rendering…</div>
  {/if}
</div>

<style>
  .visore {
    position: relative;
    flex: 1;
    overflow: auto;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding: 24px;
    background: var(--tela);
  }
  .pagina-wrap {
    position: relative;
    display: inline-block;
    line-height: 0;
  }
  .pagina {
    max-width: none;
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.5);
    background: #fff;
    border-radius: 2px;
    display: block;
  }
  .overlay {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .box {
    position: absolute;
    background: rgba(255, 213, 0, 0.32);
    border: 1.5px solid rgba(255, 193, 7, 0.95);
    border-radius: 2px;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.15);
  }
  .overlay.tag .box {
    background: rgba(64, 156, 255, 0.22);
    border-color: rgba(64, 156, 255, 0.95);
  }
  .vuoto,
  .errore-render {
    margin: auto;
    text-align: center;
    color: var(--testo-soft);
  }
  .vuoto h2 {
    color: var(--testo);
  }
  .vuoto button {
    margin-top: 12px;
    background: var(--accento);
    color: #fff;
    border: none;
    border-radius: 8px;
    padding: 10px 20px;
    cursor: pointer;
    font-size: 14px;
  }
  .recenti {
    margin-top: 24px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    align-items: center;
  }
  .rec-tit {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 4px;
  }
  button.rec {
    margin: 0;
    background: transparent;
    color: var(--accento);
    border: none;
    padding: 3px 8px;
    font-size: 13px;
    cursor: pointer;
    max-width: 320px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  button.rec:hover {
    text-decoration: underline;
  }
  .errore-render code {
    display: block;
    margin-top: 10px;
    color: var(--errore);
    font-size: 12px;
    word-break: break-word;
  }
  .spinner {
    position: absolute;
    top: 12px;
    right: 16px;
    color: var(--testo-soft);
  }
  .spinner.grande {
    position: static;
    margin: auto;
  }
</style>
