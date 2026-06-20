<script>
  // Strumenti PDF a livello documento (Fasi 27, 29, 30): protezione con password,
  // esportazione pagine come immagini, numerazione/intestazioni.
  import { schede } from "../lib/schede.svelte.js";
  import {
    pdfProtetto, proteggiPdf, sbloccaPdf,
    esportaImmagini, creaPdfDaImmagini, numeraPagine,
  } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);
  let sezione = $state("protezione"); // protezione | immagini | numerazione
  let esito = $state(null);

  // Protezione.
  let pwUtente = $state("");
  let pwProprietario = $state("");
  let permessi = $state({ stampa: true, copia: false, modifica: false, annotazioni: true });
  let pwSblocco = $state("");
  let protetto = $state(false);
  $effect(() => {
    if (!s) return;
    pdfProtetto(s.id).then((p) => (protetto = p)).catch(() => {});
  });

  // Immagini.
  let imgLarghezza = $state(1500);
  let imgJpeg = $state(false);

  // Numerazione.
  let num = $state({ testo: "Pagina {n} di {tot}", dimPt: 11, colore: "#444444", margineMm: 12, ancora: "basso-centro", inizio: 1 });

  const ancore = [
    ["alto-sinistra", "Alto sx"], ["alto-centro", "Alto centro"], ["alto-destra", "Alto dx"],
    ["basso-sinistra", "Basso sx"], ["basso-centro", "Basso centro"], ["basso-destra", "Basso dx"],
  ];

  async function azione(fn) {
    esito = null;
    try {
      return await fn();
    } catch (e) {
      esito = { ok: false, msg: String(e) };
    }
  }

  async function proteggi() {
    const dest = await azione(() => proteggiPdf(s.id, pwUtente, pwProprietario, permessi));
    if (dest) esito = { ok: true, dest };
  }
  async function sblocca() {
    const dest = await azione(() => sbloccaPdf(s.id, pwSblocco));
    if (dest) esito = { ok: true, dest };
  }
  async function esporta() {
    const r = await azione(() => esportaImmagini(s.id, imgLarghezza, imgJpeg));
    if (r) esito = { ok: true, msg: `${r.length} immagini esportate.` };
  }
  async function importaImmagini(e) {
    const files = [...(e.target.files || [])];
    if (!files.length) return;
    const b64 = await Promise.all(files.map((f) => new Promise((ris, rej) => {
      const fr = new FileReader();
      fr.onload = () => ris(String(fr.result).split(",")[1]);
      fr.onerror = rej;
      fr.readAsDataURL(f);
    })));
    const dest = await azione(() => creaPdfDaImmagini(b64));
    if (dest) esito = { ok: true, dest };
  }
  async function numera() {
    const r = await azione(() => numeraPagine(s.id, num));
    if (r) esito = { ok: true, dest: r.dest };
  }
</script>

