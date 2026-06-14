<script>
  // Pannello struttura/tag: mostra l'albero e permette l'export in JSON/XML.
  import { schede } from "../lib/schede.svelte.js";
  import { alberoTag, salvaTag } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);
  let info = $state(null);
  let caricamento = $state(false);
  let errore = $state(null);
  let esito = $state(null);

  $effect(() => {
    if (!s) return;
    const id = s.id;
    let annullato = false;
    caricamento = true;
    errore = null;
    info = null;
    alberoTag(id)
      .then((r) => !annullato && (info = r))
      .catch((e) => !annullato && (errore = String(e)))
      .finally(() => !annullato && (caricamento = false));
    return () => (annullato = true);
  });

  // Appiattisce l'albero in righe indentate per la visualizzazione.
  function righe(nodi, prof = 0, acc = []) {
    for (const n of nodi) {
      acc.push({ prof, ruolo: n.ruolo, alt: n.alt, lang: n.lang, pagina: n.pagina });
      righe(n.figli, prof + 1, acc);
    }
    return acc;
  }

  async function esporta(formato) {
    esito = null;
    try {
      const ok = await salvaTag(s.id, formato);
      if (ok) esito = `Esportato in ${formato.toUpperCase()}.`;
    } catch (e) {
      esito = `Errore: ${e}`;
    }
  }
</script>

<div class="pannello">
  <header>
    <h3>Struttura / Tag</h3>
    {#if info}
      <div class="azioni">
        <button onclick={() => esporta("json")} disabled={!s}>Export JSON</button>
        <button onclick={() => esporta("xml")} disabled={!s}>Export XML</button>
      </div>
    {/if}
  </header>

  {#if esito}<p class="esito">{esito}</p>{/if}

  {#if caricamento}
    <p class="info">Lettura tag…</p>
  {:else if errore}
    <p class="err">{errore}</p>
  {:else if info}
    {#if !info.ha_struct_tree}
      <p class="info">Questo PDF non ha un albero dei tag (non è taggato).</p>
    {:else}
      <p class="suggerimento">Clicca un elemento per saltare alla sua pagina.</p>
      <ul>
        {#each righe(info.radice) as r}
          <li>
            <button
              class="riga"
              style={`padding-left:${8 + r.prof * 16}px`}
              disabled={r.pagina == null}
              onclick={() => schede.vaiAPagina(r.pagina)}
            >
              <span class="ruolo">{r.ruolo}</span>
              {#if r.alt}<span class="alt">alt: {r.alt}</span>{/if}
              {#if r.lang}<span class="lang">{r.lang}</span>{/if}
              {#if r.pagina != null}<span class="pag">p.{r.pagina + 1}</span>{/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
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
    margin: 0 0 8px;
    font-size: 15px;
  }
  .azioni {
    display: flex;
    gap: 8px;
  }
  .azioni button {
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 5px 10px;
    cursor: pointer;
    font-size: 12px;
  }
  .azioni button:hover {
    border-color: var(--accento);
  }
  .suggerimento {
    margin: 0;
    padding: 8px 14px 0;
    font-size: 12px;
    color: var(--testo-soft);
    font-style: italic;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 8px 0 16px;
    font-family: ui-monospace, monospace;
    font-size: 13px;
  }
  li {
    margin: 0;
  }
  button.riga {
    display: flex;
    width: 100%;
    gap: 8px;
    align-items: baseline;
    background: transparent;
    border: none;
    color: var(--testo);
    font: inherit;
    text-align: left;
    padding: 3px 8px;
    cursor: pointer;
    border-radius: 4px;
  }
  button.riga:hover:not(:disabled) {
    background: var(--scheda);
  }
  button.riga:disabled {
    cursor: default;
  }
  .pag {
    margin-left: auto;
    color: var(--testo-soft);
    font-size: 11px;
  }
  .ruolo {
    color: var(--accento);
    font-weight: 600;
  }
  .alt {
    color: var(--testo-soft);
  }
  .lang {
    color: #7ab0ff;
    font-size: 11px;
  }
  .info,
  .err,
  .esito {
    padding: 12px 14px;
    color: var(--testo-soft);
  }
  .err {
    color: var(--errore);
  }
  .esito {
    color: #7ad08f;
  }
</style>
