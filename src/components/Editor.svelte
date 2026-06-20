<script>
  // Editor visuale su PDF esistente (Fasi 23-26): canvas della pagina con righelli
  // in millimetri, posizionamento a clic di testo / immagini / firme / campi, e
  // filigrana su tutte le pagine. Salva un unico PDF (overlay + campi).
  import { schede } from "../lib/schede.svelte.js";
  import {
    renderPagina, dimensioniPagina, salvaEditor,
    firmeElenco, firmaSalva, firmaElimina, redigi,
  } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);
  const PT_PER_MM = 72 / 25.4;
  const MM_PER_PT = 25.4 / 72;

  let pagina = $state(0);
  let larghezzaPx = $state(760);
  let dims = $state(null); // { larghezza, altezza } in punti
  let imgUrl = $state("");
  let areaEl = $state(null);
  let cursore = $state(null); // { x_mm, y_mm } sotto al puntatore

  let strumento = $state("testo"); // testo | immagine | firma | campo | redazione
  let elementi = $state([]); // overlay: testo/immagine/firma
  let campi = $state([]); // campi modulo
  let redazioni = $state([]); // aree da redigere
  let red = $state({ larghezza_mm: 60, altezza_mm: 10 });
  let prossimoId = 0;

  // Configurazione dello strumento attivo.
  let txt = $state({ testo: "Testo", dim_pt: 14, colore: "#000000", opacita: 100 });
  let campo = $state({ nome: "campo1", tooltip: "", larghezza_mm: 70, altezza_mm: 8 });
  let imgBozza = $state(null); // { png_base64, larghezza_mm, altezza_mm, opacita }
  let firme = $state([]);
  let firmaSel = $state(null); // { nome, dati_base64 }
  let nuovaFirmaNome = $state("");

  // Filigrana (tutte le pagine).
  let filiga = $state({ attiva: false, tipo: "testo", testo: "RISERVATO", dim_pt: 60, colore: "#c81e1e", opacita: 40, rotazione: 45, png_base64: null, larghezza_mm: 80, altezza_mm: 80 });

  let esito = $state(null);
  let salvando = $state(false);

  // Geometria derivata.
  const ptToMmF = (pt) => pt * MM_PER_PT;
  const pageWmm = $derived(dims ? dims.larghezza * MM_PER_PT : 210);
  const pageHmm = $derived(dims ? dims.altezza * MM_PER_PT : 297);
  const pxPerMm = $derived(larghezzaPx / pageWmm);
  const pageHpx = $derived(pageHmm * pxPerMm);

  // Carica pagina e dimensioni quando cambiano scheda/pagina/zoom.
  $effect(() => {
    if (!s) return;
    const id = s.id;
    const p = pagina;
    const w = larghezzaPx;
    let annullato = false;
    Promise.all([renderPagina(id, p, w), dimensioniPagina(id, p)])
      .then(([url, d]) => {
        if (annullato) return;
        imgUrl = url;
        dims = d;
      })
      .catch((e) => (esito = { ok: false, msg: String(e) }));
    return () => (annullato = true);
  });

  // Carica la libreria firme una volta.
  $effect(() => {
    firmeElenco().then((f) => (firme = f)).catch(() => {});
  });

  function mmDaEvento(e) {
    const r = areaEl.getBoundingClientRect();
    const x = (e.clientX - r.left) / pxPerMm;
    const y = (e.clientY - r.top) / pxPerMm;
    return { x_mm: Math.max(0, Math.round(x * 10) / 10), y_mm: Math.max(0, Math.round(y * 10) / 10) };
  }

  function suMovimento(e) {
    cursore = mmDaEvento(e);
  }

  function suClic(e) {
    if (!dims) return;
    const { x_mm, y_mm } = mmDaEvento(e);
    if (strumento === "testo") {
      elementi.push({ id: prossimoId++, tipo: "testo", pagina, x_mm, y_mm,
        testo: txt.testo, dim_pt: txt.dim_pt, colore: txt.colore, opacita: txt.opacita });
    } else if (strumento === "immagine") {
      if (!imgBozza) { esito = { ok: false, msg: "Scegli prima un'immagine." }; return; }
      elementi.push({ id: prossimoId++, tipo: "immagine", pagina, x_mm, y_mm, ...imgBozza });
    } else if (strumento === "firma") {
      const f = firmaSel;
      if (!f) { esito = { ok: false, msg: "Scegli prima una firma." }; return; }
      elementi.push({ id: prossimoId++, tipo: "firma", pagina, x_mm, y_mm,
        png_base64: f.dati_base64, larghezza_mm: 50, altezza_mm: 20, opacita: 100, nomeFirma: f.nome });
    } else if (strumento === "campo") {
      campi.push({ id: prossimoId++, pagina, x_mm, y_mm,
        nome: campo.nome, tooltip: campo.tooltip, larghezza_mm: campo.larghezza_mm, altezza_mm: campo.altezza_mm });
    } else if (strumento === "redazione") {
      redazioni.push({ id: prossimoId++, pagina, x_mm, y_mm, larghezza_mm: red.larghezza_mm, altezza_mm: red.altezza_mm });
    }
    elementi = [...elementi];
    campi = [...campi];
    redazioni = [...redazioni];
  }

  function rimuovi(lista, id) {
    if (lista === "el") elementi = elementi.filter((x) => x.id !== id);
    else if (lista === "campo") campi = campi.filter((x) => x.id !== id);
    else redazioni = redazioni.filter((x) => x.id !== id);
  }

  async function salvaRedazione() {
    if (!s || !redazioni.length) { esito = { ok: false, msg: "Aggiungi almeno un'area da redigere." }; return; }
    esito = null;
    try {
      const aree = redazioni.map((a) => ({ pagina: a.pagina, x_mm: a.x_mm, y_mm: a.y_mm, larghezza_mm: a.larghezza_mm, altezza_mm: a.altezza_mm }));
      const r = await redigi(s.id, aree);
      if (r) esito = { ok: true, dest: r.dest, n: r.n };
    } catch (e) {
      esito = { ok: false, msg: String(e) };
    }
  }

  // Lettura di un file immagine come base64 (senza prefisso data URL).
  function leggiFileBase64(file) {
    return new Promise((ris, rej) => {
      const fr = new FileReader();
      fr.onload = () => ris(String(fr.result).split(",")[1]);
      fr.onerror = rej;
      fr.readAsDataURL(file);
    });
  }

  async function scegliImmagine(e) {
    const file = e.target.files?.[0];
    if (!file) return;
    const b64 = await leggiFileBase64(file);
    imgBozza = { png_base64: b64, larghezza_mm: 40, altezza_mm: 25, opacita: 100 };
    strumento = "immagine";
  }

  async function importaFirma(e) {
    const file = e.target.files?.[0];
    if (!file) return;
    const b64 = await leggiFileBase64(file);
    const nome = nuovaFirmaNome.trim() || file.name.replace(/\.[^.]+$/, "");
    try {
      await firmaSalva(nome, b64);
      firme = await firmeElenco();
      firmaSel = firme.find((f) => f.nome === nome) || null;
      nuovaFirmaNome = "";
      strumento = "firma";
    } catch (err) {
      esito = { ok: false, msg: String(err) };
    }
  }

  async function eliminaFirma(nome) {
    await firmaElimina(nome);
    firme = await firmeElenco();
    if (firmaSel?.nome === nome) firmaSel = null;
  }

  async function filigImmagine(e) {
    const file = e.target.files?.[0];
    if (!file) return;
    filiga.png_base64 = await leggiFileBase64(file);
  }

  async function salva() {
    if (!s) return;
    esito = null;
    salvando = true;
    try {
      const els = elementi.map((x) => ({
        tipo: x.tipo === "testo" ? "testo" : "immagine",
        pagina: x.pagina, x_mm: x.x_mm, y_mm: x.y_mm,
        opacita: x.opacita ?? 255, rotazione: x.rotazione ?? 0,
        testo: x.testo ?? null, dim_pt: x.dim_pt ?? null, colore: x.colore ?? null,
        png_base64: x.png_base64 ?? null, larghezza_mm: x.larghezza_mm ?? null, altezza_mm: x.altezza_mm ?? null,
      }));
      const cps = campi.map((c) => ({
        pagina: c.pagina, x_mm: c.x_mm, y_mm: c.y_mm,
        larghezza_mm: c.larghezza_mm, altezza_mm: c.altezza_mm,
        nome: c.nome, tooltip: c.tooltip || null, valore: null,
      }));
      let fil = null;
      if (filiga.attiva) {
        fil = filiga.tipo === "immagine"
          ? { tipo: "immagine", png_base64: filiga.png_base64, larghezza_mm: filiga.larghezza_mm, altezza_mm: filiga.altezza_mm, opacita: filiga.opacita, rotazione: filiga.rotazione }
          : { tipo: "testo", testo: filiga.testo, dim_pt: filiga.dim_pt, colore: filiga.colore, opacita: filiga.opacita, rotazione: filiga.rotazione };
      }
      if (!els.length && !cps.length && !fil) { esito = { ok: false, msg: "Aggiungi almeno un elemento." }; salvando = false; return; }
      const r = await salvaEditor(s.id, els, fil, cps);
      if (r) esito = { ok: true, dest: r.dest, n: r.n };
    } catch (e) {
      esito = { ok: false, msg: String(e) };
    } finally {
      salvando = false;
    }
  }

  const elementiPagina = $derived(elementi.filter((x) => x.pagina === pagina));
  const campiPagina = $derived(campi.filter((x) => x.pagina === pagina));
  const redazioniPagina = $derived(redazioni.filter((x) => x.pagina === pagina));
  // Tacche del righello ogni 10 mm.
  const tacche = (lungMm) => Array.from({ length: Math.ceil(lungMm / 10) + 1 }, (_, i) => i * 10);
