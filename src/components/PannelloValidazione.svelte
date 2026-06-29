<script>
  // Pannello validazione accessibilita': mostra gli esiti delle regole.
  import { schede } from "../lib/schede.svelte.js";
  import { valida, salvaReportValidazione, ocrInfo, eseguiOcr, contrasto, matterhorn, salvaReportMatterhorn, autoCorreggi } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);

  // Mappa regola → strumento di correzione (salto guidato al pannello giusto).
  const FIX = {
    "Testo alternativo immagini": { pannello: "tag", modo: "alt", label: "Aggiungi testo alternativo" },
    "Ordine dei titoli": { pannello: "tag", modo: "ruoli", label: "Sistema gerarchia titoli" },
    "Struttura dei titoli": { pannello: "tag", modo: "ruoli", label: "Sistema i titoli" },
    "Intestazioni tabelle": { pannello: "tag", modo: "tabelle", label: "Correggi intestazioni tabelle" },
    "Righe delle tabelle": { pannello: "tag", modo: "tabelle", label: "Correggi tabelle" },
    "Struttura liste": { pannello: "tag", modo: "ruoli", label: "Correggi nei Tag" },
    "Testo dei link": { pannello: "tag", modo: "ruoli", label: "Correggi nei Tag" },
    "Documento taggato": { pannello: "autotag", modo: null, label: "Genera tag automatici" },
  };

  // Correzione automatica (lingua, titolo, DisplayDocTitle, id PDF/UA).
  let langFix = $state("it");
  let autoInCorso = $state(false);
  async function autoFix() {
    if (!s) return;
    esito = null;
    autoInCorso = true;
    try {
      const r = await autoCorreggi(s.id, langFix);
      if (r) {
        esito = `Correzione automatica: errori ${r.errori_prima} → ${r.errori_dopo}. Apro la copia corretta…`;
        await schede.apri(r.dest); // diventa la scheda attiva → rivalida
      }
    } catch (e) {
      esito = `Errore: ${e}`;
    } finally {
      autoInCorso = false;
    }
  }
  let vista = $state("validazione"); // "validazione" | "matterhorn"
  let mh = $state(null);
  let mhCaricamento = $state(false);
  let report = $state(null);

  async function caricaMatterhorn() {
    vista = "matterhorn";
    if (mh || !s) return;
    mhCaricamento = true;
    try {
      mh = await matterhorn(s.id);
    } catch (e) {
      esito = `Matterhorn: ${e}`;
    } finally {
      mhCaricamento = false;
    }
  }
  async function esportaMatterhorn() {
    try {
      const ok = await salvaReportMatterhorn(s.id);
      if (ok) esito = "Report Matterhorn HTML salvato.";
    } catch (e) {
      esito = `Errore: ${e}`;
    }
  }
  const mhBadge = { superato: "ok", fallito: "errore", avviso: "avviso", manuale: "man" };
  let caricamento = $state(false);
  let errore = $state(null);
  let esito = $state(null);

  // OCR
  let ocr = $state({ disponibile: false, lingue: [] });
  let linguaOcr = $state("eng");
  let ocrInCorso = $state(false);
  $effect(() => {
    ocrInfo().then((r) => {
      ocr = r;
      if (r.lingue.length) linguaOcr = r.lingue.find((l) => l === "ita") || r.lingue[0];
    }).catch(() => {});
  });

  // Contrasto colori della pagina corrente.
  let contr = $state(null);
  let contrInCorso = $state(false);
  const rgb = (c) => `rgb(${c[0]},${c[1]},${c[2]})`;

  async function analizzaContrasto() {
    if (!s) return;
    contrInCorso = true;
    contr = null;
    try {
      contr = await contrasto(s.id, s.pagina);
    } catch (e) {
      esito = `Contrasto: ${e}`;
    } finally {
      contrInCorso = false;
    }
  }

  async function ocrEsegui() {
    esito = null;
    ocrInCorso = true;
    try {
      const dest = await eseguiOcr(s.id, linguaOcr);
      if (dest) esito = "PDF ricercabile creato.";
    } catch (e) {
      esito = `OCR: ${e}`;
    } finally {
      ocrInCorso = false;
    }
  }

  async function esporta(formato) {
    esito = null;
    try {
      const ok = await salvaReportValidazione(s.id, formato);
      if (ok) esito = `Report ${formato.toUpperCase()} salvato.`;
    } catch (e) {
      esito = `Errore: ${e}`;
    }
  }

  // Rivalida quando cambia la scheda attiva.
  $effect(() => {
    if (!s) return;
    const id = s.id;
    let annullato = false;
    caricamento = true;
    errore = null;
    report = null;
    mh = null;
    valida(id)
      .then((r) => !annullato && (report = r))
      .catch((e) => !annullato && (errore = String(e)))
      .finally(() => !annullato && (caricamento = false));
    return () => (annullato = true);
  });

  const icona = { errore: "✕", avviso: "!", ok: "✓", man: "?" };
