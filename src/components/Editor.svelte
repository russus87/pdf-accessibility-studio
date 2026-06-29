<script>
  // Editor visuale su PDF esistente (Fasi 23-26): canvas della pagina con righelli
  // in millimetri, posizionamento a clic di testo / immagini / firme / campi, e
  // filigrana su tutte le pagine. Salva un unico PDF (overlay + campi).
  import { schede } from "../lib/schede.svelte.js";
  import {
    renderPagina, dimensioniPagina, salvaEditor,
    firmeElenco, firmaSalva, firmaElimina, redigi, annota,
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
  let annotazioni = $state([]); // evidenziazioni / note
  let ann = $state({ larghezza_mm: 50, altezza_mm: 6, contenuto: "", colore: "#ffe53c" });
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
  const pxPerPt = $derived(pxPerMm * MM_PER_PT); // anteprima testo a grandezza reale
  const pageHpx = $derived(pageHmm * pxPerMm);

  // Opacità: la UI usa percentuali (0-100), il backend PDF vuole 0-255.
  const pctA255 = (p) => Math.round((Math.min(100, Math.max(0, p ?? 100)) / 100) * 255);

  // Modifica del testo direttamente sulla pagina: al termine dell'editing
  // inline scrive il contenuto del nodo nell'elemento corrispondente.
  function fineModifica(x, e) {
    const t = e.currentTarget.textContent.replace(/\s+$/, "");
    x.testo = t.length ? t : "Testo";
    elementi = [...elementi];
    registra();
  }

  // Trascinamento degli oggetti già posizionati. `dragStato` tiene l'oggetto
  // mosso e lo scarto (in mm) fra il puntatore e la sua origine, così l'oggetto
  // segue il mouse senza “saltare”.
  let dragStato = null;
  let dragMosso = false;
  function inizioDrag(e, item, lista) {
    e.stopPropagation();
    e.preventDefault();
    const { x_mm, y_mm } = mmDaEvento(e);
    dragStato = { item, lista, dxMm: x_mm - item.x_mm, dyMm: y_mm - item.y_mm };
    dragMosso = false;
    e.currentTarget.setPointerCapture?.(e.pointerId);
  }
  function muoviDrag(e) {
    if (!dragStato) return;
    const { x_mm, y_mm } = mmDaEvento(e);
    const desX = Math.max(0, x_mm - dragStato.dxMm);
    const desY = Math.max(0, y_mm - dragStato.dyMm);
    const { x, y, gx, gy } = applicaSnap(desX, desY, dragStato.item);
    dragStato.item.x_mm = Math.round(x * 10) / 10;
    dragStato.item.y_mm = Math.round(y * 10) / 10;
    guida = { x: gx, y: gy };
    dragMosso = true;
    rinfresca(dragStato.lista);
  }
  function fineDrag() {
    if (dragMosso) registra();
    dragStato = null;
    dragMosso = false;
    guida = { x: null, y: null };
  }
  function rinfresca(lista) {
    if (lista === "el") elementi = [...elementi];
    else if (lista === "campo") campi = [...campi];
    else if (lista === "red") redazioni = [...redazioni];
    else annotazioni = [...annotazioni];
  }

  // --- Snap e guide di allineamento ---
  // Griglia opzionale + aggancio ai bordi/centro pagina e agli altri oggetti.
  const PASSO_GRIGLIA = 5; // mm
  let griglia = $state(false);
  let guida = $state({ x: null, y: null }); // linee guida attive (in mm)

  function bersagliSnap(item) {
    const xs = [0, pageWmm / 2, pageWmm];
    const ys = [0, pageHmm / 2, pageHmm];
    const tutti = [...elementi, ...campi, ...redazioni, ...annotazioni];
    for (const o of tutti) {
      if (o === item || o.pagina !== pagina) continue;
      xs.push(o.x_mm);
      ys.push(o.y_mm);
      if (o.larghezza_mm) { xs.push(o.x_mm + o.larghezza_mm, o.x_mm + o.larghezza_mm / 2); }
      if (o.altezza_mm) { ys.push(o.y_mm + o.altezza_mm, o.y_mm + o.altezza_mm / 2); }
    }
    return { xs, ys };
  }
  function applicaSnap(xMm, yMm, item) {
    const soglia = 8 / pxPerMm; // ~8 px sullo schermo, indipendente dallo zoom
    let x = xMm, y = yMm, gx = null, gy = null;
    if (griglia) {
      x = Math.round(x / PASSO_GRIGLIA) * PASSO_GRIGLIA;
      y = Math.round(y / PASSO_GRIGLIA) * PASSO_GRIGLIA;
    }
    const { xs, ys } = bersagliSnap(item);
    let bx = soglia;
    for (const t of xs) { const d = Math.abs(xMm - t); if (d < bx) { bx = d; x = t; gx = t; } }
    let by = soglia;
    for (const t of ys) { const d = Math.abs(yMm - t); if (d < by) { by = d; y = t; gy = t; } }
    return { x: Math.max(0, x), y: Math.max(0, y), gx, gy };
  }

  // --- Ridimensionamento (immagini/firme) dalla maniglia d'angolo ---
  let resizeStato = null;
  function inizioRidim(e, item) {
    e.stopPropagation();
    e.preventDefault();
    resizeStato = {
      item, startW: item.larghezza_mm ?? 40, startH: item.altezza_mm ?? 20,
      startX: e.clientX, startY: e.clientY,
      aspect: (item.larghezza_mm ?? 40) / (item.altezza_mm ?? 20),
    };
    e.currentTarget.setPointerCapture?.(e.pointerId);
  }
  function muoviRidim(e) {
    if (!resizeStato) return;
    const dW = (e.clientX - resizeStato.startX) / pxPerMm;
    const dH = (e.clientY - resizeStato.startY) / pxPerMm;
    const w = Math.max(5, resizeStato.startW + dW);
    const h = e.shiftKey ? w / resizeStato.aspect : Math.max(5, resizeStato.startH + dH);
    resizeStato.item.larghezza_mm = Math.round(w * 10) / 10;
    resizeStato.item.altezza_mm = Math.round(h * 10) / 10;
    elementi = [...elementi];
  }
  function fineRidim() {
    if (resizeStato) registra();
    resizeStato = null;
  }

  // --- Cronologia: annulla/ripeti (Ctrl+Z / Ctrl+Shift+Z) ---
  // Ogni voce è uno snapshot JSON dei quattro insiemi di oggetti posizionati.
  const MAX_CRONO = 100;
  function istantanea() {
    return JSON.stringify({ elementi, campi, redazioni, annotazioni });
  }
  let cronologia = $state([istantanea()]);
  let cronoIdx = $state(0);
  const puoAnnullare = $derived(cronoIdx > 0);
  const puoRipetere = $derived(cronoIdx < cronologia.length - 1);

  function registra() {
    const snap = istantanea();
    if (snap === cronologia[cronoIdx]) return; // nessun cambiamento reale
    cronologia = [...cronologia.slice(0, cronoIdx + 1), snap].slice(-MAX_CRONO);
    cronoIdx = cronologia.length - 1;
  }
  function ripristina(snap) {
    const o = JSON.parse(snap);
    elementi = o.elementi;
    campi = o.campi;
    redazioni = o.redazioni;
    annotazioni = o.annotazioni;
  }
  function annulla() {
    if (!puoAnnullare) return;
    cronoIdx -= 1;
    ripristina(cronologia[cronoIdx]);
  }
  function ripeti() {
    if (!puoRipetere) return;
    cronoIdx += 1;
    ripristina(cronologia[cronoIdx]);
  }

  // Scorciatoie da tastiera, ignorate mentre si scrive in un campo (lì vale
  // l'annulla nativo del testo).
  $effect(() => {
    function onKey(e) {
      const t = e.target;
      if (t && (t.isContentEditable || t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.tagName === "SELECT")) return;
      if (!(e.ctrlKey || e.metaKey)) return;
      const k = e.key.toLowerCase();
      if (k === "z" && !e.shiftKey) { e.preventDefault(); annulla(); }
      else if ((k === "z" && e.shiftKey) || k === "y") { e.preventDefault(); ripeti(); }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

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
    // Posiziona un nuovo elemento solo cliccando sullo sfondo della pagina,
    // non sopra un oggetto già presente (che invece si sposta).
    if (e.target !== areaEl && !e.target.classList?.contains("pag-img")) return;
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
    } else if (strumento === "evidenzia") {
      annotazioni.push({ id: prossimoId++, tipo: "evidenzia", pagina, x_mm, y_mm, larghezza_mm: ann.larghezza_mm, altezza_mm: ann.altezza_mm, colore: ann.colore });
    } else if (strumento === "nota") {
      annotazioni.push({ id: prossimoId++, tipo: "nota", pagina, x_mm, y_mm, contenuto: ann.contenuto || "Nota", colore: "#ffd100" });
    }
    elementi = [...elementi];
    campi = [...campi];
    redazioni = [...redazioni];
    annotazioni = [...annotazioni];
    registra();
  }

  function rimuovi(lista, id) {
    if (lista === "el") elementi = elementi.filter((x) => x.id !== id);
    else if (lista === "campo") campi = campi.filter((x) => x.id !== id);
    else if (lista === "red") redazioni = redazioni.filter((x) => x.id !== id);
    else annotazioni = annotazioni.filter((x) => x.id !== id);
    registra();
  }

  async function salvaAnnotazioni() {
    if (!s || !annotazioni.length) { esito = { ok: false, msg: "Aggiungi un'annotazione." }; return; }
    esito = null;
    try {
      const a = annotazioni.map((x) => ({ tipo: x.tipo, pagina: x.pagina, x_mm: x.x_mm, y_mm: x.y_mm,
        larghezza_mm: x.larghezza_mm ?? null, altezza_mm: x.altezza_mm ?? null, contenuto: x.contenuto ?? null, dim_pt: null, colore: x.colore ?? null }));
      const r = await annota(s.id, a);
      if (r) esito = { ok: true, dest: r.dest, n: r.n };
    } catch (e) {
      esito = { ok: false, msg: String(e) };
    }
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
        opacita: pctA255(x.opacita), rotazione: x.rotazione ?? 0,
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
          ? { tipo: "immagine", png_base64: filiga.png_base64, larghezza_mm: filiga.larghezza_mm, altezza_mm: filiga.altezza_mm, opacita: pctA255(filiga.opacita), rotazione: filiga.rotazione }
          : { tipo: "testo", testo: filiga.testo, dim_pt: filiga.dim_pt, colore: filiga.colore, opacita: pctA255(filiga.opacita), rotazione: filiga.rotazione };
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
  const annotazioniPagina = $derived(annotazioni.filter((x) => x.pagina === pagina));
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

      <div class="crono">
        <button onclick={annulla} disabled={!puoAnnullare} title="Annulla (Ctrl+Z)">↶ Annulla</button>
        <button onclick={ripeti} disabled={!puoRipetere} title="Ripeti (Ctrl+Shift+Z)">↷ Ripeti</button>
      </div>
      <label class="check check-grid"><input type="checkbox" bind:checked={griglia} /> Griglia {PASSO_GRIGLIA} mm (snap)</label>

      <div class="strumenti">
        {#each [["testo", "Testo"], ["immagine", "Immagine"], ["firma", "Firma"], ["campo", "Campo"], ["evidenzia", "Evidenzia"], ["nota", "Nota"], ["redazione", "Redazione"]] as [k, lab]}
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
      {:else if strumento === "evidenzia"}
        <div class="cfg">
          <div class="riga">
            <label>L.mm<input type="number" min="2" bind:value={ann.larghezza_mm} /></label>
            <label>A.mm<input type="number" min="2" bind:value={ann.altezza_mm} /></label>
            <label>Colore<input type="color" bind:value={ann.colore} /></label>
          </div>
          <p class="hint">Clicca per evidenziare un'area.</p>
          {#if annotazioni.length}<button class="salva" onclick={salvaAnnotazioni}>Salva annotazioni ({annotazioni.length})…</button>{/if}
        </div>
      {:else if strumento === "nota"}
        <div class="cfg">
          <label>Testo nota<input type="text" bind:value={ann.contenuto} placeholder="Contenuto della nota" /></label>
          <p class="hint">Clicca per inserire una nota a fumetto.</p>
          {#if annotazioni.length}<button class="salva" onclick={salvaAnnotazioni}>Salva annotazioni ({annotazioni.length})…</button>{/if}
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
            onmousemove={suMovimento} onclick={suClic}
            onpointerup={fineDrag} onpointerleave={fineDrag}>
            {#if imgUrl}<img class="pag-img" src={imgUrl} alt={`Pagina ${pagina + 1}`} draggable="false" />{/if}
            {#if griglia}<div class="grid-overlay" style={`background-size:${PASSO_GRIGLIA * pxPerMm}px ${PASSO_GRIGLIA * pxPerMm}px`}></div>{/if}
            {#if guida.x != null}<div class="guida v" style={`left:${guida.x * pxPerMm}px`}></div>{/if}
            {#if guida.y != null}<div class="guida h" style={`top:${guida.y * pxPerMm}px`}></div>{/if}
            {#each elementiPagina as x (x.id)}
              {#if x.tipo === "testo"}
                <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
                <div class="el-wrap" style={`left:${x.x_mm * pxPerMm}px; top:${x.y_mm * pxPerMm}px;`}>
                  <div
                    class="maniglia"
                    title="Trascina per spostare"
                    onpointerdown={(e) => inizioDrag(e, x, 'el')}
                    onpointermove={muoviDrag}
                    onpointerup={fineDrag}
                    onpointercancel={fineDrag}
                    onclick={(e) => e.stopPropagation()}
                  >✥</div>
                  <div
                    class="txt-el"
                    style={`font-size:${x.dim_pt * pxPerPt}px; color:${x.colore}; opacity:${(x.opacita ?? 100) / 100};`}
                    contenteditable="true"
                    spellcheck="false"
                    role="textbox"
                    tabindex="-1"
                    title="Doppio clic / scrivi per modificare il testo"
                    onpointerdown={(e) => e.stopPropagation()}
                    onclick={(e) => e.stopPropagation()}
                    onblur={(e) => fineModifica(x, e)}
                    onkeydown={(e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); e.currentTarget.blur(); } }}
                  >{x.testo}</div>
                </div>
              {:else if x.png_base64}
                <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
                <div class="el-wrap" style={`left:${x.x_mm * pxPerMm}px; top:${x.y_mm * pxPerMm}px;`}>
                  <img
                    class="img-el"
                    src={`data:image/png;base64,${x.png_base64}`}
                    alt={x.nomeFirma ?? 'immagine'}
                    draggable="false"
                    title="Trascina per spostare"
                    style={`width:${(x.larghezza_mm ?? 40) * pxPerMm}px; height:${(x.altezza_mm ?? 20) * pxPerMm}px; opacity:${(x.opacita ?? 100) / 100};`}
                    onpointerdown={(e) => inizioDrag(e, x, 'el')}
                    onpointermove={muoviDrag}
                    onpointerup={fineDrag}
                    onpointercancel={fineDrag}
                    onclick={(e) => e.stopPropagation()}
                  />
                  <div class="ridim" title="Trascina per ridimensionare (Shift = proporzioni)"
                    onpointerdown={(e) => inizioRidim(e, x)} onpointermove={muoviRidim}
                    onpointerup={fineRidim} onpointercancel={fineRidim} onclick={(e) => e.stopPropagation()}></div>
                </div>
              {:else}
                <div class="marker" style={`left:${x.x_mm * pxPerMm}px; top:${x.y_mm * pxPerMm}px;`} title={x.tipo}>🖼</div>
              {/if}
            {/each}
            {#each campiPagina as c (c.id)}
              <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
              <div class="campo-box" title="Trascina per spostare"
                style={`left:${c.x_mm * pxPerMm}px; top:${c.y_mm * pxPerMm}px; width:${c.larghezza_mm * pxPerMm}px; height:${c.altezza_mm * pxPerMm}px;`}
                onpointerdown={(e) => inizioDrag(e, c, 'campo')} onpointermove={muoviDrag}
                onpointerup={fineDrag} onpointercancel={fineDrag} onclick={(e) => e.stopPropagation()}>{c.nome}</div>
            {/each}
            {#each redazioniPagina as a (a.id)}
              <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
              <div class="red-box" title="Trascina per spostare"
                style={`left:${a.x_mm * pxPerMm}px; top:${a.y_mm * pxPerMm}px; width:${a.larghezza_mm * pxPerMm}px; height:${a.altezza_mm * pxPerMm}px;`}
                onpointerdown={(e) => inizioDrag(e, a, 'red')} onpointermove={muoviDrag}
                onpointerup={fineDrag} onpointercancel={fineDrag} onclick={(e) => e.stopPropagation()}></div>
            {/each}
            {#each annotazioniPagina as a (a.id)}
              {#if a.tipo === "evidenzia"}
                <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
                <div class="evid-box" title="Trascina per spostare"
                  style={`left:${a.x_mm * pxPerMm}px; top:${a.y_mm * pxPerMm}px; width:${a.larghezza_mm * pxPerMm}px; height:${a.altezza_mm * pxPerMm}px; background:${a.colore};`}
                  onpointerdown={(e) => inizioDrag(e, a, 'ann')} onpointermove={muoviDrag}
                  onpointerup={fineDrag} onpointercancel={fineDrag} onclick={(e) => e.stopPropagation()}></div>
              {:else}
                <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
                <div class="nota-pin" title={a.contenuto}
                  style={`left:${a.x_mm * pxPerMm}px; top:${a.y_mm * pxPerMm}px;`}
                  onpointerdown={(e) => inizioDrag(e, a, 'ann')} onpointermove={muoviDrag}
                  onpointerup={fineDrag} onpointercancel={fineDrag} onclick={(e) => e.stopPropagation()}>📝</div>
              {/if}
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
  .crono { display: flex; gap: 6px; }
  .crono button { flex: 1; background: var(--scheda); border: 1px solid var(--bordo); color: var(--testo); border-radius: 6px; padding: 5px 8px; cursor: pointer; font-size: 12px; }
  .crono button:disabled { opacity: 0.4; cursor: default; }
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
  /* Testo a grandezza reale, modificabile sulla pagina */
  /* Contenitore di un testo posizionato (porta la maniglia di spostamento) */
  .el-wrap { position: absolute; }
  .txt-el {
    line-height: 1; white-space: pre; font-family: Helvetica, Arial, sans-serif;
    cursor: text; outline: none; padding: 0;
  }
  .txt-el:hover { outline: 1px dashed rgba(30,111,216,0.6); outline-offset: 1px; }
  .txt-el:focus { outline: 1px solid var(--accento); outline-offset: 1px; background: rgba(30,111,216,0.06); }
  /* Maniglia di trascinamento (appare al passaggio del mouse) */
  .maniglia {
    position: absolute; left: -10px; top: -10px; width: 18px; height: 18px;
    display: flex; align-items: center; justify-content: center;
    background: var(--accento); color: #fff; font-size: 11px; line-height: 1;
    border-radius: 50%; cursor: move; touch-action: none; opacity: 0;
    box-shadow: 0 1px 4px rgba(0,0,0,0.4); transition: opacity 0.12s; z-index: 2;
  }
  .el-wrap:hover .maniglia, .maniglia:active { opacity: 1; }
  /* Anteprima reale di immagini e firme (trascinabile) */
  .img-el { position: absolute; object-fit: fill; cursor: move; touch-action: none; }
  /* Maniglia d'angolo per ridimensionare immagini/firme */
  .ridim {
    position: absolute; right: -7px; bottom: -7px; width: 14px; height: 14px;
    background: var(--accento); border: 2px solid #fff; border-radius: 3px;
    cursor: nwse-resize; touch-action: none; opacity: 0; transition: opacity 0.12s; z-index: 2;
    box-shadow: 0 1px 4px rgba(0,0,0,0.4);
  }
  .el-wrap:hover .ridim, .ridim:active { opacity: 1; }
  /* Linee guida di allineamento durante il trascinamento */
  .guida { position: absolute; pointer-events: none; z-index: 3; }
  .guida.v { top: 0; bottom: 0; width: 0; border-left: 1px dashed #e91e63; }
  .guida.h { left: 0; right: 0; height: 0; border-top: 1px dashed #e91e63; }
  /* Griglia di sfondo (snap) */
  .grid-overlay {
    position: absolute; inset: 0; pointer-events: none; z-index: 1;
    background-image:
      linear-gradient(to right, rgba(30,111,216,0.18) 1px, transparent 1px),
      linear-gradient(to bottom, rgba(30,111,216,0.18) 1px, transparent 1px);
  }
  .check-grid { font-size: 12px; }
  .campo-box { position: absolute; border: 1.5px dashed #1e6fd8; background: rgba(30,111,216,0.12); color: #1e6fd8; font-size: 10px; padding: 1px 3px; box-sizing: border-box; cursor: move; touch-action: none; overflow: hidden; }
  .red-box { position: absolute; background: rgba(0,0,0,0.82); border: 1px solid #c81e1e; box-sizing: border-box; cursor: move; touch-action: none; }
  .salva.rosso { background: #c81e1e; }
  .evid-box { position: absolute; opacity: 0.4; box-sizing: border-box; cursor: move; touch-action: none; }
  .nota-pin { position: absolute; transform: translate(-2px, -50%); font-size: 14px; cursor: move; touch-action: none; }
</style>
