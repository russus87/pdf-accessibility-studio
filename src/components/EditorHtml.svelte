<script>
  // Editor HTML con due modalita': "visuale" (WYSIWYG) e "codice" (sorgente).
  // Il WYSIWYG vive in un <iframe> isolato: cosi' i modelli sono documenti HTML
  // completi (con <head>/<style>) senza che i loro stili "sporchino" l'app, e si
  // vede l'impaginazione reale. I segnaposto {{variabili}} e {{#each}} restano
  // testo letterale. Nessuna dipendenza esterna. `value` e' bindabile.
  import Icona from "./Icona.svelte";

  let {
    value = $bindable(""),
    variabili = [],
    compatto = false,
    numeriPagina = false,
    // Dimensioni del "foglio" mostrato nell'editor (mm) e margini interni (mm).
    // Servono a far coincidere proporzioni e posizioni col PDF finale.
    paginaW = 210,
    paginaH = 297,
    margini = [0, 0, 0, 0], // [alto, destra, basso, sinistra] in mm
  } = $props();

  // 1 mm = 96/25.4 px CSS (costante in tutti i browser): converte px↔mm.
  const PX_PER_MM = 96 / 25.4;

  let modo = $state("visuale"); // "visuale" | "codice"
  let iframe = $state(null);
  let rangeSalvato = null; // ultima selezione interna al documento dell'iframe
  let popover = $state(null); // { tipo, etichetta, ph } | null
  let testoPop = $state("");

  let scrittoIn = null; // iframe in cui abbiamo gia' scritto
  let ultimoValore = ""; // ultimo HTML scritto/letto dal documento

  const DOC_VUOTO = "<!doctype html><html><head><meta charset=\"utf-8\"></head><body></body></html>";

  // (Ri)scrive il documento dell'iframe quando il valore cambia dall'esterno
  // o quando l'iframe viene rimontato (ritorno dalla vista codice).
  $effect(() => {
    const v = value;
    if (modo !== "visuale" || !iframe) return;
    if (iframe === scrittoIn && v === ultimoValore) return;
    scriviDoc(v);
  });

  function scriviDoc(v) {
    const doc = iframe.contentDocument;
    if (!doc) return;
    doc.open();
    doc.write(v && v.trim() ? v : DOC_VUOTO);
    doc.close();
    doc.body.contentEditable = "true";
    doc.body.spellcheck = false;
    doc.body.style.minHeight = "100%";
    doc.body.style.outline = "none";
    doc.addEventListener("input", onInput);
    doc.addEventListener("selectionchange", salvaSel);
    iniettaAiuti(doc);
    attaccaHandle(doc);
    collegaCanvas(doc);
    scrittoIn = iframe;
    ultimoValore = v;
  }

  // Stili e maniglie di editing sono marcati [data-ui] e MAI salvati nel valore.
  function iniettaAiuti(doc) {
    if (!doc.querySelector('style[data-ui="aiuti"]')) {
      const s = doc.createElement("style");
      s.setAttribute("data-ui", "aiuti");
      s.textContent =
        ".pa-canvas{outline:1px dashed #4a90d9}" +
        ".pa-ui{position:absolute;z-index:9;box-sizing:border-box;user-select:none}" +
        ".pa-move{left:0;right:0;top:0;height:14px;background:rgba(74,144,217,.55);cursor:move}" +
        ".pa-resize{right:0;bottom:0;width:14px;height:14px;background:#4a90d9;cursor:nwse-resize}" +
        ".pa-badge{left:0;top:-18px;height:16px;line-height:16px;padding:0 6px;font:11px sans-serif;" +
        "color:#fff;background:#2563eb;border-radius:3px;white-space:nowrap}";
      (doc.head || doc.documentElement).appendChild(s);
    }
    if (!doc.querySelector('style[data-ui="foglio"]')) {
      const f = doc.createElement("style");
      f.setAttribute("data-ui", "foglio");
      (doc.head || doc.documentElement).appendChild(f);
    }
    if (!doc.querySelector('[data-ui="margini"]')) {
      const g = doc.createElement("div");
      g.dataset.ui = "margini";
      g.contentEditable = "false";
      doc.body.appendChild(g);
    }
    aggiornaFoglio(doc);
  }
  // Mostra il contenuto come un "foglio" largo quanto la pagina (origine in alto
  // a sinistra) così proporzioni e posizioni del canvas coincidono col PDF.
  function aggiornaFoglio(doc) {
    if (!doc) return;
    const f = doc.querySelector('style[data-ui="foglio"]');
    const [mt, mr, mb, ml] = margini;
    if (f) {
      f.textContent =
        "html{background:#e9eaed;margin:0;padding:0}" +
        `body{box-sizing:border-box!important;width:${paginaW}mm!important;min-height:${paginaH}mm!important;` +
        `margin:0!important;padding:${mt}mm ${mr}mm ${mb}mm ${ml}mm!important;background:#fff!important;` +
        "box-shadow:0 0 0 1px #c8ccd2}";
    }
    // Guida tratteggiata sui margini (solo se ce ne sono).
    const g = doc.querySelector('[data-ui="margini"]');
    if (g) {
      if (mt + mr + mb + ml > 0) {
        g.setAttribute(
          "style",
          `position:absolute;left:${ml}mm;top:${mt}mm;width:${Math.max(0, paginaW - ml - mr)}mm;` +
            `height:${Math.max(0, paginaH - mt - mb)}mm;border:1px dashed #b06; opacity:.45;pointer-events:none;z-index:1`,
        );
      } else {
        g.setAttribute("style", "display:none");
      }
    }
  }
  // Riaggiorna il foglio quando cambiano dimensioni pagina o margini.
  $effect(() => {
    void [paginaW, paginaH, margini[0], margini[1], margini[2], margini[3]];
    if (modo === "visuale") aggiornaFoglio(iframe?.contentDocument);
  });

  function attaccaHandle(doc) {
    doc.querySelectorAll(".pa-canvas").forEach((box) => {
      if (box.querySelector('[data-ui="move"]')) return;
      box.style.position = box.style.position || "absolute";
      const badge = doc.createElement("div");
      badge.dataset.ui = "badge";
      badge.className = "pa-ui pa-badge";
      badge.contentEditable = "false";
      badge.textContent = dimBadge(box);
      box.appendChild(badge);
      for (const tipo of ["move", "resize"]) {
        const h = doc.createElement("div");
        h.dataset.ui = tipo;
        h.className = "pa-ui pa-" + tipo;
        h.contentEditable = "false";
        box.appendChild(h);
      }
    });
  }
  function dimBadge(box) {
    const w = Math.round(box.offsetWidth / PX_PER_MM);
    const h = Math.round(box.offsetHeight / PX_PER_MM);
    const l = Math.round((parseFloat(box.style.left) || box.offsetLeft / PX_PER_MM));
    const t = Math.round((parseFloat(box.style.top) || box.offsetTop / PX_PER_MM));
    return `${l},${t} · ${w}×${h} mm`;
  }
  function collegaCanvas(doc) {
    let drag = null;
    const mm = (px) => Math.round((px / PX_PER_MM) * 10) / 10; // 1 decimale
    doc.addEventListener("pointerdown", (e) => {
      const h = e.target.closest?.("[data-ui]");
      const box = h?.closest?.(".pa-canvas");
      if (!h || !box || h.dataset.ui === "badge") return;
      e.preventDefault();
      drag = {
        tipo: h.dataset.ui,
        box,
        x: e.clientX,
        y: e.clientY,
        l: parseFloat(box.style.left) || box.offsetLeft / PX_PER_MM,
        t: parseFloat(box.style.top) || box.offsetTop / PX_PER_MM,
        w: parseFloat(box.style.width) || box.offsetWidth / PX_PER_MM,
        hh: parseFloat(box.style.height) || box.offsetHeight / PX_PER_MM,
      };
      h.setPointerCapture?.(e.pointerId);
    });
    doc.addEventListener("pointermove", (e) => {
      if (!drag) return;
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
        drag = null;
        onInput();
      }
    });
  }

  function onInput() {
    const doc = iframe?.contentDocument;
    if (!doc) return;
    // Serializza una copia senza le maniglie/stili di editing [data-ui].
    const clone = doc.documentElement.cloneNode(true);
    clone.querySelectorAll("[data-ui]").forEach((e) => e.remove());
    const v = "<!doctype html>\n" + clone.outerHTML;
    ultimoValore = v;
    value = v;
  }

  function salvaSel() {
    const win = iframe?.contentWindow;
    const doc = iframe?.contentDocument;
    if (!win || !doc) return;
    const sel = win.getSelection();
    if (sel && sel.rangeCount && doc.body.contains(sel.anchorNode)) {
      rangeSalvato = sel.getRangeAt(0).cloneRange();
    }
  }

  function ripristina() {
    const win = iframe?.contentWindow;
    win?.focus();
    iframe?.contentDocument?.body?.focus();
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

  // --- popover per i valori che richiedono input ---
  function apriPop(tipo) {
    const cfg = {
      variabile: { etichetta: "Nome variabile", ph: "es. cliente, totale" },
      ciclo: { etichetta: "Nome elenco da ripetere", ph: "es. voci, righe" },
      link: { etichetta: "Indirizzo del link", ph: "https://…" },
      immagine: { etichetta: "URL immagine", ph: "https://… oppure data:" },
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
        inserisci(
          `<div style="column-count:${n};column-gap:10mm;text-align:justify">\n  <p>Testo su ${n} colonne: sostituiscilo col tuo contenuto. Il testo fluisce automaticamente da una colonna all'altra, come in un giornale.</p>\n</div>\n<p><br></p>`,
        );
      }
    }
    popover = null;
    testoPop = "";
  }
  function tastoPop(e) {
    if (e.key === "Enter") {
      e.preventDefault();
      confermaPop();
    } else if (e.key === "Escape") {
      popover = null;
    }
  }

  function inserisciTabella() {
    inserisci(
      '<table border="1" cellpadding="6" style="border-collapse:collapse;width:100%"><thead><tr><th>Colonna 1</th><th>Colonna 2</th></tr></thead><tbody><tr><td>&nbsp;</td><td>&nbsp;</td></tr><tr><td>&nbsp;</td><td>&nbsp;</td></tr></tbody></table><p><br></p>',
    );
  }

  // Canvas: box a posizione libera, trascinabile (barra blu) e ridimensionabile
  // (angolo). Dentro si possono mettere testo, immagini e tabelle.
  function inserisciCanvas() {
    const doc = iframe?.contentDocument;
    if (!doc) return;
    const box = doc.createElement("div");
    box.className = "pa-canvas";
    box.setAttribute(
      "style",
      "position:absolute;left:20mm;top:20mm;width:60mm;height:35mm;padding:3mm 4mm;border:1px solid #888;background:#ffffff",
    );
    box.innerHTML = "<p>Canvas: scrivi qui. Puoi inserire testo, immagini e tabelle. Trascina la barra blu per spostarlo, l'angolo per ridimensionarlo.</p>";
    doc.body.appendChild(box);
    attaccaHandle(doc);
    onInput();
  }
  function inserisciNumeroPagina(token) {
    inserisci(token);
  }

  // Sommario generato dai titoli (h1–h3) del documento. Se ne esiste già uno
  // (marcato data-toc) lo rigenera; gli stili inline sono modificabili a mano.
  function inserisciSommario() {
    const doc = iframe?.contentDocument;
    if (!doc) return;
    const titoli = [...doc.body.querySelectorAll("h1,h2,h3")].filter((h) => !h.closest("[data-toc]"));
    if (!titoli.length) {
      popover = { tipo: "info", etichetta: "Aggiungi prima dei titoli (Titolo 1/2/3) al documento.", ph: "" };
      return;
    }
    let n = 0;
    const voci = titoli
      .map((h) => {
        if (!h.id) {
          n++;
          const slug = h.textContent.trim().toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "").slice(0, 28);
          h.id = `sez-${n}-${slug || n}`;
        }
        const liv = Number(h.tagName[1]);
        return `<li style="margin-left:${(liv - 1) * 16}px"><a href="#${h.id}" style="color:inherit;text-decoration:none">${h.textContent}</a></li>`;
      })
      .join("");
    const html = `<nav data-toc style="border:1px solid #ccc;border-radius:6px;padding:14px 18px;margin:0 0 18px;font-family:inherit;page-break-after:always"><div style="font-weight:700;font-size:1.15em;margin-bottom:8px">Sommario</div><ul style="list-style:none;padding:0;margin:0;line-height:1.9">${voci}</ul></nav>`;
    const esistente = doc.querySelector("[data-toc]");
    if (esistente) {
      esistente.outerHTML = html;
    } else {
      doc.body.insertAdjacentHTML("afterbegin", html);
    }
    onInput();
  }

  // evita che i bottoni della toolbar rubino la selezione all'iframe
  function tieniFuoco(e) {
    e.preventDefault();
  }
