<script>
  // Editor a "foglio unico": una sola pagina in scala reale con tre regioni
  // modificabili in posizione — Intestazione, Corpo, Piè di pagina — più i
  // margini disegnati e le bande ridimensionabili col mouse. Una sola barra
  // strumenti agisce sulla regione che ha la selezione. Tutto vive in un <iframe>
  // isolato; i frammenti delle regioni sono bindabili. Nessuna dipendenza esterna.
  import Icona from "./Icona.svelte";

  let {
    header = $bindable(""),
    body = $bindable(""),
    footer = $bindable(""),
    stile = "",
    opzioni,
    variabili = [],
  } = $props();

  const PX_PER_MM = 96 / 25.4;

  let modo = $state("visuale"); // "visuale" | "codice"
  let iframe = $state(null);
  let regioneAttiva = "body"; // ultima regione con selezione/fuoco
  let rangeSalvato = null;
  let popover = $state(null);
  let testoPop = $state("");
  let scrittoIn = null;
  // Ultimi frammenti scritti/letti: distingue le modifiche esterne (es. caricamento
  // esempio o cambio scheda) da quelle interne (digitazione), evitando di
  // riscrivere il documento — e perdere il cursore — a ogni tasto.
  let ultimo = { h: null, b: null, f: null };

  const cw = $derived(opzioni.larghezzaMm - opzioni.margineSx - opzioni.margineDx);

  function frammenti() {
    return (
      `<div data-reg="header" class="reg reg-h">${header || ""}</div>` +
      `<div data-reg="body" class="reg reg-b">${body || ""}</div>` +
      `<div data-reg="footer" class="reg reg-f">${footer || ""}</div>`
    );
  }
  function docHtml() {
    return (
      `<!doctype html><html><head><meta charset="utf-8"><style>${stile}</style></head>` +
      `<body>${frammenti()}</body></html>`
    );
  }

  // (Ri)costruisce il documento al primo montaggio e quando i frammenti cambiano
  // dall'esterno (non per le modifiche interne, marcate in `ultimo`).
  $effect(() => {
    const h = header, b = body, f = footer;
    if (modo !== "visuale" || !iframe) return;
    if (iframe === scrittoIn && h === ultimo.h && b === ultimo.b && f === ultimo.f) return;
    scriviDoc();
  });
  // Riallinea il layout (posizioni regioni, margini, bande) quando cambiano le opzioni.
  $effect(() => {
    void [opzioni.larghezzaMm, opzioni.altezzaMm, opzioni.margineAlto, opzioni.margineBasso, opzioni.margineSx, opzioni.margineDx, opzioni.headerMm, opzioni.footerMm];
    if (modo === "visuale") aggiornaUI(iframe?.contentDocument);
  });

  function scriviDoc() {
    const doc = iframe.contentDocument;
    if (!doc) return;
    doc.open();
    doc.write(docHtml());
    doc.close();
    for (const r of doc.querySelectorAll("[data-reg]")) {
      r.contentEditable = "true";
      r.spellcheck = false;
    }
    // stile UI (layout + decorazioni + maniglie), marcato data-ui = non salvato.
    const s = doc.createElement("style");
    s.setAttribute("data-ui", "1");
    doc.head.appendChild(s);
    // margine guida + bande trascinabili
    for (const cls of ["pa-margini", "band-h", "band-f"]) {
      const d = doc.createElement("div");
      d.dataset.ui = "1";
      d.className = cls;
      d.contentEditable = "false";
      doc.body.appendChild(d);
    }
    doc.addEventListener("input", onInput);
    doc.addEventListener("selectionchange", salvaSel);
    collega(doc);
    attaccaHandle(doc);
    aggiornaUI(doc);
    scrittoIn = iframe;
    ultimo = { h: header, b: body, f: footer };
  }

  function aggiornaUI(doc) {
    if (!doc) return;
    const s = doc.querySelector('style[data-ui="1"]');
    if (!s) return;
    const o = opzioni;
    const w = cw;
    const hOn = o.headerMm > 0;
    const fOn = o.footerMm > 0;
    const padTop = o.margineAlto + (hOn ? o.headerMm : 0);
    const padBot = o.margineBasso + (fOn ? o.footerMm : 0);
    s.textContent =
      `html{margin:0;padding:0;background:#fff}` +
      `body{position:relative;margin:0;padding:0;width:${o.larghezzaMm}mm;min-height:${o.altezzaMm}mm;box-sizing:border-box}` +
      `.reg{outline:none}` +
      `.reg-h{position:absolute;left:${o.margineSx}mm;top:${o.margineAlto}mm;width:${w}mm;height:${o.headerMm}mm;overflow:hidden;outline:1px dashed #4a90d9;background:rgba(74,144,217,.05);${hOn ? "" : "display:none"}}` +
      `.reg-f{position:absolute;left:${o.margineSx}mm;top:${o.altezzaMm - o.margineBasso - o.footerMm}mm;width:${w}mm;height:${o.footerMm}mm;overflow:hidden;outline:1px dashed #4a90d9;background:rgba(74,144,217,.05);${fOn ? "" : "display:none"}}` +
      `.reg-b{padding:${padTop}mm ${o.margineDx}mm ${padBot}mm ${o.margineSx}mm;min-height:${o.altezzaMm}mm;box-sizing:border-box}` +
      `.reg-h:empty::before{content:'Intestazione';color:#9bb;font:11px sans-serif}` +
      `.reg-f:empty::before{content:'Piè di pagina';color:#9bb;font:11px sans-serif}` +
      `.pa-margini{position:absolute;left:${o.margineSx}mm;top:${o.margineAlto}mm;width:${w}mm;height:${Math.max(0, o.altezzaMm - o.margineAlto - o.margineBasso)}mm;border:1px dashed #b06;opacity:.4;pointer-events:none;z-index:1}` +
      `.band-h{position:absolute;left:${o.margineSx}mm;top:${o.margineAlto + o.headerMm}mm;width:${w}mm;height:8px;transform:translateY(-50%);cursor:ns-resize;z-index:8;${hOn ? "" : "display:none"}}` +
      `.band-f{position:absolute;left:${o.margineSx}mm;top:${o.altezzaMm - o.margineBasso - o.footerMm}mm;width:${w}mm;height:8px;transform:translateY(-50%);cursor:ns-resize;z-index:8;${fOn ? "" : "display:none"}}` +
      `.band-h::after,.band-f::after{content:'';position:absolute;left:0;right:0;top:50%;height:3px;transform:translateY(-50%);background:#4a90d9;border-radius:2px;opacity:.6}` +
      `.pa-canvas{outline:1px dashed #4a90d9;position:absolute}` +
      `.pa-ui{position:absolute;z-index:9;box-sizing:border-box;user-select:none}` +
      `.pa-move{left:0;right:0;top:0;height:14px;background:rgba(74,144,217,.55);cursor:move}` +
      `.pa-resize{right:0;bottom:0;width:14px;height:14px;background:#4a90d9;cursor:nwse-resize}` +
      `.pa-badge{left:0;top:-18px;height:16px;line-height:16px;padding:0 6px;font:11px sans-serif;color:#fff;background:#2563eb;border-radius:3px;white-space:nowrap}`;
  }

  function onInput() {
    const doc = iframe?.contentDocument;
    if (!doc) return;
    header = pulito(doc.querySelector('[data-reg="header"]'));
    body = pulito(doc.querySelector('[data-reg="body"]'));
    footer = pulito(doc.querySelector('[data-reg="footer"]'));
    ultimo = { h: header, b: body, f: footer };
  }
  function pulito(reg) {
    if (!reg) return "";
    const c = reg.cloneNode(true);
    c.querySelectorAll("[data-ui]").forEach((e) => e.remove());
    return c.innerHTML;
  }

  // --- selezione / regione attiva ---
  function salvaSel() {
    const doc = iframe?.contentDocument;
    const win = iframe?.contentWindow;
    const sel = win?.getSelection?.();
    if (!sel || !sel.rangeCount) return;
    const nodo = sel.anchorNode;
    const reg = nodo && (nodo.nodeType === 1 ? nodo : nodo.parentElement)?.closest?.("[data-reg]");
    if (reg) {
      regioneAttiva = reg.dataset.reg;
      if (reg.contains(sel.anchorNode)) rangeSalvato = sel.getRangeAt(0).cloneRange();
    }
    void doc;
  }
  function regBody() {
    return iframe?.contentDocument?.querySelector('[data-reg="body"]');
  }
  function regAttiva() {
    return iframe?.contentDocument?.querySelector(`[data-reg="${regioneAttiva}"]`) || regBody();
  }
  function ripristina() {
    const win = iframe?.contentWindow;
    const reg = regAttiva();
    reg?.focus?.();
    if (rangeSalvato && win) {
      const sel = win.getSelection();
      sel.removeAllRanges();
      sel.addRange(rangeSalvato);
    }
  }
  function esegui(comando, arg = null) {
    const doc = iframe?.contentDocument;
    if (!doc) return;
    ripristina();
    doc.execCommand(comando, false, arg);
    onInput();
  }
  function blocco(e) {
    const tag = e.currentTarget.value;
    if (tag) esegui("formatBlock", tag);
    e.currentTarget.selectedIndex = 0;
  }
  function inserisci(html) {
    esegui("insertHTML", html);
  }
  function tieniFuoco(e) {
    e.preventDefault();
  }

  // --- canvas trascinabile (sempre nella regione corpo, ancorato alla pagina) ---
  function inserisciCanvas() {
    const doc = iframe?.contentDocument;
    const b = regBody();
    if (!doc || !b) return;
    const box = doc.createElement("div");
    box.className = "pa-canvas";
    box.setAttribute("style", "position:absolute;left:20mm;top:40mm;width:60mm;height:35mm;padding:3mm 4mm;border:1px solid #888;background:#fff");
    box.innerHTML = "<p>Canvas: testo, immagini, tabelle. Trascina la barra blu, ridimensiona dall'angolo.</p>";
    b.appendChild(box);
    attaccaHandle(doc);
    onInput();
  }
  function attaccaHandle(doc) {
    doc.querySelectorAll(".pa-canvas").forEach((box) => {
      if (box.querySelector('[data-ui="badge"]')) return;
      box.style.position = box.style.position || "absolute";
      const badge = doc.createElement("div");
      badge.dataset.ui = "badge";
      badge.className = "pa-ui pa-badge";
      badge.contentEditable = "false";
      badge.textContent = dimBadge(box);
      box.appendChild(badge);
      for (const t of ["move", "resize"]) {
        const h = doc.createElement("div");
        h.dataset.ui = t;
        h.className = "pa-ui pa-" + t;
        h.contentEditable = "false";
        box.appendChild(h);
      }
    });
  }
  function dimBadge(box) {
    const w = Math.round(box.offsetWidth / PX_PER_MM);
    const h = Math.round(box.offsetHeight / PX_PER_MM);
    const l = Math.round(parseFloat(box.style.left) || box.offsetLeft / PX_PER_MM);
    const t = Math.round(parseFloat(box.style.top) || box.offsetTop / PX_PER_MM);
    return `${l},${t} · ${w}×${h} mm`;
  }

  function collega(doc) {
    let drag = null;
    const mm = (px) => Math.round((px / PX_PER_MM) * 10) / 10;
    doc.addEventListener("pointerdown", (e) => {
      const t = e.target;
      // bande
      if (t.classList?.contains("band-h") || t.classList?.contains("band-f")) {
        e.preventDefault();
        drag = { tipo: "banda", quale: t.classList.contains("band-h") ? "header" : "footer", y: e.clientY, start: t.classList.contains("band-h") ? opzioni.headerMm : opzioni.footerMm };
        t.setPointerCapture?.(e.pointerId);
        return;
      }
      // canvas
      const h = t.closest?.("[data-ui]");
      const box = h?.closest?.(".pa-canvas");
      if (h && box && h.dataset.ui !== "badge") {
        e.preventDefault();
        drag = {
          tipo: h.dataset.ui, box,
          x: e.clientX, y: e.clientY,
          l: parseFloat(box.style.left) || box.offsetLeft / PX_PER_MM,
          t: parseFloat(box.style.top) || box.offsetTop / PX_PER_MM,
          w: parseFloat(box.style.width) || box.offsetWidth / PX_PER_MM,
          hh: parseFloat(box.style.height) || box.offsetHeight / PX_PER_MM,
        };
        h.setPointerCapture?.(e.pointerId);
      }
    });
    doc.addEventListener("pointermove", (e) => {
      if (!drag) return;
      if (drag.tipo === "banda") {
        let d = mm(e.clientY - drag.y);
        let v = Math.max(0, Math.min(Math.round(opzioni.altezzaMm * 0.4), Math.round(drag.start + (drag.quale === "header" ? d : -d))));
        if (drag.quale === "header") opzioni.headerMm = v;
        else opzioni.footerMm = v;
        return;
      }
      const dx = mm(e.clientX - drag.x);
      const dy = mm(e.clientY - drag.y);
      if (drag.tipo === "move") {
        drag.box.style.left = Math.max(0, drag.l + dx) + "mm";
        drag.box.style.top = Math.max(0, drag.t + dy) + "mm";
      } else {
        drag.box.style.width = Math.max(10, drag.w + dx) + "mm";
        drag.box.style.height = Math.max(8, drag.hh + dy) + "mm";
      }
      const badge = drag.box.querySelector('[data-ui="badge"]');
      if (badge) badge.textContent = dimBadge(drag.box);
    });
    doc.addEventListener("pointerup", () => {
      if (drag) {
        const era = drag.tipo;
        drag = null;
        if (era !== "banda") onInput();
      }
    });
  }

  // --- popover (variabile / link / immagine / colonne / ciclo) ---
  function apriPop(tipo) {
    const cfg = {
      variabile: { etichetta: "Nome variabile", ph: "es. cliente, totale" },
      ciclo: { etichetta: "Nome elenco da ripetere", ph: "es. voci" },
      link: { etichetta: "Indirizzo del link", ph: "https://…" },
      immagine: { etichetta: "URL immagine", ph: "https://… o data:" },
      colonne: { etichetta: "Quante colonne? (2–4)", ph: "es. 2" },
    }[tipo];
    popover = { tipo, ...cfg };
    testoPop = "";
  }
  function confermaPop() {
    const t = testoPop.trim();
    const tipo = popover?.tipo;
    if (t) {
      if (tipo === "variabile") inserisci(`{{${t}}}`);
      else if (tipo === "ciclo") inserisci(`{{#each ${t}}}\n  <p>{{.}}</p>\n{{/each}}`);
      else if (tipo === "link") esegui("createLink", t);
      else if (tipo === "immagine") esegui("insertImage", t);
      else if (tipo === "colonne") {
        const n = Math.max(2, Math.min(4, parseInt(t, 10) || 2));
        inserisci(`<div style="column-count:${n};column-gap:10mm;text-align:justify"><p>Testo su ${n} colonne…</p></div><p><br></p>`);
      }
    }
    popover = null;
    testoPop = "";
  }
  function tastoPop(e) {
    if (e.key === "Enter") { e.preventDefault(); confermaPop(); }
    else if (e.key === "Escape") popover = null;
  }
  function inserisciTabella() {
    inserisci('<table border="1" cellpadding="6" style="border-collapse:collapse;width:100%"><thead><tr><th>Col 1</th><th>Col 2</th></tr></thead><tbody><tr><td>&nbsp;</td><td>&nbsp;</td></tr></tbody></table><p><br></p>');
  }

  function abilitaBanda(quale) {
    if (quale === "header") opzioni.headerMm = opzioni.headerMm > 0 ? 0 : 18;
    else opzioni.footerMm = opzioni.footerMm > 0 ? 0 : 14;
  }
