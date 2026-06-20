<script>
  // Indice / outline navigabile: cliccando una voce il visore salta alla pagina.
  import { schede } from "../lib/schede.svelte.js";
  import { segnalibri, generaSegnalibri, impostaSegnalibri } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);
  let voci = $state(null);
  let caricamento = $state(false);
  let errore = $state(null);
  let esito = $state(null);
  let modifica = $state(false);
  let bozza = $state([]); // [{ titolo, liv (1-based), pag (1-based) }]

  function avviaModifica() {
    bozza = (voci || []).map((v) => ({ titolo: v.titolo || "", liv: (v.livello ?? 0) + 1, pag: (v.pagina ?? 0) + 1 }));
    modifica = true;
    esito = null;
  }
  function aggiungiVoce() {
    bozza = [...bozza, { titolo: "Nuova voce", liv: 1, pag: (s?.pagina ?? 0) + 1 }];
  }
  function rimuoviVoce(i) {
    bozza = bozza.filter((_, j) => j !== i);
  }
  function muovi(i, d) {
    const j = i + d;
    if (j < 0 || j >= bozza.length) return;
    const a = [...bozza];
    [a[i], a[j]] = [a[j], a[i]];
    bozza = a;
  }
  async function salvaModifiche() {
    esito = null;
    const v = bozza
      .filter((b) => b.titolo.trim())
      .map((b) => ({ titolo: b.titolo.trim(), livello: Math.max(1, b.liv | 0), pagina: Math.max(0, (b.pag | 0) - 1) }));
    try {
      const r = await impostaSegnalibri(s.id, v);
      if (r) { esito = { ok: true, dest: r.dest, n: r.n }; modifica = false; }
    } catch (e) {
      esito = { ok: false, msg: String(e) };
    }
  }

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
    <div class="modi">
      <button class="genera" onclick={genera} disabled={!s}>Genera dai titoli</button>
      {#if !modifica}
        <button class="genera" onclick={avviaModifica} disabled={!s}>Modifica</button>
      {:else}
        <button class="genera" onclick={() => (modifica = false)}>Annulla</button>
      {/if}
    </div>
  </header>

  {#if esito?.ok}
    <p class="esito">Creati {esito.n} segnalibri.
      <button class="link" onclick={() => schede.apri(esito.dest)}>Apri la copia</button>
    </p>
  {:else if esito && !esito.ok}
    <p class="err">{esito.msg}</p>
  {/if}

  {#if modifica}
    <div class="editor">
      {#each bozza as b, i}
        <div class="riga">
          <input class="tit" type="text" bind:value={b.titolo} placeholder="Titolo" />
          <input class="num" type="number" min="1" max="6" bind:value={b.liv} title="Livello" />
          <input class="num" type="number" min="1" bind:value={b.pag} title="Pagina" />
          <button class="ic" onclick={() => muovi(i, -1)} disabled={i === 0} aria-label="Su">▲</button>
          <button class="ic" onclick={() => muovi(i, 1)} disabled={i === bozza.length - 1} aria-label="Giù">▼</button>
          <button class="ic del" onclick={() => rimuoviVoce(i)} aria-label="Elimina">×</button>
        </div>
      {/each}
      <div class="azioni-ed">
        <button class="genera" onclick={aggiungiVoce}>+ Aggiungi voce</button>
        <button class="salva" onclick={salvaModifiche}>Salva segnalibri…</button>
      </div>
    </div>
  {:else if caricamento}
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
  .modi { display: flex; gap: 6px; }
  .editor { display: flex; flex-direction: column; gap: 6px; padding: 10px 12px; }
  .editor .riga { display: flex; gap: 4px; align-items: center; }
  .editor .tit { flex: 1; background: var(--scheda); color: var(--testo); border: 1px solid var(--bordo); border-radius: 6px; padding: 5px 7px; font-size: 13px; min-width: 0; }
  .editor .num { width: 42px; background: var(--scheda); color: var(--testo); border: 1px solid var(--bordo); border-radius: 6px; padding: 5px 4px; font-size: 12px; }
  .editor .ic { background: var(--scheda); color: var(--testo); border: 1px solid var(--bordo); border-radius: 6px; padding: 4px 6px; cursor: pointer; font-size: 11px; }
  .editor .ic.del { color: var(--errore); }
  .editor .ic:disabled { opacity: 0.4; }
  .azioni-ed { display: flex; justify-content: space-between; margin-top: 6px; }
  .salva { background: var(--accento); color: #fff; border: none; border-radius: 8px; padding: 7px 12px; cursor: pointer; font-size: 13px; }
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
