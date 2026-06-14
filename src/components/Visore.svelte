<script>
  // Visore della pagina corrente: chiede al backend il rendering PNG e lo mostra.
  import { schede } from "../lib/schede.svelte.js";
  import { renderPagina, fileRecenti } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);

  // File recenti, mostrati nello stato vuoto.
  let recenti = $state([]);
  const nomeFile = (p) => p.split(/[\\/]/).pop();
  $effect(() => {
    if (!s) fileRecenti().then((r) => (recenti = r)).catch(() => {});
  });

  let src = $state(null);
  let caricamento = $state(false);
  let erroreRender = $state(null);

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
    <img class="pagina" {src} alt={`Pagina ${s.pagina + 1} di ${s.nome}`} />
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
  .pagina {
    max-width: none;
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.5);
    background: #fff;
    border-radius: 2px;
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
