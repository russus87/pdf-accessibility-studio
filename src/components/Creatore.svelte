<script>
  // Scheda "Nuovo da modello": editor a foglio unico (intestazione/corpo/piè
  // modificabili in posizione) + schede Dati e Impagina. L'anteprima con i dati
  // sostituiti è a richiesta. Contenuto e opzioni vivono nella scheda (`tab`).
  //
  // Dati: un solo JSON. Oggetto → un documento; Array → stampa unione (un blocco
  // per record, dati[0]…dati[n]).
  import { schede } from "../lib/schede.svelte.js";
  import { modelloEsempio, anteprimaModello, generaDaModello, stampaUnione } from "../lib/api.js";
  import FoglioEditor from "./FoglioEditor.svelte";
  import Icona from "./Icona.svelte";

  let { tab } = $props();

  let vista = $state("documento"); // "documento" | "dati" | "impagina"
  let combina = $state(true);
  let mostraAnteprima = $state(false);
  let htmlAnteprima = $state("");
  let errore = $state(null);
  let esito = $state(null);
  let generando = $state(false);

  const FORMATI = { A4: [210, 297], A5: [148, 210], A3: [297, 420], Letter: [215.9, 279.4], Legal: [215.9, 355.6] };

  // Carica l'esempio iniziale e lo divide in stile condiviso + frammento corpo.
  $effect(() => {
    if (tab.modello || tab.stile) return;
    let annullato = false;
    modelloEsempio()
      .then((e) => {
        if (annullato || tab.modello) return;
        const doc = new DOMParser().parseFromString(e.modello, "text/html");
        tab.stile = [...doc.querySelectorAll("style")].map((s) => s.textContent).join("\n");
        tab.modello = doc.body.innerHTML;
        tab.dati = e.dati;
      })
      .catch(() => {});
    return () => (annullato = true);
  });

  const datiVal = $derived.by(() => {
    try { return JSON.parse(tab.dati || "{}"); } catch (_) { return null; }
  });
  const isArray = $derived(Array.isArray(datiVal));
  const numRecord = $derived(isArray ? datiVal.length : 1);
  const variabili = $derived.by(() => {
    const o = isArray ? datiVal[0] : datiVal;
    return o && typeof o === "object" && !Array.isArray(o) ? Object.keys(o) : [];
  });
  const datiAnteprima = $derived(isArray ? JSON.stringify(datiVal[0] ?? {}) : tab.dati);

  // Anteprima (a richiesta) del corpo con i dati sostituiti.
  $effect(() => {
    if (!mostraAnteprima) return;
    const m = `<style>${tab.stile}</style>${tab.modello}`;
    const d = datiAnteprima;
    const t = setTimeout(() => {
      anteprimaModello(m, d).then((h) => { htmlAnteprima = h; errore = null; }).catch((e) => (errore = String(e)));
    }, 300);
    return () => clearTimeout(t);
  });

  function impostaFormato(nome) {
    const f = FORMATI[nome];
    if (!f) return;
    const orizz = tab.opzioni.larghezzaMm > tab.opzioni.altezzaMm;
    tab.opzioni.larghezzaMm = orizz ? f[1] : f[0];
    tab.opzioni.altezzaMm = orizz ? f[0] : f[1];
  }
  function orientamento(orizz) {
    const w = tab.opzioni.larghezzaMm, h = tab.opzioni.altezzaMm;
    if (orizz && w < h) { tab.opzioni.larghezzaMm = h; tab.opzioni.altezzaMm = w; }
    if (!orizz && w > h) { tab.opzioni.larghezzaMm = h; tab.opzioni.altezzaMm = w; }
  }
  const isOrizz = $derived(tab.opzioni.larghezzaMm > tab.opzioni.altezzaMm);

  async function genera() {
    esito = null;
    if (datiVal === null) { esito = { ok: false, msg: "Dati JSON non validi." }; return; }
    generando = true;
    try {
      if (isArray) {
        const r = await stampaUnione(tab.modello, tab.dati, combina, tab.opzioni);
        if (r) esito = { ok: true, unione: true, ...r };
      } else {
        const dest = await generaDaModello(tab.modello, tab.dati, tab.opzioni, tab.header, tab.footer, tab.stile);
        if (dest) esito = { ok: true, dest };
      }
    } catch (e) {
      esito = { ok: false, msg: String(e) };
    } finally {
      generando = false;
    }
  }
  function apriGenerato() {
    if (esito?.ok && esito.dest) schede.apri(esito.dest);
  }
</script>

