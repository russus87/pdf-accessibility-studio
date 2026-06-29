<script>
  // Pannello struttura/tag: naviga, modifica i ruoli, riordina l'ordine di
  // lettura ed esporta in JSON/XML. Le modifiche salvano una copia corretta.
  import { schede } from "../lib/schede.svelte.js";
  import { alberoTag, salvaTag, salvaDoclang, correggi, riordina, riquadroTag, marcaArtifact, applicaTabella, suggerisciAlt, statoAi } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);
  let info = $state(null);
  let caricamento = $state(false);
  let errore = $state(null);
  let esito = $state(null);
  let modo = $state("naviga"); // "naviga" | "ruoli" | "riordina" | "artifact" | "tabelle" | "alt"

  // Modifiche in sospeso.
  let ruoliMod = $state({}); // riferimento -> nuovo ruolo
  let ordineTop = $state([]); // elementi di primo livello riordinabili
  let artifactSel = $state({}); // riferimento -> selezionato per artifact
  let tabMod = $state({}); // riferimento -> { scope?, rowSpan?, colSpan? } modifiche celle

  // Assistente Alt-text (AI): testo per figura + stato di generazione.
  let altMod = $state({}); // riferimento -> testo alternativo
  let altGen = $state({}); // riferimento -> in generazione
  let altBatch = $state(false); // generazione in blocco in corso
  let aiHaChiave = $state(false);
  $effect(() => {
    statoAi().then((st) => (aiHaChiave = !!st?.ha_chiave)).catch(() => {});
  });
  const figure = $derived(info ? righe(info.radice).filter((r) => r.ruolo === "Figure" && r.riferimento) : []);

  // Modalità richiesta dall'esterno (flusso guidato dalla validazione).
  $effect(() => {
    if (schede.pannello === "tag" && schede.pannelloModo) {
      modo = schede.pannelloModo;
      schede.pannelloModo = null;
    }
  });

  const RUOLI = ["P","H1","H2","H3","H4","H5","H6","Figure","Table","TR","TH","TD","L","LI","Span","Link","Caption","Note"];

  $effect(() => {
    if (!s) return;
    const id = s.id;
    let annullato = false;
    caricamento = true;
    errore = null;
    info = null;
    ruoliMod = {};
    artifactSel = {};
    tabMod = {};
    altMod = {};
    alberoTag(id)
      .then((r) => {
        if (annullato) return;
        info = r;
        ordineTop = r.radice
          .filter((n) => n.riferimento)
          .map((n) => ({ riferimento: n.riferimento, ruolo: n.ruolo, pagina: n.pagina }));
      })
      .catch((e) => !annullato && (errore = String(e)))
      .finally(() => !annullato && (caricamento = false));
    return () => (annullato = true);
  });

  // Salta all'elemento: ne evidenzia il riquadro sul PDF (best-effort) e ci porta.
  async function vaiAElemento(r) {
    if (r.riferimento) {
      try {
        const rt = await riquadroTag(s.id, r.riferimento);
        if (rt) {
          schede.evidenzia(rt.pagina, rt.rettangoli, "tag");
          return;
        }
      } catch (_) {
        // ripiega sul semplice salto di pagina
      }
    }
    if (r.pagina != null) schede.vaiAPagina(r.pagina);
  }

  function righe(nodi, prof = 0, acc = []) {
    for (const n of nodi) {
      acc.push({
        prof, ruolo: n.ruolo, alt: n.alt, lang: n.lang, pagina: n.pagina, riferimento: n.riferimento,
        scope: n.scope, rowSpan: n.row_span, colSpan: n.col_span,
      });
      righe(n.figli, prof + 1, acc);
    }
    return acc;
  }

  async function esporta(formato) {
    esito = null;
    try {
      const ok = await salvaTag(s.id, formato);
      if (ok) esito = `Esportato in ${formato.toUpperCase()}.`;
    } catch (e) {
      esito = `Errore: ${e}`;
    }
  }

  async function esportaDoclang() {
    esito = null;
    try {
      const ok = await salvaDoclang(s.id);
      if (ok) esito = "Esportato in DocLang (DocTags).";
    } catch (e) {
      esito = `Errore: ${e}`;
    }
  }

  async function salvaRuoli() {
    esito = null;
    const ruoli = Object.entries(ruoliMod)
      .filter(([, r]) => r)
      .map(([riferimento, ruolo]) => ({ riferimento, ruolo }));
    if (!ruoli.length) {
      esito = "Nessuna modifica di ruolo.";
      return;
    }
    try {
      const dest = await correggi(s.id, { ruoli });
      if (dest) esito = "Copia con ruoli aggiornati salvata.";
    } catch (e) {
      esito = `Errore: ${e}`;
    }
  }

  function muovi(i, delta) {
    const j = i + delta;
    if (j < 0 || j >= ordineTop.length) return;
    const a = ordineTop;
    [a[i], a[j]] = [a[j], a[i]];
    ordineTop = [...a];
  }

  // Riordino per trascinamento.
  let trascIdx = $state(null);
  function dragRilascia(i) {
    if (trascIdx == null || trascIdx === i) { trascIdx = null; return; }
    const a = [...ordineTop];
    const [el] = a.splice(trascIdx, 1);
    a.splice(i, 0, el);
    ordineTop = a;
    trascIdx = null;
  }

  async function salvaOrdine() {
    esito = null;
    try {
      const dest = await riordina(s.id, ordineTop.map((e) => e.riferimento));
      if (dest) esito = "Copia con nuovo ordine di lettura salvata.";
    } catch (e) {
      esito = `Errore: ${e}`;
    }
  }

  // Registra una modifica a una cella (scope / rowSpan / colSpan).
  function modCella(rif, campo, valore) {
    tabMod[rif] = { ...(tabMod[rif] || {}), [campo]: valore };
  }

  async function salvaTabelle() {
    esito = null;
    const celle = Object.entries(tabMod)
      .map(([riferimento, m]) => ({ riferimento, ...m }))
      .filter((c) => c.scope !== undefined || c.rowSpan !== undefined || c.colSpan !== undefined);
    if (!celle.length) {
      esito = "Nessuna modifica alle celle.";
      return;
    }
    try {
      const r = await applicaTabella(s.id, celle);
      if (r) esito = `Copia salvata: ${r.n} cell${r.n === 1 ? "a" : "e"} aggiornat${r.n === 1 ? "a" : "e"}.`;
    } catch (e) {
      esito = `Errore: ${e}`;
    }
  }

  async function salvaArtifact() {
    esito = null;
    const rif = Object.entries(artifactSel)
      .filter(([, on]) => on)
      .map(([riferimento]) => riferimento);
    if (!rif.length) {
      esito = "Seleziona almeno un elemento da marcare come Artifact.";
      return;
    }
    try {
      const r = await marcaArtifact(s.id, rif);
      if (r) esito = `Copia salvata: ${r.n} element${r.n === 1 ? "o" : "i"} marcat${r.n === 1 ? "o" : "i"} come Artifact.`;
    } catch (e) {
      esito = `Errore: ${e}`;
    }
  }

  // Testo alternativo corrente di una figura (modifica in sospeso o valore già nel PDF).
  const altCorrente = (r) => altMod[r.riferimento] ?? r.alt ?? "";

  // Genera l'alt di una figura con Claude (vision sulla pagina).
  async function generaAlt(r) {
    if (r.pagina == null) { esito = "Pagina sconosciuta per questa figura."; return; }
    altGen[r.riferimento] = true;
    try {
      const t = await suggerisciAlt(s.id, r.pagina);
      altMod[r.riferimento] = (t || "").trim();
    } catch (e) {
      esito = `Errore AI: ${e}`;
    } finally {
      altGen[r.riferimento] = false;
    }
  }

  // Genera in blocco l'alt per tutte le figure ancora prive di testo.
  async function generaTutte() {
    altBatch = true;
    esito = null;
    try {
      for (const r of figure) {
        if (altCorrente(r).trim() || r.pagina == null) continue;
        await generaAlt(r);
      }
    } finally {
      altBatch = false;
    }
  }

  async function salvaAlt() {
    esito = null;
    const alt = Object.entries(altMod)
      .filter(([, t]) => t != null && t.trim() !== "")
      .map(([riferimento, testo]) => ({ riferimento, testo: testo.trim() }));
    if (!alt.length) { esito = "Nessun testo alternativo da salvare."; return; }
    try {
      const dest = await correggi(s.id, { alt });
      if (dest) esito = `Copia salvata con ${alt.length} testo/i alternativo/i.`;
    } catch (e) {
      esito = `Errore: ${e}`;
    }
  }

  // Riallinea la gerarchia dei titoli: il primo titolo diventa H1 e nessun
  // titolo salta più di un livello rispetto al precedente. Precompila i cambi
  // di ruolo (da rivedere e salvare nella stessa schermata).
  function sistemaGerarchia() {
    esito = null;
    const heads = righe(info.radice).filter((r) => /^H[1-6]$/.test(r.ruolo) && r.riferimento);
    if (!heads.length) { esito = "Nessun titolo (H1–H6) trovato."; return; }
    let prev = 0, cambi = 0;
    for (const h of heads) {
      const liv = parseInt(h.ruolo.slice(1), 10);
      const target = Math.max(1, prev === 0 ? 1 : Math.min(liv, prev + 1));
      const nuovo = "H" + target;
      if (nuovo !== h.ruolo) { ruoliMod[h.riferimento] = nuovo; cambi++; }
      prev = target;
    }
    esito = cambi ? `${cambi} titolo/i riallineato/i: rivedi i ruoli e salva.` : "Gerarchia dei titoli già corretta.";
  }
