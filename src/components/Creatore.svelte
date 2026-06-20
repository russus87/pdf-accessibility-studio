<script>
  // Creazione di PDF da modello HTML + dati JSON (Fase 22). A sinistra l'editor
  // (modello e dati a schede), a destra l'anteprima HTML renderizzata. Il PDF si
  // genera con lo stesso renderer dei report.
  import { schede } from "../lib/schede.svelte.js";
  import { modelloEsempio, anteprimaModello, generaDaModello, stampaUnione } from "../lib/api.js";

  let modello = $state("");
  let dati = $state("");
  let tab = $state("modello"); // "modello" | "dati" | "unione"
  let datiUnione = $state('[\n  { "nome": "Anna" },\n  { "nome": "Luca" }\n]');
  let combina = $state(true);
  let unioneEsito = $state(null);
  let htmlAnteprima = $state("");
  let errore = $state(null);
  let esito = $state(null);
  let generando = $state(false);

  // Carica l'esempio iniziale una volta.
  $effect(() => {
    let annullato = false;
    modelloEsempio()
      .then((e) => {
        if (annullato || modello) return;
        modello = e.modello;
        dati = e.dati;
      })
      .catch(() => {});
    return () => (annullato = true);
  });

  // Aggiorna l'anteprima (con un piccolo debounce) quando cambiano modello o dati.
  $effect(() => {
    const m = modello;
    const d = dati;
    if (!m) return;
    const t = setTimeout(() => {
      anteprimaModello(m, d)
        .then((h) => {
          htmlAnteprima = h;
          errore = null;
        })
        .catch((e) => (errore = String(e)));
    }, 350);
    return () => clearTimeout(t);
  });

  async function genera() {
    esito = null;
    generando = true;
    try {
      const dest = await generaDaModello(modello, dati);
      if (dest) esito = { ok: true, dest };
    } catch (e) {
      esito = { ok: false, msg: String(e) };
    } finally {
      generando = false;
    }
  }

  function apriGenerato() {
    if (esito?.ok) schede.apri(esito.dest);
  }

  async function generaUnione() {
    unioneEsito = null;
    try {
      const r = await stampaUnione(modello, datiUnione, combina);
      if (r) unioneEsito = { ok: true, ...r };
    } catch (e) {
      unioneEsito = { ok: false, msg: String(e) };
    }
  }
</script>

<div class="creatore">
  <div class="editor">
    <div class="tabs">
      <button class:on={tab === "modello"} onclick={() => (tab = "modello")}>Modello (HTML)</button>
      <button class:on={tab === "dati"} onclick={() => (tab = "dati")}>Dati (JSON)</button>
      <button class:on={tab === "unione"} onclick={() => (tab = "unione")}>Unione</button>
    </div>
    {#if tab === "modello"}
      <textarea bind:value={modello} spellcheck="false" placeholder={"HTML del modello con {{variabili}} e {{#each elenco}}…{{/each}}"}></textarea>
    {:else if tab === "dati"}
      <textarea bind:value={dati} spellcheck="false" placeholder={'{ "chiave": "valore" }'}></textarea>
    {:else}
      <textarea bind:value={datiUnione} spellcheck="false" placeholder={'[ { "nome": "..." }, { "nome": "..." } ]'}></textarea>
    {/if}
    {#if errore}<p class="err">{errore}</p>{/if}
    <div class="azioni">
      {#if tab === "unione"}
        <label class="cmb"><input type="checkbox" bind:checked={combina} /> Unico file</label>
        <button class="genera" onclick={generaUnione}>Genera unione…</button>
        {#if unioneEsito?.ok}
          <span class="ok">{unioneEsito.n} documenti {unioneEsito.combina ? "uniti" : "salvati"}.{#if unioneEsito.combina} <button class="link" onclick={() => schede.apri(unioneEsito.dest)}>Apri</button>{/if}</span>
        {:else if unioneEsito && !unioneEsito.ok}
          <span class="errInline">{unioneEsito.msg}</span>
        {/if}
      {:else}
        <button class="genera" onclick={genera} disabled={generando}>
          {generando ? "Generazione…" : "Genera PDF…"}
        </button>
        {#if esito?.ok}
          <span class="ok">Salvato. <button class="link" onclick={apriGenerato}>Apri</button></span>
        {:else if esito && !esito.ok}
          <span class="errInline">{esito.msg}</span>
        {/if}
      {/if}
    </div>
  </div>

  <div class="anteprima">
    <div class="cap">Anteprima</div>
    <iframe title="Anteprima" srcdoc={htmlAnteprima}></iframe>
  </div>
</div>

<style>
  .creatore {
    flex: 1;
    display: flex;
    min-width: 0;
    overflow: hidden;
  }
  .editor {
    width: 46%;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--bordo);
    min-width: 0;
  }
  .tabs {
    display: flex;
    gap: 6px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--bordo);
  }
  .tabs button {
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 5px 12px;
    cursor: pointer;
    font-size: 12px;
  }
  .tabs button.on {
    background: var(--accento);
    border-color: var(--accento);
    color: #fff;
  }
  textarea {
    flex: 1;
    resize: none;
    border: none;
    outline: none;
    padding: 12px;
    background: var(--sfondo);
    color: var(--testo);
    font-family: ui-monospace, monospace;
    font-size: 13px;
    line-height: 1.5;
  }
  .azioni {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-top: 1px solid var(--bordo);
  }
  .genera {
    background: var(--accento);
    color: #fff;
    border: none;
    border-radius: 8px;
    padding: 9px 16px;
    cursor: pointer;
    font-size: 14px;
  }
  .genera:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .cmb {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--testo-soft);
  }
  .ok {
    color: #7ad08f;
    font-size: 13px;
  }
  .errInline {
    color: var(--errore);
    font-size: 13px;
  }
  .link {
    background: none;
    border: none;
    color: var(--accento);
    cursor: pointer;
    text-decoration: underline;
    padding: 0;
    font-size: 13px;
  }
  .err {
    margin: 0;
    padding: 6px 12px;
    color: var(--errore);
    font-size: 12px;
    background: var(--errore-sfondo);
  }
  .anteprima {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: #525659;
  }
  .cap {
    padding: 8px 12px;
    font-size: 12px;
    color: var(--testo-soft);
    background: var(--barra);
    border-bottom: 1px solid var(--bordo);
  }
  iframe {
    flex: 1;
    border: none;
    background: #fff;
  }
</style>