<div class="pannello">
  <header><h3>Strumenti PDF</h3></header>
  {#if !s}
    <p class="info">Apri un PDF.</p>
  {:else}
    <div class="tabs">
      <button class:on={sezione === "protezione"} onclick={() => (sezione = "protezione")}>Protezione</button>
      <button class:on={sezione === "immagini"} onclick={() => (sezione = "immagini")}>Immagini</button>
      <button class:on={sezione === "numerazione"} onclick={() => (sezione = "numerazione")}>Numerazione</button>
    </div>

    {#if sezione === "protezione"}
      <div class="sez">
        {#if protetto}<p class="badge">Questo PDF è già protetto.</p>{/if}
        <div class="cap">Proteggi</div>
        <label>Password di apertura<input type="password" bind:value={pwUtente} placeholder="lascia vuoto per non richiederla" /></label>
        <label>Password proprietario<input type="password" bind:value={pwProprietario} placeholder="per permessi/sblocco (opzionale)" /></label>
        <div class="perm">
          <label class="ck"><input type="checkbox" bind:checked={permessi.stampa} /> Stampa</label>
          <label class="ck"><input type="checkbox" bind:checked={permessi.copia} /> Copia testo</label>
          <label class="ck"><input type="checkbox" bind:checked={permessi.modifica} /> Modifica</label>
          <label class="ck"><input type="checkbox" bind:checked={permessi.annotazioni} /> Annotazioni</label>
        </div>
        <button class="vai" onclick={proteggi}>Proteggi e salva…</button>
        <div class="cap">Sblocca</div>
        <label>Password<input type="password" bind:value={pwSblocco} /></label>
        <button class="vai" onclick={sblocca}>Rimuovi protezione e salva…</button>
      </div>

    {:else if sezione === "immagini"}
      <div class="sez">
        <div class="cap">Esporta pagine come immagini</div>
        <label>Larghezza (px)<input type="number" min="200" bind:value={imgLarghezza} /></label>
        <label class="ck"><input type="checkbox" bind:checked={imgJpeg} /> JPEG (altrimenti PNG)</label>
        <button class="vai" onclick={esporta}>Esporta in cartella…</button>
        <div class="cap">Crea PDF da immagini</div>
        <input type="file" accept="image/png,image/jpeg" multiple onchange={importaImmagini} />
      </div>

    {:else if sezione === "numerazione"}
      <div class="sez">
        <label>Testo (usa <code>{"{n}"}</code> e <code>{"{tot}"}</code>)<input type="text" bind:value={num.testo} /></label>
        <div class="riga">
          <label>Dim<input type="number" bind:value={num.dimPt} /></label>
          <label>Colore<input type="color" bind:value={num.colore} /></label>
          <label>Inizio<input type="number" bind:value={num.inizio} /></label>
        </div>
        <div class="riga">
          <label>Posizione
            <select bind:value={num.ancora}>{#each ancore as [v, l]}<option value={v}>{l}</option>{/each}</select>
          </label>
          <label>Margine mm<input type="number" bind:value={num.margineMm} /></label>
        </div>
        <button class="vai" onclick={numera}>Applica e salva…</button>
      </div>
    {/if}

    {#if esito?.ok}
      <div class="ok">{esito.msg || "Salvato."} {#if esito.dest}<button class="link" onclick={() => schede.apri(esito.dest)}>Apri</button>{/if}</div>
    {:else if esito && !esito.ok}
      <div class="errm">{esito.msg}</div>
    {/if}
  {/if}
</div>

<style>
  .pannello { display: flex; flex-direction: column; gap: 10px; height: 100%; overflow: auto; padding-bottom: 16px; }
  header { padding: 12px 14px; border-bottom: 1px solid var(--bordo); }
  h3 { margin: 0; font-size: 15px; }
  .tabs { display: flex; gap: 4px; padding: 0 14px; }
  .tabs button { flex: 1; background: var(--scheda); border: 1px solid var(--bordo); color: var(--testo); border-radius: 6px; padding: 6px 4px; cursor: pointer; font-size: 12px; }
  .tabs button.on { background: var(--accento); border-color: var(--accento); color: #fff; }
  .sez { display: flex; flex-direction: column; gap: 8px; padding: 0 14px; }
  .cap { font-weight: 600; font-size: 13px; margin-top: 6px; border-top: 1px solid var(--bordo); padding-top: 8px; }
  label { display: flex; flex-direction: column; gap: 3px; font-size: 12px; color: var(--testo-soft); }
  .riga { display: flex; gap: 6px; }
  .riga label { flex: 1; }
  .perm { display: grid; grid-template-columns: 1fr 1fr; gap: 4px; }
  .ck { flex-direction: row; align-items: center; gap: 6px; color: var(--testo); }
  input[type="text"], input[type="number"], input[type="password"], select {
    background: var(--scheda); color: var(--testo); border: 1px solid var(--bordo); border-radius: 6px; padding: 6px 8px; font-size: 13px; width: 100%; box-sizing: border-box;
  }
  input[type="color"] { width: 100%; height: 30px; border: 1px solid var(--bordo); border-radius: 6px; }
  code { background: var(--scheda); padding: 0 3px; border-radius: 3px; }
  .vai { background: var(--accento); color: #fff; border: none; border-radius: 8px; padding: 8px 12px; cursor: pointer; font-size: 13px; margin-top: 2px; }
  .badge { background: #4a2326; color: #ff9a9a; border-radius: 8px; padding: 4px 8px; font-size: 12px; margin: 0; }
  .ok { margin: 0 14px; color: #7ad08f; font-size: 13px; }
  .errm { margin: 0 14px; color: var(--errore); font-size: 13px; }
  .link { background: none; border: none; color: var(--accento); text-decoration: underline; cursor: pointer; font-size: 13px; }
  .info { padding: 14px; color: var(--testo-soft); }
</style>
