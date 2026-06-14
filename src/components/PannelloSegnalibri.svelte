<script>
  // Indice / outline navigabile: cliccando una voce il visore salta alla pagina.
  import { schede } from "../lib/schede.svelte.js";
  import { segnalibri, generaSegnalibri } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);
  let voci = $state(null);
  let caricamento = $state(false);
  let errore = $state(null);
  let esito = $state(null);

  async function genera() {
    esito = null;
    try {
      const r = await generaSegnalibri(s.id);
      if (r) esito = { ok: true, dest: r.dest, n: r.n };
    } catch (e) {
      esito = { ok: false, msg: String(e) };
    }
  }

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
  <header>
    <h3>Indice / Segnalibri</h3>
    <button class="genera" onclick={genera} disabled={!s}>Genera dai titoli</button>
  </header>

  {#if esito?.ok}
    <p class="esito">Creati {esito.n} segnalibri.
      <button class="link" onclick={() => schede.apri(esito.dest)}>Apri la copia</button>
    </p>
  {:else if esito && !esito.ok}
    <p class="err">{esito.msg}</p>
  {/if}

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
    margin: 0 0 8px;
    font-size: 15px;
  }
  .genera {
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 5px 10px;
    cursor: pointer;
    font-size: 12px;
  }
  .genera:hover {
    border-color: var(--accento);
  }
  .esito {
    padding: 10px 14px;
    margin: 0;
    color: #7ad08f;
    font-size: 13px;
  }
  .link {
    background: none;
    border: none;
    color: var(--accento);
    cursor: pointer;
    text-decoration: underline;
    font-size: 13px;
    padding: 0 0 0 6px;
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