</script>

<div class="pannello">
  <header>
    <h3>Validazione accessibilità</h3>
    <div class="modi">
      <button class:on={vista === "validazione"} onclick={() => (vista = "validazione")}>WCAG/PDF-UA</button>
      <button class:on={vista === "matterhorn"} onclick={caricaMatterhorn} disabled={!s}>Matterhorn</button>
    </div>
    {#if vista === "validazione" && report}
      <div class="azioni">
        <button onclick={() => esporta("html")}>Report HTML</button>
        <button onclick={() => esporta("pdf")}>Report PDF</button>
      </div>
    {:else if vista === "matterhorn" && mh}
      <div class="azioni">
        <button onclick={esportaMatterhorn}>Report HTML</button>
      </div>
    {/if}
  </header>

  {#if esito}<p class="esito">{esito}</p>{/if}

  {#if s && vista === "validazione"}
    <div class="ocr contr">
      <span class="ocr-tit">Contrasto pagina {s.pagina + 1} (stima):</span>
      <button onclick={analizzaContrasto} disabled={contrInCorso}>{contrInCorso ? "…" : "Analizza"}</button>
      {#if contr}
        <span class="campione" style={`background:${rgb(contr.testo)}`} title="testo"></span>
        <span class="campione" style={`background:${rgb(contr.sfondo)}`} title="sfondo"></span>
        <b>{contr.rapporto.toFixed(2)}:1</b>
        <span class={contr.aa_normale ? "pass" : "fail"}>AA testo {contr.aa_normale ? "✓" : "✗"}</span>
        <span class={contr.aa_grande ? "pass" : "fail"}>AA grande {contr.aa_grande ? "✓" : "✗"}</span>
      {/if}
    </div>
    <div class="ocr">
      {#if ocr.disponibile}
        <span class="ocr-tit">OCR (PDF scansionati):</span>
        <select bind:value={linguaOcr}>
          {#each ocr.lingue as l}<option value={l}>{l}</option>{/each}
        </select>
        <button onclick={ocrEsegui} disabled={ocrInCorso}>{ocrInCorso ? "OCR in corso…" : "Crea PDF ricercabile"}</button>
      {:else}
        <span class="ocr-tit">OCR non disponibile: installa <code>tesseract</code> (+ dati lingua).</span>
      {/if}
    </div>
  {/if}

  {#if vista === "matterhorn"}
    {#if mhCaricamento}
      <p class="info">Analisi Matterhorn…</p>
    {:else if mh}
      <div class="riepilogo">
        <span class="badge ok2">{mh.superati} superati</span>
        <span class="badge err">{mh.falliti} falliti</span>
        <span class="badge avv">{mh.avvisi} avvisi</span>
        <span class="badge man">{mh.manuali} manuali</span>
      </div>
      <ul>
        {#each mh.voci as v}
          <li class={mhBadge[v.stato]}>
            <span class="segno {mhBadge[v.stato]}">{icona[mhBadge[v.stato]] || "?"}</span>
            <div>
              <div class="regola">{v.checkpoint} · {v.titolo} {#if v.wcag !== "—"}<span class="wcag">WCAG {v.wcag}</span>{/if}</div>
              <div class="msg">{v.dettaglio}</div>
            </div>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="info">Premi “Matterhorn” per generare il report.</p>
    {/if}
  {:else if caricamento}
    <p class="info">Analisi in corso…</p>
  {:else if errore}
    <p class="err">{errore}</p>
  {:else if report}
    <div class="riepilogo">
      <span class="badge err">{report.errori} errori</span>
      <span class="badge avv">{report.avvisi} avvisi</span>
    </div>
    {#if report.errori + report.avvisi > 0}
      <div class="guida-fix">
        <div class="gf-tit">🔧 Correzione guidata</div>
        <div class="gf-riga">
          <label>Lingua
            <input type="text" bind:value={langFix} maxlength="5" style="width:54px" />
          </label>
          <button onclick={autoFix} disabled={autoInCorso}>
            {autoInCorso ? "Correzione…" : "Correggi automaticamente"}
          </button>
        </div>
        <div class="gf-nota">Imposta lingua, titolo, DisplayDocTitle e id PDF/UA, poi rivalida sulla copia. Per gli altri problemi usa i pulsanti «Correggi» qui sotto.</div>
      </div>
    {/if}
    <ul>
      {#each report.esiti as e}
        <li class={e.gravita}>
          <span class="segno {e.gravita}">{icona[e.gravita]}</span>
          <div class="corpo">
            <div class="regola">{e.regola}</div>
            <div class="msg">{e.messaggio}</div>
            {#if e.gravita !== "ok" && FIX[e.regola]}
              <button class="fixbtn" onclick={() => schede.apriPannello(FIX[e.regola].pannello, FIX[e.regola].modo)}>{FIX[e.regola].label} →</button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
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
  .modi {
    display: flex;
    gap: 6px;
    margin-bottom: 8px;
  }
  .modi button {
    flex: 1;
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 5px 8px;
    cursor: pointer;
    font-size: 12px;
  }
  .modi button.on {
    background: var(--accento);
    border-color: var(--accento);
    color: #fff;
  }
  .azioni {
    display: flex;
    gap: 6px;
  }
  .azioni button {
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 5px 10px;
    cursor: pointer;
    font-size: 12px;
  }
  .azioni button:hover {
    border-color: var(--accento);
  }
  .esito {
    margin: 0;
    padding: 10px 14px;
    color: #7ad08f;
    font-size: 13px;
  }
  .ocr {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--bordo);
    font-size: 12px;
    color: var(--testo-soft);
  }
  .ocr select {
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 4px 8px;
  }
  .ocr button {
    background: var(--accento);
    color: #fff;
    border: none;
    border-radius: 6px;
    padding: 5px 10px;
    cursor: pointer;
  }
  .ocr button:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .ocr code {
    color: var(--accento);
  }
  .campione {
    width: 16px;
    height: 16px;
    border-radius: 3px;
    border: 1px solid var(--bordo);
    display: inline-block;
  }
  .pass {
    color: #6ee08a;
  }
  .fail {
    color: #ff8d8d;
  }
  .riepilogo {
    display: flex;
    gap: 8px;
    padding: 12px 14px;
  }
  .guida-fix {
    margin: 0 14px 8px;
    padding: 10px 12px;
    background: var(--scheda);
    border: 1px solid var(--bordo);
    border-radius: 8px;
  }
  .gf-tit { font-weight: 600; font-size: 13px; margin-bottom: 6px; }
  .gf-riga { display: flex; align-items: center; gap: 10px; }
  .gf-riga label { font-size: 12px; color: var(--testo-soft); display: inline-flex; align-items: center; gap: 6px; }
  .gf-riga input {
    background: var(--sfondo, var(--scheda)); color: var(--testo);
    border: 1px solid var(--bordo); border-radius: 6px; padding: 4px 6px;
  }
  .gf-riga button {
    background: var(--accento); color: #fff; border: none; border-radius: 6px;
    padding: 6px 12px; cursor: pointer; font-size: 12px;
  }
  .gf-riga button:disabled { opacity: 0.6; cursor: default; }
  .gf-nota { margin-top: 6px; font-size: 11px; color: var(--testo-soft); line-height: 1.4; }
  .corpo { display: flex; flex-direction: column; gap: 2px; }
  .fixbtn {
    align-self: flex-start; margin-top: 6px;
    background: transparent; color: var(--accento);
    border: 1px solid var(--accento); border-radius: 6px;
    padding: 3px 10px; cursor: pointer; font-size: 12px;
  }
  .fixbtn:hover { background: var(--accento); color: #fff; }
  .badge {
    padding: 3px 10px;
    border-radius: 12px;
    font-size: 12px;
    font-weight: bold;
  }
  .badge.err {
    background: #4a2326;
    color: #ff9a9a;
  }
  .badge.avv {
    background: #4a3a1f;
    color: #ffcf7a;
  }
  .badge.ok2 {
    background: #173021;
    color: #7ad08f;
  }
  .badge.man {
    background: #2b2f36;
    color: #c4ccd6;
  }
  .segno.man {
    background: #57606a;
  }
  .wcag {
    font-size: 10px;
    color: #7ab0ff;
    border: 1px solid #2e4a6a;
    border-radius: 8px;
    padding: 0 6px;
    margin-left: 6px;
    font-weight: normal;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0 8px 16px;
  }
  li {
    display: flex;
    gap: 10px;
    padding: 10px;
    border-radius: 8px;
    align-items: flex-start;
  }
  li + li {
    margin-top: 4px;
  }
  .segno {
    flex: none;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: 13px;
    font-weight: bold;
    color: #fff;
  }
  .segno.errore {
    background: #c0392b;
  }
  .segno.avviso {
    background: #d68910;
  }
  .segno.ok {
    background: #27885a;
  }
  .regola {
    font-weight: 600;
  }
  .msg {
    color: var(--testo-soft);
    font-size: 13px;
    margin-top: 2px;
  }
  .info,
  .err {
    padding: 14px;
    color: var(--testo-soft);
  }
  .err {
    color: var(--errore);
  }
</style>
