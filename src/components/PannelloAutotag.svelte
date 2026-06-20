<script>
  // Auto-tagging assistito (Docling): SOLO per i PDF non taggati.
  // Inferisce la struttura semantica con la libreria Python Docling (sidecar) e
  // ne ricava proposte di tag PDF/UA, navigabili come l'albero dei tag.
  import { schede } from "../lib/schede.svelte.js";
  import { alberoTag, doclingStato, doclingPrepara, analizzaStrutturaAi, generaPdfAccessibile } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);

  let stato = $state(null); // stato del ponte Docling
  let taggato = $state(null); // il PDF ha gia' uno struct tree?
  let caricamento = $state(true);
  let errore = $state(null);

  let analisi = $state(null); // risultato di analizzaStrutturaAi
  let inCorso = $state(false);
  let esito = $state(null);
  let mostraDoclang = $state(false);
  let generando = $state(false);
  let pdfa3 = $state(false);
  let embedDoclang = $state(true);

  // Pronto a lanciare l'analisi.
  const pronto = $derived(
    !!stato && stato.python_disponibile && stato.docling_installato,
  );

  // Istruzioni di installazione per piattaforma.
  const ISTRUZIONI = {
    windows: {
      titolo: "Windows",
      passi: [
        "Installa Python 3 da python.org (oppure: winget install Python.Python.3). Spunta «Add Python to PATH» durante l'installazione.",
        "Apri PowerShell e installa Docling: pip install docling",
        "Riapri l'app (o premi «Ricontrolla»).",
      ],
    },
    macos: {
      titolo: "macOS",
      passi: [
        "Installa Python 3: brew install python (Homebrew) oppure scaricalo da python.org.",
        "Nel Terminale installa Docling: pip3 install docling",
        "Riapri l'app (o premi «Ricontrolla»).",
      ],
    },
    linux: {
      titolo: "Linux",
      passi: [
        "Assicurati di avere Python 3 (di solito già presente).",
        "Installa Docling: pipx install docling (consigliato) oppure pip install --user docling",
        "Premi «Ricontrolla».",
      ],
    },
  };
  const istruzioni = $derived(ISTRUZIONI[stato?.piattaforma] || ISTRUZIONI.linux);

  async function caricaStato() {
    caricamento = true;
    errore = null;
    try {
      stato = await doclingStato();
      if (s) {
        const info = await alberoTag(s.id);
        taggato = info.ha_struct_tree;
      }
    } catch (e) {
      errore = String(e);
    } finally {
      caricamento = false;
    }
  }

  $effect(() => {
    if (!s) return;
    analisi = null;
    esito = null;
    caricaStato();
  });

  async function analizza() {
    if (!s) return;
    inCorso = true;
    esito = null;
    errore = null;
    analisi = null;
    try {
      // Assicura lo script-ponte, poi analizza.
      await doclingPrepara();
      analisi = await analizzaStrutturaAi(s.id);
      esito = `${analisi.blocchi_totali} blocchi analizzati, ${analisi.proposte.length} proposte di tag.`;
    } catch (e) {
      errore = String(e);
    } finally {
      inCorso = false;
    }
  }

  // Porta alla pagina di una proposta nel visore.
  function vai(p) {
    if (p.pagina != null) schede.vaiAPagina(p.pagina);
  }

  // Genera la bozza di PDF taggato dalle proposte (opzione A).
  async function generaBozza() {
    if (!s || !analisi) return;
    generando = true;
    esito = null;
    errore = null;
    try {
      const r = await generaPdfAccessibile(s.id, analisi.proposte, analisi.lingua, s.titolo, {
        doclang: embedDoclang ? analisi.doclang : null,
        pdfa3,
      });
      if (r) esito = `PDF taggato salvato (${r.n} elementi): ${r.dest}`;
    } catch (e) {
      errore = String(e);
    } finally {
      generando = false;
    }
  }
</script>