<div class="creatore">
  <div class="pannello-editor" class:largo={!mostraAnteprima}>
    <div class="schede-int" role="tablist">
      <button role="tab" aria-selected={vista === "documento"} class:on={vista === "documento"} onclick={() => (vista = "documento")}>
        <Icona nome="type" dim={15} /> Documento
      </button>
      <button role="tab" aria-selected={vista === "dati"} class:on={vista === "dati"} onclick={() => (vista = "dati")}>
        <Icona nome="braces" dim={15} /> Dati{#if isArray} <span class="badge">{numRecord}</span>{/if}
      </button>
      <button role="tab" aria-selected={vista === "impagina"} class:on={vista === "impagina"} onclick={() => (vista = "impagina")}>
        <Icona nome="file" dim={15} /> Impagina
      </button>
      <span class="sp"></span>
      <button class="ant" class:on={mostraAnteprima} onclick={() => (mostraAnteprima = !mostraAnteprima)} title="Mostra/nascondi anteprima con i dati">
        <Icona nome="eye" dim={15} /> Anteprima dati
      </button>
    </div>

    {#if vista === "documento"}
      <FoglioEditor bind:header={tab.header} bind:body={tab.modello} bind:footer={tab.footer} stile={tab.stile} opzioni={tab.opzioni} {variabili} />
    {:else if vista === "dati"}
      <div class="codice">
        <p class="aiuto">
          Valori per le <code>{"{{variabili}}"}</code>. Un <b>oggetto</b> <code>{"{ }"}</code> = un documento; un
          <b>array</b> <code>[ ]</code> = stampa unione (un blocco per elemento).
        </p>
        <textarea bind:value={tab.dati} spellcheck="false" placeholder={'{ "cliente": "Mario Rossi" }  oppure  [ { … }, { … } ]'}></textarea>
      </div>
    {:else}
      <div class="impagina">
        <div class="campo">
          <span class="et">Formato</span>
          <div class="riga">
            <select onchange={(e) => impostaFormato(e.target.value)} aria-label="Formato pagina">
              <option value="">— preimpostati —</option>
              {#each Object.keys(FORMATI) as f}<option value={f}>{f}</option>{/each}
            </select>
            <div class="seg">
              <button class:on={!isOrizz} onclick={() => orientamento(false)}>Verticale</button>
              <button class:on={isOrizz} onclick={() => orientamento(true)}>Orizzontale</button>
            </div>
          </div>
        </div>
        <div class="campo">
          <span class="et">Dimensioni (mm)</span>
          <div class="riga">
            <label class="mini">L <input type="number" min="50" bind:value={tab.opzioni.larghezzaMm} /></label>
            <label class="mini">A <input type="number" min="50" bind:value={tab.opzioni.altezzaMm} /></label>
          </div>
        </div>
        <div class="campo">
          <span class="et">Margini (mm)</span>
          <div class="riga quattro">
            <label class="mini">↑ <input type="number" min="0" bind:value={tab.opzioni.margineAlto} /></label>
            <label class="mini">↓ <input type="number" min="0" bind:value={tab.opzioni.margineBasso} /></label>
            <label class="mini">← <input type="number" min="0" bind:value={tab.opzioni.margineSx} /></label>
            <label class="mini">→ <input type="number" min="0" bind:value={tab.opzioni.margineDx} /></label>
          </div>
        </div>
        <label class="check"><input type="checkbox" bind:checked={tab.opzioni.saltaPrima} /> Salta intestazione/piè sulla prima pagina</label>
        <p class="nota">
          <Icona nome="info" dim={13} /> Intestazione e piè di pagina si attivano e si modificano direttamente sul foglio
          (scheda Documento); si ripetono su ogni pagina del PDF. Usa <code>{"{{PAGENUM}}"}</code> e <code>{"{{TTLPAGES}}"}</code> per i numeri.
        </p>
      </div>
    {/if}

    {#if errore}<p class="err"><Icona nome="info" dim={14} /> {errore}</p>{/if}

    <div class="azioni">
      {#if isArray}
        <label class="cmb"><input type="checkbox" bind:checked={combina} /> Unico file</label>
        <button class="genera" onclick={genera} disabled={generando}>
          <Icona nome="repeat" dim={16} /> {generando ? "Generazione…" : `Genera ${numRecord} documenti…`}
        </button>
        {#if esito?.ok}
          <span class="ok">{esito.n} documenti {esito.combina ? "uniti" : "salvati"}.{#if esito.combina} <button class="link" onclick={() => schede.apri(esito.dest)}>Apri</button>{/if}</span>
        {:else if esito && !esito.ok}<span class="errInline">{esito.msg}</span>{/if}
      {:else}
        <button class="genera" onclick={genera} disabled={generando}>
          <Icona nome="file-plus" dim={16} /> {generando ? "Generazione…" : "Genera PDF…"}
        </button>
        {#if esito?.ok}<span class="ok">Salvato. <button class="link" onclick={apriGenerato}>Apri</button></span>
        {:else if esito && !esito.ok}<span class="errInline">{esito.msg}</span>{/if}
      {/if}
    </div>
  </div>

  {#if mostraAnteprima}
    <div class="anteprima">
      <div class="cap">
        <Icona nome="eye" dim={14} /> Anteprima corpo con dati
        {#if isArray}<span class="cap-info">record 1 di {numRecord}</span>{/if}
      </div>
      <div class="foglio"><iframe title="Anteprima" srcdoc={htmlAnteprima}></iframe></div>
    </div>
  {/if}
</div>

<style>
  .creatore { flex: 1; display: flex; min-width: 0; overflow: hidden; background: var(--sfondo); }
  .pannello-editor { width: 50%; min-width: 420px; display: flex; flex-direction: column; border-right: 1px solid var(--bordo); min-height: 0; }
  .pannello-editor.largo { width: 100%; border-right: none; }
  .schede-int { display: flex; align-items: center; gap: 2px; padding: 8px 10px 0; background: var(--barra); }
  .schede-int button { display: inline-flex; align-items: center; gap: 6px; background: transparent; color: var(--testo-soft); border: none; border-bottom: 2px solid transparent; padding: 8px 14px; cursor: pointer; font-size: 13px; border-radius: var(--r-1) var(--r-1) 0 0; }
  .schede-int button:hover { color: var(--testo); background: var(--hover); }
  .schede-int button.on { color: var(--testo); border-bottom-color: var(--accento); }
  .schede-int .sp { flex: 1; }
  .schede-int .ant { border-bottom: 2px solid transparent; }
  .schede-int .ant.on { color: var(--accento-testo); }
  .badge { background: var(--accento); color: #1b1206; border-radius: 999px; font-size: 11px; font-weight: 700; padding: 0 6px; line-height: 16px; }
  .codice { flex: 1; display: flex; flex-direction: column; min-height: 0; }
  .aiuto { margin: 0; padding: 9px 14px; font-size: 12px; color: var(--testo-soft); background: var(--sfondo); border-bottom: 1px solid var(--bordo-soft); line-height: 1.5; }
  .aiuto code { font-family: ui-monospace, monospace; color: var(--accento-testo); }
  textarea { flex: 1; resize: none; border: none; outline: none; padding: 14px 16px; background: var(--tela); color: var(--testo); font-family: ui-monospace, "Cascadia Code", monospace; font-size: 13px; line-height: 1.6; tab-size: 2; }
  .impagina { flex: 1; overflow-y: auto; padding: 16px; display: flex; flex-direction: column; gap: 16px; }
  .campo { display: flex; flex-direction: column; gap: 6px; }
  .et { font-size: 11px; text-transform: uppercase; letter-spacing: .04em; color: var(--testo-soft); }
  .riga { display: flex; gap: 8px; align-items: center; }
  .riga.quattro { display: grid; grid-template-columns: repeat(4, 1fr); }
  .mini { display: flex; align-items: center; gap: 5px; font-size: 12px; color: var(--testo-soft); }
  .impagina input[type="number"], .impagina select { background: var(--scheda); color: var(--testo); border: 1px solid var(--bordo); border-radius: var(--r-1); padding: 6px 8px; font-size: 13px; }
  .impagina input[type="number"] { width: 72px; }
  .seg { display: flex; gap: 2px; background: var(--scheda); border: 1px solid var(--bordo); border-radius: var(--r-2); padding: 2px; }
  .seg button { background: transparent; color: var(--testo-soft); border: none; border-radius: var(--r-1); padding: 5px 10px; cursor: pointer; font-size: 12px; }
  .seg button.on { background: var(--accento-tenue); color: var(--accento-testo); }
  .check { display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--testo); }
  .nota { display: flex; align-items: flex-start; gap: 7px; margin: 4px 0 0; padding: 10px 12px; font-size: 12px; color: var(--testo-soft); background: var(--scheda); border-radius: var(--r-2); line-height: 1.5; }
  .nota code { font-family: ui-monospace, monospace; color: var(--accento-testo); }
  .azioni { display: flex; align-items: center; gap: 12px; padding: 10px 14px; border-top: 1px solid var(--bordo); background: var(--barra); flex-wrap: wrap; }
  .genera { display: inline-flex; align-items: center; gap: 7px; background: var(--accento); color: #1b1206; border: none; border-radius: var(--r-2); padding: 9px 16px; cursor: pointer; font-size: 13.5px; font-weight: 600; }
  .genera:hover:not(:disabled) { filter: brightness(1.06); }
  .genera:disabled { opacity: .5; cursor: default; }
  .cmb { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--testo-soft); }
  .ok { color: var(--ok); font-size: 13px; }
  .errInline { color: var(--errore); font-size: 13px; }
  .link { background: none; border: none; color: var(--accento-testo); cursor: pointer; text-decoration: underline; padding: 0; font-size: 13px; }
  .err { display: flex; align-items: center; gap: 6px; margin: 0; padding: 8px 14px; color: var(--errore); font-size: 12px; background: var(--errore-sfondo); }
  .anteprima { flex: 1; display: flex; flex-direction: column; min-width: 0; background: var(--tela); }
  .cap { display: flex; align-items: center; gap: 7px; padding: 9px 14px; font-size: 12px; color: var(--testo-soft); background: var(--barra); border-bottom: 1px solid var(--bordo); }
  .cap-info { margin-left: auto; color: var(--accento-testo); }
  .foglio { flex: 1; overflow: auto; padding: 22px; display: flex; justify-content: center; }
  iframe { width: 100%; max-width: 820px; height: 100%; border: none; background: #fff; border-radius: var(--r-2); box-shadow: var(--ombra-pop); }
</style>