</script>

<div class="editor-html">
  <div class="toolbar" role="toolbar" aria-label="Formattazione">
    <div class="modi">
      <button class:on={modo === "visuale"} onclick={() => (modo = "visuale")} title="Editor visuale">
        <Icona nome="eye" dim={15} /><span>Visuale</span>
      </button>
      <button class:on={modo === "codice"} onclick={() => (modo = "codice")} title="Codice HTML">
        <Icona nome="code" dim={15} /><span>Codice</span>
      </button>
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
      <button onmousedown={tieniFuoco} onclick={() => apriPop("link")} title="Inserisci link"><Icona nome="link" dim={16} /></button>
      <button onmousedown={tieniFuoco} onclick={() => apriPop("immagine")} title="Inserisci immagine"><Icona nome="image" dim={16} /></button>
      {#if !compatto}
        <button onmousedown={tieniFuoco} onclick={inserisciTabella} title="Inserisci tabella"><Icona nome="table" dim={16} /></button>
      {/if}
      <span class="div"></span>
      <button class="testuale" onmousedown={tieniFuoco} onclick={inserisciCanvas} title="Inserisci canvas: box trascinabile con testo/immagini/tabelle">
        <Icona nome="panel-left" dim={15} /><span>Canvas</span>
      </button>
      {#if !compatto}
        <button onmousedown={tieniFuoco} onclick={() => apriPop("colonne")} title="Sezione a colonne (giornale)"><Icona nome="colonne" dim={16} /></button>
        <button onmousedown={tieniFuoco} onclick={inserisciSommario} title="Inserisci/aggiorna sommario dai titoli"><Icona nome="sommario" dim={16} /></button>
      {/if}
      <span class="div"></span>
      {#if numeriPagina}
        <button class="testuale" onmousedown={tieniFuoco} onclick={() => inserisciNumeroPagina("{{PAGENUM}}")} title="Numero pagina corrente">
          <Icona nome="file" dim={15} /><span>N° pag.</span>
        </button>
        <button class="testuale" onmousedown={tieniFuoco} onclick={() => inserisciNumeroPagina("{{TTLPAGES}}")} title="Numero totale di pagine">
          <Icona nome="sommario" dim={15} /><span>Tot.</span>
        </button>
      {/if}
      <button class="testuale" onmousedown={tieniFuoco} onclick={() => apriPop("variabile")} title={"Inserisci variabile {{…}}"}>
        <Icona nome="braces" dim={15} /><span>Variabile</span>
      </button>
      {#if !compatto}
        <button class="testuale" onmousedown={tieniFuoco} onclick={() => apriPop("ciclo")} title={"Inserisci ripetizione {{#each}}"}>
          <Icona nome="repeat" dim={15} /><span>Ripeti</span>
        </button>
      {/if}
    {/if}
  </div>

  {#if modo === "visuale" && variabili.length}
    <div class="chips" aria-label="Variabili disponibili">
      <span class="chip-eti">Variabili:</span>
      {#each variabili as v}
        <button class="chip" onmousedown={tieniFuoco} onclick={() => inserisci(`{{${v}}}`)}>{`{{${v}}}`}</button>
      {/each}
    </div>
  {/if}

  {#if popover}
    <div class="popover">
      <label for="pop-input">{popover.etichetta}</label>
      {#if popover.tipo === "info"}
        <button class="ok" onclick={() => (popover = null)}>Ho capito</button>
      {:else}
        <!-- svelte-ignore a11y_autofocus -->
        <input id="pop-input" bind:value={testoPop} placeholder={popover.ph} onkeydown={tastoPop} autofocus />
        <button class="ok" onclick={confermaPop}>Inserisci</button>
        <button class="annulla" onclick={() => (popover = null)}>Annulla</button>
      {/if}
    </div>
  {/if}

  <div class="area">
    {#if modo === "visuale"}
      <iframe bind:this={iframe} class="tela" title="Editor visuale del documento"></iframe>
    {:else}
      <textarea bind:value spellcheck="false" placeholder={"HTML del modello con {{variabili}} e {{#each elenco}}…{{/each}}"}></textarea>
    {/if}
  </div>
</div>

<style>
  .editor-html {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
  }
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 3px;
    padding: 7px 10px;
    background: var(--barra);
    border-bottom: 1px solid var(--bordo);
  }
  .toolbar button {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 30px;
    min-width: 30px;
    justify-content: center;
    padding: 0 7px;
    background: transparent;
    color: var(--testo-soft);
    border: 1px solid transparent;
    border-radius: var(--r-1);
    cursor: pointer;
    font-size: 12px;
    transition: background var(--transizione), color var(--transizione);
  }
  .toolbar button:hover {
    background: var(--hover);
    color: var(--testo);
  }
  .toolbar button.on {
    background: var(--accento-tenue);
    color: var(--accento-testo);
  }
  .toolbar button.testuale {
    font-weight: 500;
  }
  .modi {
    display: flex;
    gap: 2px;
    padding: 2px;
    background: var(--scheda);
    border-radius: var(--r-2);
  }
  .modi button {
    height: 26px;
    border-radius: var(--r-1);
  }
  .blocco {
    height: 30px;
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: var(--r-1);
    padding: 0 6px;
    font-size: 12px;
    cursor: pointer;
  }
  .div {
    width: 1px;
    height: 20px;
    background: var(--bordo);
    margin: 0 4px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 7px 10px;
    background: var(--sfondo);
    border-bottom: 1px solid var(--bordo-soft);
  }
  .chip-eti {
    font-size: 11px;
    color: var(--testo-soft);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .chip {
    background: var(--scheda);
    color: var(--accento-testo);
    border: 1px solid var(--bordo);
    border-radius: 999px;
    padding: 3px 10px;
    font-size: 12px;
    font-family: ui-monospace, monospace;
    cursor: pointer;
  }
  .chip:hover {
    border-color: var(--accento);
  }
  .popover {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 12px;
    background: var(--scheda);
    border-bottom: 1px solid var(--bordo);
  }
  .popover label {
    font-size: 12px;
    color: var(--testo-soft);
  }
  .popover input {
    flex: 1;
    max-width: 320px;
    background: var(--sfondo);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: var(--r-1);
    padding: 6px 9px;
    font-size: 13px;
  }
  .popover .ok {
    background: var(--accento);
    color: #1b1206;
    border: none;
    border-radius: var(--r-1);
    padding: 6px 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .popover .annulla {
    background: transparent;
    color: var(--testo-soft);
    border: 1px solid var(--bordo);
    border-radius: var(--r-1);
    padding: 6px 12px;
    cursor: pointer;
  }
  .area {
    flex: 1;
    display: flex;
    min-height: 0;
    background: #e9eaed;
    padding: 14px;
    overflow: auto;
  }
  .tela {
    flex: 1;
    width: 100%;
    border: none;
    background: #ffffff;
    border-radius: var(--r-2);
    box-shadow: var(--ombra);
  }
  textarea {
    flex: 1;
    resize: none;
    border: none;
    outline: none;
    padding: 16px 18px;
    background: var(--tela);
    color: var(--testo);
    font-family: ui-monospace, "Cascadia Code", monospace;
    font-size: 13px;
    line-height: 1.6;
    tab-size: 2;
    border-radius: var(--r-2);
  }
</style>
