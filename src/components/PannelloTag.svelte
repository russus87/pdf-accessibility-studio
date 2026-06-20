<script>
  // Pannello struttura/tag: naviga, modifica i ruoli, riordina l'ordine di
  // lettura ed esporta in JSON/XML. Le modifiche salvano una copia corretta.
  import { schede } from "../lib/schede.svelte.js";
  import { alberoTag, salvaTag, salvaDoclang, correggi, riordina, riquadroTag } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);
  let info = $state(null);
  let caricamento = $state(false);
  let errore = $state(null);
  let esito = $state(null);
  let modo = $state("naviga"); // "naviga" | "ruoli" | "riordina"

  // Modifiche in sospeso.
  let ruoliMod = $state({}); // riferimento -> nuovo ruolo
  let ordineTop = $state([]); // elementi di primo livello riordinabili

  const RUOLI = ["P","H1","H2","H3","H4","H5","H6","Figure","Table","TR","TH","TD","L","LI","Span","Link","Caption","Note"];

  $effect(() => {
    if (!s) return;
    const id = s.id;
    let annullato = false;
    caricamento = true;
    errore = null;
    info = null;
    ruoliMod = {};
    alberoTag(id)
      .then((r) => {
        if (annullato) return;
        info = r;
        ordineTop = r.radice
          .filter((n) => n.riferimento)
          .map((n) => ({ riferimento: n.riferimento, ruolo: n.ruolo }));
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
      acc.push({ prof, ruolo: n.ruolo, alt: n.alt, lang: n.lang, pagina: n.pagina, riferimento: n.riferimento });
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

  async function salvaOrdine() {
    esito = null;
    try {
      const dest = await riordina(s.id, ordineTop.map((e) => e.riferimento));
      if (dest) esito = "Copia con nuovo ordine di lettura salvata.";
    } catch (e) {
      esito = `Errore: ${e}`;
    }
  }
</script>

<div class="pannello">
  <header>
    <h3>Struttura / Tag</h3>
    {#if info}
      <div class="modi">
        <button class:on={modo === "naviga"} onclick={() => (modo = "naviga")}>Naviga</button>
        <button class:on={modo === "ruoli"} onclick={() => (modo = "ruoli")}>Ruoli</button>
        <button class:on={modo === "riordina"} onclick={() => (modo = "riordina")} disabled={ordineTop.length < 2}>Riordina</button>
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

    {:else if modo === "riordina"}
      <p class="suggerimento">Riordina i blocchi di primo livello: cambia l'ordine di lettura logico.</p>
      <ul class="editor">
        {#each ordineTop as e, i}
          <li class="riordina">
            <span class="ruolo">{e.ruolo}</span>
            <span class="frecce">
              <button onclick={() => muovi(i, -1)} disabled={i === 0} aria-label="Su">▲</button>
              <button onclick={() => muovi(i, 1)} disabled={i === ordineTop.length - 1} aria-label="Giù">▼</button>
            </span>
          </li>
        {/each}
      </ul>
      <button class="salva" onclick={salvaOrdine}>Salva copia riordinata…</button>
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
    justify-content: space-between;
  }
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
  .pag {
    margin-left: auto;
    color: var(--testo-soft);
    font-size: 11px;
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
