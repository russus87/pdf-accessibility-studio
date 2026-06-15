<script>
  // Ricerca testuale: trova la query nel documento e mostra le pagine con
  // un estratto; cliccando si salta alla pagina.
  import { schede } from "../lib/schede.svelte.js";
  import { cerca, evidenziaRicerca } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);
  let query = $state("");
  let risultati = $state(null);
  let inCorso = $state(false);
  let errore = $state(null);
  let pagAttiva = $state(null);

  async function esegui() {
    if (!s || !query.trim()) return;
    inCorso = true;
    errore = null;
    risultati = null;
    pagAttiva = null;
    schede.pulisciEvidenziazioni();
    try {
      risultati = await cerca(s.id, query);
      // Evidenzia subito il primo risultato.
      if (risultati.length) await vaiA(risultati[0].pagina);
    } catch (e) {
      errore = String(e);
    } finally {
      inCorso = false;
    }
  }

  // Salta alla pagina ed evidenzia le occorrenze del termine cercato.
  async function vaiA(pagina) {
    pagAttiva = pagina;
    try {
      const rett = await evidenziaRicerca(s.id, query, pagina);
      schede.evidenzia(pagina, rett, "ricerca");
    } catch (_) {
      schede.vaiAPagina(pagina);
    }
  }

  const totale = $derived(risultati ? risultati.reduce((a, r) => a + r.conteggio, 0) : 0);
</script>

<div class="pannello">
  <header>
    <h3>Cerca nel documento</h3>
    <form onsubmit={(e) => { e.preventDefault(); esegui(); }}>
      <input type="search" bind:value={query} placeholder="Testo da cercare…" />
      <button type="submit" disabled={!s || !query.trim()}>Cerca</button>
    </form>
  </header>

  {#if inCorso}
    <p class="info">Ricerca in corso…</p>
  {:else if errore}
    <p class="err">{errore}</p>
  {:else if risultati}
    {#if risultati.length === 0}
      <p class="info">Nessun risultato.</p>
    {:else}
      <p class="conteggio">{totale} occorrenze in {risultati.length} pagine</p>
      <ul>
        {#each risultati as r}
          <li>
            <button class:attiva={pagAttiva === r.pagina} onclick={() => vaiA(r.pagina)}>
              <span class="pag">Pagina {r.pagina + 1} · {r.conteggio}×</span>
              <span class="ctx">{r.contesto}</span>
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
  form {
    display: flex;
    gap: 6px;
  }
  input {
    flex: 1;
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 7px 9px;
  }
  form button {
    background: var(--accento);
    color: #fff;
    border: none;
    border-radius: 6px;
    padding: 7px 14px;
    cursor: pointer;
  }
  form button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .conteggio {
    padding: 8px 14px 0;
    margin: 0;
    color: var(--testo-soft);
    font-size: 12px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 8px 8px 16px;
  }
  li button {
    display: flex;
    flex-direction: column;
    gap: 3px;
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    color: var(--testo);
    padding: 8px 10px;
    border-radius: 6px;
    cursor: pointer;
  }
  li button:hover {
    background: var(--scheda);
  }
  li button.attiva {
    background: var(--scheda);
    box-shadow: inset 3px 0 0 var(--accento);
  }
  .pag {
    color: var(--accento);
    font-size: 12px;
    font-weight: 600;
  }
  .ctx {
    color: var(--testo-soft);
    font-size: 13px;
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