</script>

<div class="pannello">
  <header>
    <h3>Struttura / Tag</h3>
    {#if info}
      <div class="modi">
        <button class:on={modo === "naviga"} onclick={() => (modo = "naviga")}>Naviga</button>
        <button class:on={modo === "ruoli"} onclick={() => (modo = "ruoli")}>Ruoli</button>
        <button class:on={modo === "alt"} onclick={() => (modo = "alt")} disabled={!figure.length}>Alt (AI)</button>
        <button class:on={modo === "riordina"} onclick={() => (modo = "riordina")} disabled={ordineTop.length < 2}>Riordina</button>
        <button class:on={modo === "artifact"} onclick={() => (modo = "artifact")}>Artifact</button>
        <button class:on={modo === "tabelle"} onclick={() => (modo = "tabelle")}>Tabelle</button>
      </div>
      <div class="azioni">
        <button onclick={() => esporta("json")}>Export JSON</button>
        <button onclick={() => esporta("xml")}>Export XML</button>
        {#if info.ha_struct_tree}
          <button onclick={esportaDoclang} title="Serializzazione DocTags/DocLang dallo StructTree">Export DocLang</button>
        {/if}
      </div>
    {/if}
  </header>

  {#if esito}<p class="esito">{esito}</p>{/if}

  {#if caricamento}
    <p class="info">Lettura tag…</p>
  {:else if errore}
    <p class="err">{errore}</p>
  {:else if info}
    {#if !info.ha_struct_tree}
      <p class="info">Questo PDF non ha un albero dei tag (non è taggato).</p>

    {:else if modo === "naviga"}
      <p class="suggerimento">Clicca un elemento per evidenziarlo sul PDF e saltarci.</p>
      <ul>
        {#each righe(info.radice) as r}
          <li>
            <button class="riga" style={`padding-left:${8 + r.prof * 16}px`} disabled={r.pagina == null && !r.riferimento} onclick={() => vaiAElemento(r)}>
              <span class="ruolo">{r.ruolo}</span>
              {#if r.alt}<span class="alt">alt: {r.alt}</span>{/if}
              {#if r.lang}<span class="lang">{r.lang}</span>{/if}
              {#if r.pagina != null}<span class="pag">p.{r.pagina + 1}</span>{/if}
            </button>
          </li>
        {/each}
      </ul>

    {:else if modo === "ruoli"}
      <p class="suggerimento">Cambia il ruolo di un elemento (es. P → H1, o segna una cella come TH).</p>
      <div class="azioni-alt">
        <button onclick={sistemaGerarchia} title="Primo titolo = H1 e nessun salto di livello">↧ Sistema gerarchia titoli</button>
      </div>
      <ul class="editor">
        {#each righe(info.radice).filter((r) => r.riferimento) as r}
          <li style={`padding-left:${8 + r.prof * 16}px`}>
            <select value={ruoliMod[r.riferimento] || r.ruolo} onchange={(e) => (ruoliMod[r.riferimento] = e.target.value)}>
              {#each RUOLI as opt}<option value={opt}>{opt}</option>{/each}
            </select>
            {#if (ruoliMod[r.riferimento] || r.ruolo) !== r.ruolo}<span class="cambiato">era {r.ruolo}</span>{/if}
            {#if r.pagina != null}<span class="pag">p.{r.pagina + 1}</span>{/if}
          </li>
        {/each}
      </ul>
      <button class="salva" onclick={salvaRuoli}>Salva copia con ruoli…</button>

    {:else if modo === "alt"}
      <p class="suggerimento">
        Testo alternativo per le <b>immagini</b> (Figure). Scrivilo a mano o generalo con
        l'AI (Claude descrive la pagina). Le figure senza alt sono un errore WCAG comune.
      </p>
      {#if !aiHaChiave}
        <p class="info">Suggerimento: imposta la chiave API nel pannello <b>AI</b> per la generazione automatica. Puoi comunque scrivere l'alt a mano.</p>
      {/if}
      <div class="azioni-alt">
        <button onclick={generaTutte} disabled={altBatch || !aiHaChiave} title="Genera l'alt per tutte le figure ancora vuote">
          {altBatch ? "Generazione…" : "🪄 Genera tutte le mancanti"}
        </button>
        <span class="conteggio">{figure.filter((r) => !altCorrente(r).trim()).length} senza alt / {figure.length}</span>
      </div>
      <ul class="editor">
        {#each figure as r}
          {@const vuoto = !altCorrente(r).trim()}
          <li class="alt-riga">
            <div class="alt-cap">
              <button class="vai" onclick={() => vaiAElemento(r)} title="Evidenzia sul PDF">{r.ruolo}{#if r.pagina != null} · p.{r.pagina + 1}{/if}</button>
              {#if vuoto}<span class="badge-manca">manca</span>{:else}<span class="badge-ok">ok</span>{/if}
              <button class="ai" onclick={() => generaAlt(r)} disabled={altGen[r.riferimento] || !aiHaChiave} title="Genera con AI">
                {altGen[r.riferimento] ? "…" : "🪄 AI"}
              </button>
            </div>
            <textarea rows="2" placeholder="Descrizione dell'immagine…" value={altCorrente(r)}
              oninput={(e) => (altMod[r.riferimento] = e.target.value)}></textarea>
          </li>
        {/each}
      </ul>
      <button class="salva" onclick={salvaAlt}>Salva copia con testi alternativi…</button>

    {:else if modo === "riordina"}
      <p class="suggerimento">Ordine di lettura logico dei blocchi di primo livello: <b>trascina</b> per riordinare (o usa ▲▼). Il numero è la posizione in lettura. Clicca il ruolo per evidenziare il blocco sul PDF.</p>
      <ul class="editor">
        {#each ordineTop as e, i (e.riferimento)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <li class="riordina" class:trasc={trascIdx === i}
            draggable="true"
            ondragstart={() => (trascIdx = i)}
            ondragover={(ev) => ev.preventDefault()}
            ondrop={() => dragRilascia(i)}
            ondragend={() => (trascIdx = null)}>
            <span class="ordn">{i + 1}</span>
            <button class="vai" onclick={() => vaiAElemento(e)} title="Evidenzia sul PDF">{e.ruolo}{#if e.pagina != null} · p.{e.pagina + 1}{/if}</button>
            <span class="frecce">
              <button onclick={() => muovi(i, -1)} disabled={i === 0} aria-label="Su">▲</button>
              <button onclick={() => muovi(i, 1)} disabled={i === ordineTop.length - 1} aria-label="Giù">▼</button>
            </span>
          </li>
        {/each}
      </ul>
      <button class="salva" onclick={salvaOrdine}>Salva copia riordinata…</button>

    {:else if modo === "artifact"}
      <p class="suggerimento">
        Marca come <b>Artifact</b> gli elementi decorativi (intestazioni, piè di pagina,
        numeri di pagina): verranno tolti dall'ordine di lettura e gli screen reader li salteranno.
      </p>
      <ul class="editor">
        {#each righe(info.radice).filter((r) => r.riferimento) as r}
          <li style={`padding-left:${8 + r.prof * 16}px`}>
            <label class="art">
              <input type="checkbox" checked={!!artifactSel[r.riferimento]} onchange={(e) => (artifactSel[r.riferimento] = e.target.checked)} />
              <span class="ruolo">{r.ruolo}</span>
              {#if r.alt}<span class="alt">alt: {r.alt}</span>{/if}
            </label>
            {#if r.pagina != null}<span class="pag">p.{r.pagina + 1}</span>{/if}
          </li>
        {/each}
      </ul>
      <button class="salva" onclick={salvaArtifact}>Salva copia con artifact…</button>

    {:else if modo === "tabelle"}
      {@const celle = righe(info.radice).filter((r) => (r.ruolo === "TH" || r.ruolo === "TD") && r.riferimento)}
      {#if !celle.length}
        <p class="info">Nessuna cella di tabella (TH/TD) trovata nei tag.</p>
      {:else}
        <p class="suggerimento">
          Imposta l'<b>ambito</b> delle intestazioni (Scope) e le celle unite (RowSpan/ColSpan).
          Lo Scope dice allo screen reader se un TH vale per la riga, la colonna o entrambe.
        </p>
        <ul class="editor">
          {#each celle as r}
            <li class="cella">
              <span class="ruolo">{r.ruolo}</span>
              {#if r.ruolo === "TH"}
                <label class="cmp">Scope
                  <select value={r.scope ?? ""} onchange={(e) => modCella(r.riferimento, "scope", e.target.value)}>
                    <option value="">(nessuno)</option>
                    <option value="Row">Riga</option>
                    <option value="Column">Colonna</option>
                    <option value="Both">Entrambe</option>
                  </select>
                </label>
              {/if}
              <label class="cmp">Righe
                <input type="number" min="1" value={r.rowSpan ?? 1} onchange={(e) => modCella(r.riferimento, "rowSpan", parseInt(e.target.value) || 1)} />
              </label>
              <label class="cmp">Col.
                <input type="number" min="1" value={r.colSpan ?? 1} onchange={(e) => modCella(r.riferimento, "colSpan", parseInt(e.target.value) || 1)} />
              </label>
              {#if r.pagina != null}<span class="pag">p.{r.pagina + 1}</span>{/if}
            </li>
          {/each}
        </ul>
        <button class="salva" onclick={salvaTabelle}>Salva copia tabelle accessibili…</button>
      {/if}
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
  .modi,
  .azioni {
    display: flex;
    gap: 6px;
  }
  .azioni {
    margin-top: 8px;
  }
  .modi button,
  .azioni button {
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 5px 10px;
    cursor: pointer;
    font-size: 12px;
  }
  .modi button.on {
    background: var(--accento);
    border-color: var(--accento);
    color: #fff;
  }
  .modi button:hover:not(.on):not(:disabled),
  .azioni button:hover {
    border-color: var(--accento);
  }
  .suggerimento {
    margin: 0;
    padding: 8px 14px 0;
    font-size: 12px;
    color: var(--testo-soft);
    font-style: italic;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 8px 0 16px;
    font-family: ui-monospace, monospace;
    font-size: 13px;
  }
  li {
    margin: 0;
  }
  ul.editor li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
  }
  li.riordina {
    cursor: grab;
    border: 1px solid transparent;
  }
  li.riordina:hover { background: var(--scheda); }
  li.riordina.trasc { opacity: 0.5; border-color: var(--accento); }
  .ordn {
    flex: none;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: 11px;
    font-weight: 700;
    background: var(--accento);
    color: #fff;
  }
  li.riordina .vai {
    flex: 1;
    text-align: left;
    background: transparent;
    border: none;
    color: var(--accento);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
    padding: 0;
  }
  li.riordina .vai:hover { text-decoration: underline; }
  li.riordina .frecce { margin-left: auto; }
  select {
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 3px 6px;
  }
  .frecce button {
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 4px;
    cursor: pointer;
    padding: 1px 7px;
  }
  .frecce button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  button.riga {
    display: flex;
    width: 100%;
    gap: 8px;
    align-items: baseline;
    background: transparent;
    border: none;
    color: var(--testo);
    font: inherit;
    text-align: left;
    padding: 3px 8px;
    cursor: pointer;
    border-radius: 4px;
  }
  button.riga:hover:not(:disabled) {
    background: var(--scheda);
  }
  button.riga:disabled {
    cursor: default;
  }
  .salva {
    margin: 4px 14px 16px;
    background: var(--accento);
    color: #fff;
    border: none;
    border-radius: 8px;
    padding: 9px 16px;
    cursor: pointer;
  }
  .ruolo {
    color: var(--accento);
    font-weight: 600;
  }
  .alt {
    color: var(--testo-soft);
  }
  .lang {
    color: #7ab0ff;
    font-size: 11px;
  }
  .cambiato {
    color: #e7c98a;
    font-size: 11px;
  }
  label.art {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }
  label.art input {
    accent-color: var(--accento);
    width: 15px;
    height: 15px;
  }
  li.cella {
    flex-wrap: wrap;
  }
  label.cmp {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--testo-soft);
  }
  label.cmp input {
    width: 48px;
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 3px 6px;
  }
  .pag {
    margin-left: auto;
    color: var(--testo-soft);
    font-size: 11px;
  }
  .azioni-alt {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 14px 0;
  }
  .azioni-alt button {
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 6px 10px;
    cursor: pointer;
    font-size: 12px;
  }
  .azioni-alt button:disabled { opacity: 0.5; cursor: default; }
  .conteggio { font-size: 11px; color: var(--testo-soft); }
  li.alt-riga {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--bordo);
  }
  .alt-cap { display: flex; align-items: center; gap: 8px; }
  .alt-cap .vai {
    background: transparent; border: none; color: var(--accento);
    font: inherit; font-weight: 600; cursor: pointer; padding: 0;
  }
  .alt-cap .vai:hover { text-decoration: underline; }
  .alt-cap .ai {
    margin-left: auto; background: var(--scheda); color: var(--testo);
    border: 1px solid var(--bordo); border-radius: 6px; padding: 3px 8px; cursor: pointer; font-size: 11px;
  }
  .alt-cap .ai:disabled { opacity: 0.5; cursor: default; }
  .badge-manca { font-size: 10px; color: #fff; background: var(--errore, #d14); border-radius: 4px; padding: 1px 6px; }
  .badge-ok { font-size: 10px; color: #062; background: #bdf0cb; border-radius: 4px; padding: 1px 6px; }
  li.alt-riga textarea {
    width: 100%; box-sizing: border-box; resize: vertical;
    background: var(--scheda); color: var(--testo);
    border: 1px solid var(--bordo); border-radius: 6px; padding: 6px 8px;
    font: inherit; font-size: 12px;
  }
  .info,
  .err,
  .esito {
    padding: 12px 14px;
    color: var(--testo-soft);
  }
  .err {
    color: var(--errore);
  }
  .esito {
    color: #7ad08f;
  }
</style>
