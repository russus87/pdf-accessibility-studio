<script>
  // Indice / outline navigabile: cliccando una voce il visore salta alla pagina.
  import { schede } from "../lib/schede.svelte.js";
  import { segnalibri } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);
  let voci = $state(null);
  let caricamento = $state(false);
  let errore = $state(null);

  $effect(() => {
    if (!s) return;
    const id = s.id;
    let annullato = false;
    caricamento = true;
    errore = null;
    voci = null;
    segnalibri(id)
      .then((v) => !annullato && (voci = v))
      .catch((e) => !annullato && (errore = String(e)))
      .finally(() => !annullato && (caricamento = false));
    return () => (annullato = true);
  });
</script>

<div class="pannello">
  <header><h3>Indice / Segnalibri</h3></header>

  {#if caricamento}
    <p class="info">Lettura indice…</p>
  {:else if errore}
    <p class="err">{errore}</p>
  {:else if voci && voci.length}
    <ul>
      {#each voci as v}
        <li>
          <button
            class="voce"
            style={`padding-left:${10 + v.livello * 16}px`}
            disabled={v.pagina == null}
            onclick={() => schede.vaiAPagina(v.pagina)}
          >
            <span class="titolo">{v.titolo || "(senza titolo)"}</span>
            {#if v.pagina != null}<span class="pag">p.{v.pagina + 1}</span>{/if}
          </button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="info">Questo PDF non ha segnalibri.</p>
  {/if}
</div>

<style>
  .pannello {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: auto;
  }
  header {
    padding: 12px 14px;
    border-bottom: 1px solid var(--bordo);
  }
  h3 {
    margin: 0;
    font-size: 15px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 6px 0 16px;
  }
  button.voce {
    display: flex;
    width: 100%;
    gap: 10px;
    align-items: baseline;
    background: transparent;
    border: none;
    color: var(--testo);
    font: inherit;
    text-align: left;
    padding: 6px 10px;
    cursor: pointer;
  }
  button.voce:hover:not(:disabled) {
    background: var(--scheda);
  }
  button.voce:disabled {
    cursor: default;
    color: var(--testo-soft);
  }
  .titolo {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pag {
    margin-left: auto;
    color: var(--testo-soft);
    font-size: 11px;
    flex: none;
  }
  .info,
  .err {
    padding: 14px;
    color: var(--testo-soft);
  }
  .err {
    color: var(--errore);
  }
</style>