<div class="pannello">
  <header>
    <h3>Auto-tag (AI)</h3>
    <p class="sub">Inferisce la struttura dei PDF <strong>senza tag</strong> con Docling.</p>
  </header>

  {#if caricamento}
    <p class="info">Verifica dell'ambiente…</p>

  {:else if errore}
    <p class="err">{errore}</p>
    <button class="secondaria" onclick={caricaStato}>Ricontrolla</button>

  {:else if taggato}
    <p class="info">
      Questo PDF è già taggato: ha un albero dei tag. L'auto-tag AI serve solo ai
      documenti <strong>senza</strong> struttura. Usa il pannello «Tag» per
      navigarli e correggerli.
    </p>

  {:else if !pronto}
    <!-- Componente da installare: guida l'utente. -->
    <div class="installa">
      <p class="info">
        Per l'auto-tag serve <strong>Docling</strong> (una libreria Python), da
        installare una volta sola sul computer.
      </p>
      <ul class="diag">
        <li>{stato.python_disponibile ? "✓" : "✗"} Python {stato.python_bin ? `(${stato.python_bin})` : ""}</li>
        <li>{stato.docling_installato ? "✓" : "✗"} Docling</li>
      </ul>
      <p class="passo-titolo">Installazione su {istruzioni.titolo}:</p>
      <ol class="passi">
        {#each istruzioni.passi as passo}
          <li>{passo}</li>
        {/each}
      </ol>
      <button class="secondaria" onclick={caricaStato}>Ricontrolla</button>
    </div>

  {:else}
    <div class="azione">
      <button class="salva" onclick={analizza} disabled={inCorso}>
        {inCorso ? "Analisi in corso…" : "Analizza struttura"}
      </button>
      {#if esito}<p class="esito">{esito}</p>{/if}
    </div>

    {#if inCorso}
      <div class="loader" role="status" aria-live="polite">
        <span class="spinner" aria-hidden="true"></span>
        <div class="loader-testo">
          <p>Analisi della struttura con Docling…</p>
          <p class="loader-nota">Al primo avvio può richiedere qualche minuto (scarica i modelli AI).</p>
        </div>
      </div>
    {/if}

    {#if analisi && !inCorso}
      {#if analisi.lingua}
        <p class="meta">Lingua rilevata: <strong>{analisi.lingua}</strong></p>
      {/if}

      <p class="suggerimento">
        Proposte di tag in ordine di lettura. Clicca per saltare alla pagina.
        Le figure senza descrizione sono da completare con un Alt (pannello «Correggi»).
      </p>
      <ul>
        {#each analisi.proposte as p}
          <li>
            <button class="riga" disabled={p.pagina == null} onclick={() => vai(p)}>
              <span class="ruolo">{p.ruolo}</span>
              {#if p.richiede_alt}<span class="alt-mancante">Alt da aggiungere</span>{/if}
              {#if p.testo}<span class="testo">{p.testo}</span>{/if}
              {#if p.pagina != null}<span class="pag">p.{p.pagina + 1}</span>{/if}
            </button>
          </li>
        {/each}
      </ul>

      <div class="genera">
        <label class="opz">
          <input type="checkbox" bind:checked={embedDoclang} disabled={!analisi.doclang} />
          Incorpora il DocLang nel PDF (associated file)
        </label>
        <label class="opz">
          <input type="checkbox" bind:checked={pdfa3} />
          Marca come PDF/A-3 (XMP + OutputIntent sRGB)
        </label>
        <button class="salva" onclick={generaBozza} disabled={generando}>
          {generando ? "Generazione…" : "Genera PDF taggato…"}
        </button>
        <p class="avviso">
          Crea un layer di testo invisibile taggato (marked content) sopra le
          pagine originali, con font incorporato e albero dei tag. Rifinisci i
          ruoli e gli Alt delle figure nei pannelli «Tag» e «Correggi», poi
          verifica con «Valida».
          {#if pdfa3}<br /><strong>PDF/A-3:</strong> i metadati vengono scritti,
          ma la conformità piena dipende anche dal contenuto originale e va
          validata con uno strumento esterno (es. veraPDF).{/if}
        </p>
      </div>

      {#if analisi.doclang}
        <div class="doclang">
          <button class="secondaria" onclick={() => (mostraDoclang = !mostraDoclang)}>
            {mostraDoclang ? "Nascondi" : "Mostra"} export DocLang ({analisi.doclang_fmt})
          </button>
          {#if mostraDoclang}
            <pre>{analisi.doclang}</pre>
          {/if}
        </div>
      {/if}
    {/if}
  {/if}
</div>

<!-- Dialog modale di caricamento: copre l'app durante l'analisi (che puo'
     durare minuti). Bloccante e indeterminato, si chiude solo a fine analisi. -->
{#if inCorso}
  <div class="overlay" role="dialog" aria-modal="true" aria-live="polite" aria-busy="true">
    <div class="dialog">
      <span class="spinner-grande" aria-hidden="true"></span>
      <h3>Analisi della struttura in corso…</h3>
      <p>Docling sta inferendo la struttura del documento.</p>
      <p class="nota">Al primo avvio può richiedere qualche minuto: scarica i modelli AI. Puoi attendere, l'app non è bloccata.</p>
    </div>
  </div>
{/if}

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
    margin: 0 0 4px;
    font-size: 15px;
  }
  .sub {
    margin: 0;
    font-size: 12px;
    color: var(--testo-soft);
  }
  .info,
  .err,
  .esito,
  .meta {
    padding: 12px 14px;
    margin: 0;
    color: var(--testo-soft);
    font-size: 13px;
  }
  .err {
    color: var(--errore);
  }
  .esito {
    color: #7ad08f;
    padding-top: 0;
  }
  .meta {
    padding-bottom: 0;
  }
  .installa {
    padding: 0 14px 16px;
  }
  .diag {
    list-style: none;
    margin: 0 0 10px;
    padding: 0;
    font-family: ui-monospace, monospace;
    font-size: 13px;
  }
  .passo-titolo {
    margin: 6px 0 4px;
    font-size: 13px;
    font-weight: 600;
  }
  .passi {
    margin: 0 0 12px;
    padding-left: 20px;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--testo);
  }
  .passi li {
    margin-bottom: 6px;
  }
  .azione {
    padding: 12px 14px 0;
  }
  .suggerimento {
    margin: 0;
    padding: 10px 14px 0;
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
    padding: 3px 14px;
    cursor: pointer;
  }
  button.riga:hover:not(:disabled) {
    background: var(--scheda);
  }
  button.riga:disabled {
    cursor: default;
  }
  .ruolo {
    color: var(--accento);
    font-weight: 600;
    flex: none;
  }
  .testo {
    color: var(--testo-soft);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .alt-mancante {
    color: #e7c98a;
    font-size: 11px;
    flex: none;
  }
  .pag {
    margin-left: auto;
    color: var(--testo-soft);
    font-size: 11px;
    flex: none;
  }
  .salva {
    width: calc(100% - 0px);
    background: var(--accento);
    color: #fff;
    border: none;
    border-radius: 8px;
    padding: 9px 16px;
    cursor: pointer;
  }
  .salva:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .secondaria {
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 6px 12px;
    cursor: pointer;
    font-size: 12px;
  }
  .secondaria:hover {
    border-color: var(--accento);
  }
  .doclang {
    padding: 0 14px 16px;
  }
  .doclang pre {
    margin: 8px 0 0;
    padding: 8px;
    background: var(--scheda);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    font-size: 11px;
    max-height: 240px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .loader {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 14px;
  }
  .spinner {
    flex: none;
    width: 22px;
    height: 22px;
    border: 3px solid var(--bordo);
    border-top-color: var(--accento);
    border-radius: 50%;
    animation: gira 0.8s linear infinite;
  }
  .loader-testo p {
    margin: 0;
    font-size: 13px;
    color: var(--testo);
  }
  .loader-nota {
    margin-top: 2px !important;
    font-size: 11px !important;
    color: var(--testo-soft) !important;
  }
  .genera {
    padding: 4px 14px 8px;
  }
  .opz {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    color: var(--testo);
    margin-bottom: 6px;
    cursor: pointer;
  }
  .opz input {
    cursor: pointer;
  }
  .avviso {
    margin: 8px 0 0;
    font-size: 11.5px;
    line-height: 1.45;
    color: var(--testo-soft);
  }
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(2px);
  }
  .dialog {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 6px;
    max-width: 360px;
    padding: 28px 32px;
    background: var(--sfondo, #1e1e1e);
    border: 1px solid var(--bordo);
    border-radius: 12px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
  }
  .dialog h3 {
    margin: 8px 0 0;
    font-size: 15px;
  }
  .dialog p {
    margin: 0;
    font-size: 13px;
    color: var(--testo-soft);
  }
  .dialog .nota {
    margin-top: 4px;
    font-size: 11.5px;
    line-height: 1.45;
  }
  .spinner-grande {
    width: 40px;
    height: 40px;
    border: 4px solid var(--bordo);
    border-top-color: var(--accento);
    border-radius: 50%;
    animation: gira 0.8s linear infinite;
  }
  @keyframes gira {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spinner,
    .spinner-grande {
      animation-duration: 2s;
    }
  }
</style>