</script>

<div class="foglio-editor">
  <div class="toolbar" role="toolbar" aria-label="Formattazione">
    <div class="modi">
      <button class:on={modo === "visuale"} onclick={() => (modo = "visuale")} title="Foglio"><Icona nome="eye" dim={15} /><span>Foglio</span></button>
      <button class:on={modo === "codice"} onclick={() => { onInput(); modo = "codice"; }} title="Codice HTML"><Icona nome="code" dim={15} /><span>Codice</span></button>
    </div>

    {#if modo === "visuale"}
      <span class="div"></span>
      <select class="blocco" onmousedown={tieniFuoco} onchange={blocco} title="Stile paragrafo" aria-label="Stile paragrafo">
        <option value="">Stile…</option>
        <option value="p">Paragrafo</option>
        <option value="h1">Titolo 1</option>
        <option value="h2">Titolo 2</option>
        <option value="h3">Titolo 3</option>
      </select>
      <span class="div"></span>
      <button onmousedown={tieniFuoco} onclick={() => esegui("bold")} title="Grassetto"><Icona nome="bold" dim={16} /></button>
      <button onmousedown={tieniFuoco} onclick={() => esegui("italic")} title="Corsivo"><Icona nome="italic" dim={16} /></button>
      <button onmousedown={tieniFuoco} onclick={() => esegui("underline")} title="Sottolineato"><Icona nome="underline" dim={16} /></button>
      <span class="div"></span>
      <button onmousedown={tieniFuoco} onclick={() => esegui("insertUnorderedList")} title="Elenco puntato"><Icona nome="list" dim={16} /></button>
      <button onmousedown={tieniFuoco} onclick={() => esegui("insertOrderedList")} title="Elenco numerato"><Icona nome="list-ordered" dim={16} /></button>
      <span class="div"></span>
      <button onmousedown={tieniFuoco} onclick={() => apriPop("link")} title="Link"><Icona nome="link" dim={16} /></button>
      <button onmousedown={tieniFuoco} onclick={() => apriPop("immagine")} title="Immagine"><Icona nome="image" dim={16} /></button>
      <button onmousedown={tieniFuoco} onclick={inserisciTabella} title="Tabella"><Icona nome="table" dim={16} /></button>
      <button onmousedown={tieniFuoco} onclick={() => apriPop("colonne")} title="Colonne"><Icona nome="colonne" dim={16} /></button>
      <button class="testuale" onmousedown={tieniFuoco} onclick={inserisciCanvas} title="Canvas trascinabile"><Icona nome="panel-left" dim={15} /><span>Canvas</span></button>
      <span class="div"></span>
      <button class="testuale" onmousedown={tieniFuoco} onclick={() => inserisci("{{PAGENUM}}")} title="Numero pagina">N° pag.</button>
      <button class="testuale" onmousedown={tieniFuoco} onclick={() => inserisci("{{TTLPAGES}}")} title="Totale pagine">Tot.</button>
      <button class="testuale" onmousedown={tieniFuoco} onclick={() => apriPop("variabile")} title={"Variabile {{…}}"}><Icona nome="braces" dim={15} /><span>Variabile</span></button>
      <button class="testuale" onmousedown={tieniFuoco} onclick={() => apriPop("ciclo")} title={"Ripeti {{#each}}"}><Icona nome="repeat" dim={15} /><span>Ripeti</span></button>

      <span class="spazio"></span>
      <button class="banda-btn" class:on={opzioni.headerMm > 0} onclick={() => abilitaBanda("header")} title="Attiva/disattiva intestazione">Intest.</button>
      <button class="banda-btn" class:on={opzioni.footerMm > 0} onclick={() => abilitaBanda("footer")} title="Attiva/disattiva piè di pagina">Piè</button>
    {/if}
  </div>

  {#if modo === "visuale" && variabili.length}
    <div class="chips">
      <span class="chip-eti">Variabili:</span>
      {#each variabili as v}
        <button class="chip" onmousedown={tieniFuoco} onclick={() => inserisci(`{{${v}}}`)}>{`{{${v}}}`}</button>
      {/each}
    </div>
  {/if}

  {#if popover}
    <div class="popover">
      <label for="fpop">{popover.etichetta}</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input id="fpop" bind:value={testoPop} placeholder={popover.ph} onkeydown={tastoPop} autofocus />
      <button class="ok" onclick={confermaPop}>Inserisci</button>
      <button class="annulla" onclick={() => (popover = null)}>Annulla</button>
    </div>
  {/if}

  {#if modo === "visuale"}
    <div class="tela-wrap">
      <iframe bind:this={iframe} title="Foglio del documento" style="width:{opzioni.larghezzaMm}mm;height:{opzioni.altezzaMm}mm"></iframe>
    </div>
  {:else}
    <div class="codice">
      <label>Intestazione<textarea bind:value={header} spellcheck="false" placeholder="HTML dell'intestazione"></textarea></label>
      <label>Corpo<textarea class="grande" bind:value={body} spellcheck="false" placeholder="HTML del corpo"></textarea></label>
      <label>Piè di pagina<textarea bind:value={footer} spellcheck="false" placeholder="HTML del piè di pagina"></textarea></label>
    </div>
  {/if}
</div>

<style>
  .foglio-editor { flex: 1; display: flex; flex-direction: column; min-height: 0; min-width: 0; }
  .toolbar { display: flex; flex-wrap: wrap; align-items: center; gap: 3px; padding: 7px 10px; background: var(--barra); border-bottom: 1px solid var(--bordo); }
  .toolbar button { display: inline-flex; align-items: center; gap: 5px; height: 30px; min-width: 30px; justify-content: center; padding: 0 7px; background: transparent; color: var(--testo-soft); border: 1px solid transparent; border-radius: var(--r-1); cursor: pointer; font-size: 12px; }
  .toolbar button:hover { background: var(--hover); color: var(--testo); }
  .toolbar button.on { background: var(--accento-tenue); color: var(--accento-testo); }
  .modi { display: flex; gap: 2px; padding: 2px; background: var(--scheda); border-radius: var(--r-2); }
  .modi button { height: 26px; border-radius: var(--r-1); }
  .blocco { height: 30px; background: var(--scheda); color: var(--testo); border: 1px solid var(--bordo); border-radius: var(--r-1); padding: 0 6px; font-size: 12px; cursor: pointer; }
  .div { width: 1px; height: 20px; background: var(--bordo); margin: 0 4px; }
  .spazio { flex: 1; }
  .banda-btn { border: 1px solid var(--bordo) !important; }
  .banda-btn.on { border-color: var(--accento) !important; }
  .chips { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; padding: 7px 10px; background: var(--sfondo); border-bottom: 1px solid var(--bordo-soft); }
  .chip-eti { font-size: 11px; color: var(--testo-soft); text-transform: uppercase; letter-spacing: .04em; }
  .chip { background: var(--scheda); color: var(--accento-testo); border: 1px solid var(--bordo); border-radius: 999px; padding: 3px 10px; font-size: 12px; font-family: ui-monospace, monospace; cursor: pointer; }
  .chip:hover { border-color: var(--accento); }
  .popover { display: flex; align-items: center; gap: 8px; padding: 9px 12px; background: var(--scheda); border-bottom: 1px solid var(--bordo); }
  .popover label { font-size: 12px; color: var(--testo-soft); }
  .popover input { flex: 1; max-width: 320px; background: var(--sfondo); color: var(--testo); border: 1px solid var(--bordo); border-radius: var(--r-1); padding: 6px 9px; font-size: 13px; }
  .popover .ok { background: var(--accento); color: #1b1206; border: none; border-radius: var(--r-1); padding: 6px 12px; font-weight: 600; cursor: pointer; }
  .popover .annulla { background: transparent; color: var(--testo-soft); border: 1px solid var(--bordo); border-radius: var(--r-1); padding: 6px 12px; cursor: pointer; }
  .tela-wrap { flex: 1; overflow: auto; background: #e9eaed; padding: 18px; display: flex; justify-content: center; align-items: flex-start; min-height: 0; }
  iframe { flex: none; border: none; background: #fff; box-shadow: var(--ombra-pop); }
  .codice { flex: 1; display: flex; flex-direction: column; gap: 10px; padding: 12px; overflow: auto; }
  .codice label { display: flex; flex-direction: column; gap: 4px; font-size: 11px; text-transform: uppercase; letter-spacing: .04em; color: var(--testo-soft); }
  .codice textarea { resize: vertical; min-height: 70px; border: 1px solid var(--bordo); border-radius: var(--r-2); padding: 10px 12px; background: var(--tela); color: var(--testo); font-family: ui-monospace, "Cascadia Code", monospace; font-size: 13px; line-height: 1.6; tab-size: 2; }
  .codice textarea.grande { min-height: 220px; }
</style>