</script>

{#if !s}
  <div class="vuoto">Apri un PDF per modificarlo con l'editor.</div>
{:else}
  <div class="editor">
    <aside class="pannello">
      <div class="nav">
        <button onclick={() => (pagina = Math.max(0, pagina - 1))} disabled={pagina === 0}>‹</button>
        <span>Pag. {pagina + 1} / {s.pagine}</span>
        <button onclick={() => (pagina = Math.min(s.pagine - 1, pagina + 1))} disabled={pagina >= s.pagine - 1}>›</button>
      </div>

      <div class="strumenti">
        {#each [["testo", "Testo"], ["immagine", "Immagine"], ["firma", "Firma"], ["campo", "Campo"], ["redazione", "Redazione"]] as [k, lab]}
          <button class:on={strumento === k} onclick={() => (strumento = k)}>{lab}</button>
        {/each}
      </div>

      {#if strumento === "testo"}
        <div class="cfg">
          <label>Testo<input type="text" bind:value={txt.testo} /></label>
          <div class="riga">
            <label>Dim<input type="number" min="4" bind:value={txt.dim_pt} /></label>
            <label>Colore<input type="color" bind:value={txt.colore} /></label>
            <label>Opacità<input type="number" min="0" max="100" bind:value={txt.opacita} /></label>
          </div>
          <p class="hint">Clicca sulla pagina per posizionare il testo.</p>
        </div>
      {:else if strumento === "immagine"}
        <div class="cfg">
          <input type="file" accept="image/png,image/jpeg" onchange={scegliImmagine} />
          {#if imgBozza}
            <div class="riga">
              <label>L.mm<input type="number" min="1" bind:value={imgBozza.larghezza_mm} /></label>
              <label>A.mm<input type="number" min="1" bind:value={imgBozza.altezza_mm} /></label>
              <label>Opac<input type="number" min="0" max="100" bind:value={imgBozza.opacita} /></label>
            </div>
            <p class="hint">Clicca sulla pagina per posizionare l'immagine.</p>
          {/if}
        </div>
      {:else if strumento === "firma"}
        <div class="cfg">
          {#if firme.length}
            <div class="firme">
              {#each firme as f}
                <div class="firma" class:sel={firmaSel?.nome === f.nome}>
                  <button class="pick" onclick={() => (firmaSel = f)}>
                    <img src={`data:image/png;base64,${f.dati_base64}`} alt={f.nome} />
                    <span>{f.nome}</span>
                  </button>
                  <button class="del" title="Elimina" onclick={() => eliminaFirma(f.nome)}>×</button>
                </div>
              {/each}
            </div>
          {:else}
            <p class="hint">Nessuna firma salvata. Importane una qui sotto.</p>
          {/if}
          <label>Nome nuova firma<input type="text" bind:value={nuovaFirmaNome} placeholder="es. Firma Mario" /></label>
          <input type="file" accept="image/png" onchange={importaFirma} />
          {#if firmaSel}<p class="hint">Firma «{firmaSel.nome}» — clicca sulla pagina per applicarla.</p>{/if}
        </div>
      {:else if strumento === "campo"}
        <div class="cfg">
          <label>Nome campo<input type="text" bind:value={campo.nome} /></label>
          <label>Etichetta (tooltip)<input type="text" bind:value={campo.tooltip} /></label>
          <div class="riga">
            <label>L.mm<input type="number" min="5" bind:value={campo.larghezza_mm} /></label>
            <label>A.mm<input type="number" min="4" bind:value={campo.altezza_mm} /></label>
          </div>
          <p class="hint">Clicca sulla pagina per posizionare il campo.</p>
        </div>
      {:else if strumento === "redazione"}
        <div class="cfg">
          <div class="riga">
            <label>L.mm<input type="number" min="2" bind:value={red.larghezza_mm} /></label>
            <label>A.mm<input type="number" min="2" bind:value={red.altezza_mm} /></label>
          </div>
          <p class="hint">Clicca per aggiungere un'area da oscurare <b>definitivamente</b> (il contenuto sotto viene rimosso).</p>
          {#if redazioni.length}
            <button class="salva rosso" onclick={salvaRedazione}>Salva con redazione ({redazioni.length})…</button>
          {/if}
        </div>
      {/if}

      <div class="filigrana">
        <label class="check"><input type="checkbox" bind:checked={filiga.attiva} /> Filigrana (tutte le pagine)</label>
        {#if filiga.attiva}
          <div class="strumenti due">
            <button class:on={filiga.tipo === "testo"} onclick={() => (filiga.tipo = "testo")}>Testo</button>
            <button class:on={filiga.tipo === "immagine"} onclick={() => (filiga.tipo = "immagine")}>Immagine</button>
          </div>
          {#if filiga.tipo === "testo"}
            <label>Testo<input type="text" bind:value={filiga.testo} /></label>
            <div class="riga">
              <label>Dim<input type="number" bind:value={filiga.dim_pt} /></label>
              <label>Colore<input type="color" bind:value={filiga.colore} /></label>
            </div>
          {:else}
            <input type="file" accept="image/png" onchange={filigImmagine} />
            <div class="riga"><label>L.mm<input type="number" bind:value={filiga.larghezza_mm} /></label><label>A.mm<input type="number" bind:value={filiga.altezza_mm} /></label></div>
          {/if}
          <div class="riga">
            <label>Opacità<input type="number" min="0" max="100" bind:value={filiga.opacita} /></label>
            <label>Rotaz°<input type="number" bind:value={filiga.rotazione} /></label>
          </div>
        {/if}
      </div>

      {#if elementi.length || campi.length}
        <div class="lista">
          <div class="cap">Elementi ({elementi.length + campi.length})</div>
          {#each elementi as x}
            <div class="voce">
              <span>{x.tipo === "testo" ? `T: ${x.testo}` : x.nomeFirma ? `Firma: ${x.nomeFirma}` : "Immagine"} — p{x.pagina + 1} ({x.x_mm},{x.y_mm})</span>
              <button onclick={() => rimuovi("el", x.id)}>×</button>
            </div>
          {/each}
          {#each campi as c}
            <div class="voce">
              <span>Campo «{c.nome}» — p{c.pagina + 1} ({c.x_mm},{c.y_mm})</span>
              <button onclick={() => rimuovi("campo", c.id)}>×</button>
            </div>
          {/each}
        </div>
      {/if}

      <button class="salva" onclick={salva} disabled={salvando}>{salvando ? "Salvataggio…" : "Salva PDF…"}</button>
      {#if esito?.ok}
        <div class="ok">Salvato ({esito.n} elementi). <button class="link" onclick={() => schede.apri(esito.dest)}>Apri</button></div>
      {:else if esito && !esito.ok}
        <div class="errm">{esito.msg}</div>
      {/if}
    </aside>

    <div class="tela">
      <div class="coord">{cursore ? `${cursore.x_mm} × ${cursore.y_mm} mm` : `${Math.round(pageWmm)} × ${Math.round(pageHmm)} mm`}</div>
      <div class="scroll">
        <div class="griglia" style={`grid-template-columns: 24px ${larghezzaPx}px;`}>
          <div class="angolo"></div>
          <!-- Righello orizzontale -->
          <div class="rig-h" style={`width:${larghezzaPx}px;`}>
            {#each tacche(pageWmm) as m}
              <span class="t" style={`left:${m * pxPerMm}px;`}>{m}</span>
            {/each}
          </div>
          <!-- Righello verticale -->
          <div class="rig-v" style={`height:${pageHpx}px;`}>
            {#each tacche(pageHmm) as m}
              <span class="t" style={`top:${m * pxPerMm}px;`}>{m}</span>
            {/each}
          </div>
          <!-- Pagina + overlay -->
          <div class="pagina" bind:this={areaEl} role="presentation"
            style={`width:${larghezzaPx}px; height:${pageHpx}px;`}
            onmousemove={suMovimento} onclick={suClic}>
            {#if imgUrl}<img src={imgUrl} alt={`Pagina ${pagina + 1}`} draggable="false" />{/if}
            {#each elementiPagina as x}
              <div class="marker" style={`left:${x.x_mm * pxPerMm}px; top:${x.y_mm * pxPerMm}px;`} title={x.tipo}>
                {x.tipo === "testo" ? "T" : "🖼"}
              </div>
            {/each}
            {#each campiPagina as c}
              <div class="campo-box" style={`left:${c.x_mm * pxPerMm}px; top:${c.y_mm * pxPerMm}px; width:${c.larghezza_mm * pxPerMm}px; height:${c.altezza_mm * pxPerMm}px;`}>{c.nome}</div>
            {/each}
            {#each redazioniPagina as a}
              <div class="red-box" style={`left:${a.x_mm * pxPerMm}px; top:${a.y_mm * pxPerMm}px; width:${a.larghezza_mm * pxPerMm}px; height:${a.altezza_mm * pxPerMm}px;`}></div>
            {/each}
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .vuoto { padding: 40px; color: var(--testo-soft); }
  .editor { flex: 1; display: flex; min-width: 0; overflow: hidden; }
  .pannello {
    width: 300px; flex: none; display: flex; flex-direction: column; gap: 10px;
    padding: 12px; overflow-y: auto; border-right: 1px solid var(--bordo); background: var(--barra);
  }
  .nav { display: flex; align-items: center; justify-content: space-between; font-size: 13px; }
  .nav button { background: var(--scheda); border: 1px solid var(--bordo); color: var(--testo); border-radius: 6px; padding: 2px 10px; cursor: pointer; }
  .strumenti { display: flex; gap: 4px; flex-wrap: wrap; }
  .strumenti.due button { flex: 1; }
  .strumenti button {
    flex: 1; background: var(--scheda); border: 1px solid var(--bordo); color: var(--testo);
    border-radius: 6px; padding: 6px 4px; cursor: pointer; font-size: 12px;
  }
  .strumenti button.on { background: var(--accento); border-color: var(--accento); color: #fff; }
  .cfg, .filigrana, .lista { display: flex; flex-direction: column; gap: 6px; border-top: 1px solid var(--bordo); padding-top: 8px; }
  label { display: flex; flex-direction: column; gap: 3px; font-size: 12px; color: var(--testo-soft); }
  .riga { display: flex; gap: 6px; }
  .riga label { flex: 1; }
  input[type="text"], input[type="number"] {
    background: var(--sfondo); color: var(--testo); border: 1px solid var(--bordo); border-radius: 6px; padding: 5px 7px; font-size: 13px; width: 100%; box-sizing: border-box;
  }
  input[type="color"] { width: 100%; height: 30px; border: 1px solid var(--bordo); border-radius: 6px; background: var(--sfondo); }
  .check { flex-direction: row; align-items: center; gap: 8px; color: var(--testo); font-size: 13px; }
  .hint { margin: 0; font-size: 11px; color: var(--testo-soft); font-style: italic; }
  .firme { display: flex; flex-direction: column; gap: 4px; max-height: 160px; overflow: auto; }
  .firma { display: flex; align-items: center; gap: 4px; }
  .firma.sel { outline: 2px solid var(--accento); border-radius: 6px; }
  .firma .pick { flex: 1; display: flex; align-items: center; gap: 8px; background: var(--scheda); border: 1px solid var(--bordo); border-radius: 6px; padding: 4px; cursor: pointer; color: var(--testo); }
  .firma .pick img { height: 28px; background: #fff; border-radius: 3px; }
  .firma .del { background: transparent; border: none; color: var(--errore); cursor: pointer; font-size: 16px; }
  .lista .cap { font-size: 12px; font-weight: 600; }
  .voce { display: flex; justify-content: space-between; gap: 6px; font-size: 11px; align-items: center; }
  .voce button { background: transparent; border: none; color: var(--errore); cursor: pointer; font-size: 14px; }
  .salva { background: var(--accento); color: #fff; border: none; border-radius: 8px; padding: 9px; cursor: pointer; font-size: 14px; margin-top: 4px; }
  .salva:disabled { opacity: 0.5; }
  .ok { color: #7ad08f; font-size: 12px; }
  .errm { color: var(--errore); font-size: 12px; }
  .link { background: none; border: none; color: var(--accento); text-decoration: underline; cursor: pointer; padding: 0; font-size: 12px; }

  .tela { flex: 1; display: flex; flex-direction: column; min-width: 0; background: #525659; }
  .coord { padding: 6px 12px; font-size: 12px; color: #ddd; background: var(--barra); border-bottom: 1px solid var(--bordo); font-family: ui-monospace, monospace; }
  .scroll { flex: 1; overflow: auto; padding: 16px; }
  .griglia { display: grid; grid-template-rows: 24px auto; }
  .angolo { background: var(--barra); border-right: 1px solid var(--bordo); border-bottom: 1px solid var(--bordo); }
  .rig-h { position: relative; height: 24px; background: var(--barra); border-bottom: 1px solid var(--bordo); }
  .rig-v { position: relative; width: 24px; background: var(--barra); border-right: 1px solid var(--bordo); }
  .rig-h .t { position: absolute; top: 4px; font-size: 9px; color: var(--testo-soft); transform: translateX(-50%); border-left: 1px solid var(--bordo); padding-left: 1px; }
  .rig-v .t { position: absolute; left: 3px; font-size: 9px; color: var(--testo-soft); transform: translateY(-50%); }
  .pagina { position: relative; background: #fff; cursor: crosshair; box-shadow: 0 0 0 1px rgba(0,0,0,0.3); }
  .pagina img { display: block; width: 100%; height: 100%; user-select: none; }
  .marker { position: absolute; transform: translate(-2px, -2px); background: var(--accento); color: #fff; font-size: 11px; min-width: 16px; height: 16px; border-radius: 3px; display: flex; align-items: center; justify-content: center; pointer-events: none; }
  .campo-box { position: absolute; border: 1.5px dashed #1e6fd8; background: rgba(30,111,216,0.12); color: #1e6fd8; font-size: 10px; padding: 1px 3px; box-sizing: border-box; pointer-events: none; overflow: hidden; }
  .red-box { position: absolute; background: rgba(0,0,0,0.82); border: 1px solid #c81e1e; box-sizing: border-box; pointer-events: none; }
  .salva.rosso { background: #c81e1e; }
</style>
