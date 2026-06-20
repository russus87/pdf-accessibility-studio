// Wrapper sottile sui comandi Tauri del backend Rust.
import { invoke } from "@tauri-apps/api/core";
import { open, save, ask } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";

/** Elenco dei file aperti di recente. */
export function fileRecenti() {
  return invoke("file_recenti");
}

/** Apre il file nel programma di sistema (per stampare o usare un altro lettore). */
export function apriEsterno(percorso) {
  return openPath(percorso);
}

/** Mostra il dialogo di sistema per scegliere uno o piu' PDF. */
export async function scegliPdf() {
  const scelta = await open({
    multiple: true,
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (!scelta) return [];
  return Array.isArray(scelta) ? scelta : [scelta];
}

/** Apre un PDF nel backend: ritorna { id, percorso, pagine, titolo }. */
export function apriPdf(percorso) {
  return invoke("apri_pdf", { percorso });
}

/** Renderizza una pagina (indice 0-based) e ritorna un data URL PNG. */
export async function renderPagina(id, indice, larghezza) {
  const b64 = await invoke("render_pagina", { id, indice, larghezza });
  return `data:image/png;base64,${b64}`;
}

/** Dimensioni della pagina (0-based) in punti PDF: { larghezza, altezza }. */
export function dimensioniPagina(id, indice) {
  return invoke("dimensioni_pagina", { id, indice });
}

/** Chiude un documento nel backend. */
export function chiudiDocumento(id) {
  return invoke("chiudi_documento", { id });
}

// --- Fase 2: validazione ---
export function valida(id) {
  return invoke("valida", { id });
}
// --- Contrasto colori ---
export function contrasto(id, pagina) {
  return invoke("contrasto", { id, pagina });
}

// --- Ricerca testo ---
export function cerca(id, query) {
  return invoke("cerca", { id, query });
}
/** Rettangoli normalizzati (0..1) delle occorrenze in una pagina. */
export function evidenziaRicerca(id, query, pagina) {
  return invoke("evidenzia_ricerca", { id, query, pagina });
}
/** Riquadro (pagina + rettangoli) di un elemento taggato, o null. */
export function riquadroTag(id, riferimento) {
  return invoke("riquadro_tag", { id, riferimento });
}

// --- Metadati documento ---
export function metadati(id) {
  return invoke("metadati", { id });
}
/** Salva i metadati chiedendo se sovrascrivere l'originale o creare una copia.
 *  Ritorna { destinazione, sovrascritto } oppure null se annullato. */
export async function salvaMetadati(id, percorsoOriginale, campi) {
  const sovrascrivi = await ask(
    "Vuoi sovrascrivere il file originale?\n\nScegli No per salvare una copia (l'originale resta intatto).",
    { title: "Salva metadati", kind: "warning" },
  );
  let destinazione;
  if (sovrascrivi) {
    destinazione = percorsoOriginale;
  } else {
    destinazione = await save({ defaultPath: "con-metadati.pdf", filters: [{ name: "PDF", extensions: ["pdf"] }] });
    if (!destinazione) return null;
  }
  await invoke("salva_metadati", { id, ...campi, destinazione });
  return { destinazione, sovrascritto: !!sovrascrivi };
}

// --- Sintesi vocale di riserva (espeak) ---
export function ttsInfo() {
  return invoke("tts_info");
}
/** Sintetizza il testo e ritorna un data URL WAV riproducibile. */
export async function ttsSintesi(testo, lingua, velocita) {
  const b64 = await invoke("tts_sintesi", { testo, lingua, velocita });
  return `data:audio/wav;base64,${b64}`;
}

// --- Piper (voce neurale, scaricata a runtime) ---
export function piperStato() {
  return invoke("piper_stato");
}
export function piperScaricaEngine() {
  return invoke("piper_scarica_engine");
}
export function piperScaricaVoce(voce) {
  return invoke("piper_scarica_voce", { voce });
}
export async function piperSintesi(testo, voce, velocita) {
  const b64 = await invoke("piper_sintesi", { testo, voce, velocita });
  return `data:audio/wav;base64,${b64}`;
}

// --- Operazioni sulle pagine ---
async function salvaPdf(nome) {
  return save({ defaultPath: nome, filters: [{ name: "PDF", extensions: ["pdf"] }] });
}
export async function ruotaPagine(id, pagine, gradi) {
  const destinazione = await salvaPdf("ruotato.pdf");
  if (!destinazione) return null;
  await invoke("ruota_pagine", { id, pagine, gradi, destinazione });
  return destinazione;
}
export async function eliminaPagine(id, pagine) {
  const destinazione = await salvaPdf("modificato.pdf");
  if (!destinazione) return null;
  await invoke("elimina_pagine", { id, pagine, destinazione });
  return destinazione;
}
export async function estraiPagine(id, pagine) {
  const destinazione = await salvaPdf("estratto.pdf");
  if (!destinazione) return null;
  await invoke("estrai_pagine", { id, pagine, destinazione });
  return destinazione;
}
export async function riordinaPagine(id, ordine) {
  const destinazione = await salvaPdf("riordinato.pdf");
  if (!destinazione) return null;
  await invoke("riordina_pagine", { id, ordine, destinazione });
  return destinazione;
}
export async function unisciPdf(id) {
  const scelta = await open({ multiple: true, filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!scelta) return null;
  const altri = Array.isArray(scelta) ? scelta : [scelta];
  const destinazione = await salvaPdf("unito.pdf");
  if (!destinazione) return null;
  await invoke("unisci_pdf", { id, altri, destinazione });
  return destinazione;
}

// --- OCR ---
export function ocrInfo() {
  return invoke("ocr_info");
}
/** Esegue l'OCR e salva un PDF ricercabile; ritorna il percorso o null. */
export async function eseguiOcr(id, lingua) {
  const destinazione = await save({
    defaultPath: "ricercabile.pdf",
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (!destinazione) return null;
  await invoke("esegui_ocr", { id, lingua, destinazione });
  return destinazione;
}

/** Chiede dove salvare ed esporta il report di validazione (html|pdf). */
export async function salvaReportValidazione(id, formato) {
  const destinazione = await save({
    defaultPath: `accessibilita.${formato}`,
    filters: [{ name: formato.toUpperCase(), extensions: [formato] }],
  });
  if (!destinazione) return false;
  await invoke("salva_report_validazione", { id, formato, destinazione });
  return true;
}
export function alberoTag(id) {
  return invoke("albero_tag", { id });
}
export function segnalibri(id) {
  return invoke("segnalibri", { id });
}

// --- Fase 3: testo per la lettura vocale ---
export function testoDocumento(id) {
  return invoke("testo_documento", { id });
}
/** Blocchi di lettura in ordine logico dei tag (vuoto se non taggato). */
export function blocchiLettura(id) {
  return invoke("blocchi_lettura", { id });
}

// --- Fase 4: confronto ---
export function confronta(idA, idB) {
  return invoke("confronta", { idA, idB });
}
export function confrontaImmagine(idA, idB, pagina, larghezza) {
  return invoke("confronta_immagine", { idA, idB, pagina, larghezza });
}
export function reportHtml(idA, idB) {
  return invoke("report_html", { idA, idB });
}

/** Chiede dove salvare e genera il report di confronto (html|pdf). */
export async function salvaReport(idA, idB, formato) {
  const destinazione = await save({
    defaultPath: `confronto.${formato}`,
    filters: [{ name: formato.toUpperCase(), extensions: [formato] }],
  });
  if (!destinazione) return false;
  await invoke("salva_report", { idA, idB, formato, destinazione });
  return true;
}

// --- Fase 5: export tag ---
export function esportaTagStringa(id, formato) {
  return invoke("esporta_tag_stringa", { id, formato });
}
export async function salvaTag(id, formato) {
  const destinazione = await save({
    defaultPath: `tag.${formato}`,
    filters: [{ name: formato.toUpperCase(), extensions: [formato] }],
  });
  if (!destinazione) return false;
  await invoke("salva_tag", { id, formato, destinazione });
  return true;
}

/** Esporta in DocTags (serializzazione DocLang) la struttura di un PDF GIA'
 *  taggato, ricavandola dal suo StructTree. Ritorna false se annullato. */
export async function salvaDoclang(id) {
  const destinazione = await save({
    defaultPath: "documento.doctags.xml",
    filters: [{ name: "DocTags (DocLang)", extensions: ["xml"] }],
  });
  if (!destinazione) return false;
  await invoke("salva_tag", { id, formato: "doclang", destinazione });
  return true;
}

// --- Correzione assistita ---
/** Chiede dove salvare la copia corretta e applica le correzioni.
 *  Ritorna il percorso salvato, oppure null se annullato. */
export async function correggi(id, opts) {
  const { lang, titolo, displayDocTitle, autore, soggetto, paroleChiave, alt, ruoli } = opts;
  const destinazione = await save({
    defaultPath: "corretto.pdf",
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (!destinazione) return null;
  await invoke("correggi", {
    id,
    lang: lang || null,
    titolo: titolo || null,
    displayDocTitle: !!displayDocTitle,
    autore: autore || null,
    soggetto: soggetto || null,
    paroleChiave: paroleChiave || null,
    alt: alt || [],
    ruoli: ruoli || [],
    destinazione,
  });
  return destinazione;
}

/** Genera i segnalibri dai titoli e salva una copia; ritorna {dest, n}. */
export async function generaSegnalibri(id) {
  const destinazione = await save({
    defaultPath: "con-segnalibri.pdf",
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (!destinazione) return null;
  const n = await invoke("genera_segnalibri", { id, destinazione });
  return { dest: destinazione, n };
}

// --- Suggerimento Alt con AI (Claude vision) ---
export function statoAi() {
  return invoke("stato_ai");
}
export function impostaAi(chiave, modello) {
  return invoke("imposta_ai", { chiave: chiave ?? null, modello: modello ?? null });
}
export function suggerisciAlt(id, pagina) {
  return invoke("suggerisci_alt", { id, pagina });
}

// --- Auto-tagging AI (Docling, solo PDF non taggati) ---
/** Stato del ponte Docling: { python_disponibile, python_bin, docling_installato, script_pronto }. */
export function doclingStato() {
  return invoke("docling_stato");
}
/** Deposita/aggiorna lo script-ponte Python nella cartella dati dell'app. */
export function doclingPrepara() {
  return invoke("docling_prepara");
}
/** Analizza con Docling la struttura di un PDF *non taggato* e ritorna le
 *  proposte di tag: { lingua, blocchi_totali, proposte, albero, doclang, doclang_fmt }.
 *  Va in errore se il PDF e' gia' taggato o se Python/Docling non sono disponibili. */
export function analizzaStrutturaAi(id) {
  return invoke("analizza_struttura_ai", { id });
}
/** Genera un PDF *realmente taggato* dalle proposte: overlay di testo invisibile
 *  taggato + marked content sopra le pagine originali, font incorporato, albero
 *  dei tag completo. Opzioni: { doclang, pdfa3 }. Chiede dove salvare; ritorna
 *  { dest, n } o null se annullato. Il marked content c'è, ma rifinisci comunque
 *  ruoli/Alt nei pannelli Tag/Correggi e verifica con Valida (PDF/A-3 best effort). */
export async function generaPdfAccessibile(id, proposte, lang, titolo, opts = {}) {
  const destinazione = await save({
    defaultPath: opts.pdfa3 ? "accessibile-pdfa3.pdf" : "accessibile.pdf",
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (!destinazione) return null;
  const n = await invoke("genera_pdf_accessibile", {
    id,
    proposte: proposte.map((p) => ({
      ruolo: p.ruolo,
      pagina: p.pagina,
      testo: p.testo,
      bbox: p.bbox ?? null,
      // Campo di struct annidata: Tauri NON converte camelCase qui, usa snake_case.
      coord_basso: p.coord_basso ?? true,
    })),
    lang: lang || null,
    titolo: titolo || null,
    doclang: opts.doclang || null,
    pdfa3: !!opts.pdfa3,
    destinazione,
  });
  return { dest: destinazione, n };
}

/** Marca gli elementi (per riferimento) come Artifact: li toglie dall'ordine di
 *  lettura e riscrive il loro marked content. Salva una copia; ritorna { dest, n }
 *  o null se annullato. */
export async function marcaArtifact(id, riferimenti) {
  const destinazione = await save({
    defaultPath: "con-artifact.pdf",
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (!destinazione) return null;
  const n = await invoke("marca_artifact", { id, riferimenti, destinazione });
  return { dest: destinazione, n };
}

/** Applica attributi di accessibilità alle celle di tabella (Scope/RowSpan/ColSpan)
 *  e salva una copia. `celle` = [{ riferimento, scope?, rowSpan?, colSpan? }].
 *  Ritorna { dest, n } o null se annullato. */
export async function applicaTabella(id, celle) {
  const destinazione = await save({
    defaultPath: "tabelle-accessibili.pdf",
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (!destinazione) return null;
  const n = await invoke("applica_tabella", {
    id,
    // Campi di struct annidata: Tauri NON converte camelCase qui, usa snake_case.
    celle: celle.map((c) => ({
      riferimento: c.riferimento,
      scope: c.scope ?? null,
      row_span: c.rowSpan ?? null,
      col_span: c.colSpan ?? null,
    })),
    destinazione,
  });
  return { dest: destinazione, n };
}

// --- Creazione PDF da modello HTML + dati JSON (Fase 22) ---
/** Modello e dati JSON di esempio: { modello, dati }. */
export function modelloEsempio() {
  return invoke("modello_esempio");
}
/** Renderizza il modello con i dati e ritorna l'HTML finale (anteprima). */
export function anteprimaModello(modello, dati) {
  return invoke("anteprima_modello", { modello, dati });
}
/** Genera il PDF dal modello + dati e lo salva; ritorna il percorso o null. */
export async function generaDaModello(modello, dati) {
  const destinazione = await save({
    defaultPath: "documento.pdf",
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (!destinazione) return null;
  await invoke("genera_da_modello", { modello, dati, destinazione });
  return destinazione;
}

// --- Sovrapposizione: testo / immagini / filigrana (Fase 24) ---
/** Applica elementi (testo/immagine) e filigrana al PDF e salva una copia.
 *  `elementi` = [{ tipo:"testo"|"immagine", pagina, x_mm, y_mm, ... }] con campi
 *  in snake_case; `filigrana` = { tipo, ... } o null. Ritorna { dest, n } o null. */
export async function sovrapponi(id, elementi, filigrana) {
  const destinazione = await save({
    defaultPath: "modificato.pdf",
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (!destinazione) return null;
  const n = await invoke("sovrapponi", { id, elementi, filigrana: filigrana ?? null, destinazione });
  return { dest: destinazione, n };
}

// --- Protezione/cifratura (Fase 27) ---
export function pdfProtetto(id) {
  return invoke("pdf_protetto", { id });
}
/** Cifra il PDF con password+permessi; salva una copia. Ritorna il percorso o null. */
export async function proteggiPdf(id, passwordUtente, passwordProprietario, permessi) {
  const destinazione = await save({ defaultPath: "protetto.pdf", filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!destinazione) return null;
  await invoke("proteggi_pdf", { id, passwordUtente, passwordProprietario, permessi, destinazione });
  return destinazione;
}
/** Rimuove la protezione (data la password); salva una copia. */
export async function sbloccaPdf(id, password) {
  const destinazione = await save({ defaultPath: "sbloccato.pdf", filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!destinazione) return null;
  await invoke("sblocca_pdf", { id, password, destinazione });
  return destinazione;
}

// --- Conversione immagini (Fase 29) ---
/** Esporta le pagine come immagini in una cartella scelta. Ritorna i percorsi o null. */
export async function esportaImmagini(id, larghezza, jpeg) {
  const cartella = await open({ directory: true });
  if (!cartella) return null;
  return invoke("esporta_immagini", { id, larghezza, jpeg, cartella });
}
/** Crea un PDF da immagini (base64) e lo salva. Ritorna il percorso o null. */
export async function creaPdfDaImmagini(immaginiBase64) {
  const destinazione = await save({ defaultPath: "da-immagini.pdf", filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!destinazione) return null;
  await invoke("crea_pdf_da_immagini", { immagini: immaginiBase64, destinazione });
  return destinazione;
}

// --- Numerazione / intestazioni (Fase 30) ---
export async function numeraPagine(id, opts) {
  const destinazione = await save({ defaultPath: "numerato.pdf", filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!destinazione) return null;
  const n = await invoke("numera_pagine", {
    id,
    testo: opts.testo,
    dimPt: opts.dimPt,
    colore: opts.colore ?? null,
    margineMm: opts.margineMm,
    ancora: opts.ancora,
    inizio: opts.inizio,
    destinazione,
  });
  return { dest: destinazione, n };
}

// --- Redazione (Fase 28) ---
/** Redige le aree (definitivamente) e salva una copia. `aree` con campi snake_case. */
export async function redigi(id, aree) {
  const destinazione = await save({ defaultPath: "redatto.pdf", filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!destinazione) return null;
  const n = await invoke("redigi", { id, aree, destinazione });
  return { dest: destinazione, n };
}

// --- Ottimizzazione (Fase 31) ---
export async function ottimizza(id) {
  const destinazione = await save({ defaultPath: "ottimizzato.pdf", filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!destinazione) return null;
  const r = await invoke("ottimizza", { id, destinazione });
  return { dest: destinazione, ...r };
}

// --- Estrazione testo/HTML/Markdown (Fase 32) ---
export async function esportaContenuto(id, formato) {
  const ext = formato === "markdown" ? "md" : formato; // txt | html | md
  const destinazione = await save({ defaultPath: `contenuto.${ext}`, filters: [{ name: ext.toUpperCase(), extensions: [ext] }] });
  if (!destinazione) return false;
  await invoke("esporta_contenuto", { id, formato, destinazione });
  return true;
}

// --- Compilazione moduli (Fase 34) ---
/** `valori` = [{ riferimento, valore }]. */
export async function compilaModulo(id, valori, flatten) {
  const destinazione = await save({ defaultPath: flatten ? "compilato-bloccato.pdf" : "compilato.pdf", filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!destinazione) return null;
  const n = await invoke("compila_modulo", { id, valori, flatten: !!flatten, destinazione });
  return { dest: destinazione, n };
}
export function esportaDatiForm(id) {
  return invoke("esporta_dati_form", { id });
}
/** Salva una stringa su file scelto dall'utente. Ritorna true se salvato. */
export async function salvaTesto(contenuto, nomeDefault, ext) {
  const destinazione = await save({ defaultPath: nomeDefault, filters: [{ name: ext.toUpperCase(), extensions: [ext] }] });
  if (!destinazione) return false;
  await invoke("salva_testo", { contenuto, destinazione });
  return true;
}

// --- Elaborazione batch (Fase 33) ---
/** Sceglie i PDF e la cartella di output, esegue l'operazione su tutti. */
export async function batch(operazione, testo) {
  const scelta = await open({ multiple: true, filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!scelta) return null;
  const files = Array.isArray(scelta) ? scelta : [scelta];
  const cartella = await open({ directory: true });
  if (!cartella) return null;
  return invoke("batch", { files, operazione, testo: testo ?? null, cartella });
}

// --- Organizer pagine (Fase 35) ---
export async function ritagliaPagine(id, pagine, box_) {
  const destinazione = await save({ defaultPath: "ritagliato.pdf", filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!destinazione) return null;
  await invoke("ritaglia_pagine", { id, pagine, xMm: box_.x, yMm: box_.y, larghezzaMm: box_.w, altezzaMm: box_.h, destinazione });
  return destinazione;
}
export async function inserisciPagine(id, posizione) {
  const altro = await open({ filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!altro) return null;
  const destinazione = await save({ defaultPath: "unito.pdf", filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!destinazione) return null;
  await invoke("inserisci_pagine", { id, altro, posizione, destinazione });
  return destinazione;
}

// --- Stampa unione / mail merge (Fase 36) ---
export async function stampaUnione(modello, dati, combina) {
  let destinazione;
  if (combina) {
    destinazione = await save({ defaultPath: "unione.pdf", filters: [{ name: "PDF", extensions: ["pdf"] }] });
  } else {
    destinazione = await open({ directory: true });
  }
  if (!destinazione) return null;
  const n = await invoke("stampa_unione", { modello, dati, combina: !!combina, destinazione });
  return { n, combina: !!combina, dest: destinazione };
}

// --- AI sul documento (Fase 37) ---
export function aiRiassumi(id) {
  return invoke("ai_riassumi", { id });
}
export function aiDomanda(id, domanda) {
  return invoke("ai_domanda", { id, domanda });
}
export function aiTraduci(id, lingua) {
  return invoke("ai_traduci", { id, lingua });
}

// --- Annotazioni (Fase 38) ---
export async function annota(id, annotazioni) {
  const destinazione = await save({ defaultPath: "annotato.pdf", filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!destinazione) return null;
  const n = await invoke("annota", { id, annotazioni, destinazione });
  return { dest: destinazione, n };
}
export function riepilogoAnnotazioni(id) {
  return invoke("riepilogo_annotazioni", { id });
}

// --- Libreria firme (Fase 25) ---
/** Elenco firme salvate: [{ nome, dati_base64 }]. */
export function firmeElenco() {
  return invoke("firme_elenco");
}
/** Salva una firma (PNG in base64). */
export function firmaSalva(nome, datiBase64) {
  return invoke("firma_salva", { nome, datiBase64 });
}
/** Elimina una firma per nome. */
export function firmaElimina(nome) {
  return invoke("firma_elimina", { nome });
}

// --- Editor visuale: salvataggio combinato (overlay + campi) (Fase 23/26) ---
/** Applica elementi+filigrana+campi e salva un unico PDF. Ritorna { dest, n }. */
export async function salvaEditor(id, elementi, filigrana, campi) {
  const destinazione = await save({
    defaultPath: "modificato.pdf",
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (!destinazione) return null;
  const n = await invoke("salva_editor", {
    id,
    elementi,
    filigrana: filigrana ?? null,
    campi,
    destinazione,
  });
  return { dest: destinazione, n };
}

// --- Form accessibili (AcroForm) ---
/** Legge i campi del modulo: [{ riferimento, nome, tipo, tooltip, pagina }]. */
export function leggiForm(id) {
  return invoke("leggi_form", { id });
}
/** Imposta i tooltip /TU sui campi e (se tabsStruttura) /Tabs /S sulle pagine.
 *  `campi` = [{ riferimento, tooltip }]. Salva una copia; ritorna { dest, n } o null. */
export async function applicaForm(id, campi, tabsStruttura) {
  const destinazione = await save({
    defaultPath: "modulo-accessibile.pdf",
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (!destinazione) return null;
  const n = await invoke("applica_form", {
    id,
    campi: campi.map((c) => ({ riferimento: c.riferimento, tooltip: c.tooltip })),
    tabsStruttura: !!tabsStruttura,
    destinazione,
  });
  return { dest: destinazione, n };
}

/** Riordina gli elementi di primo livello (ordine di lettura) e salva una copia. */
export async function riordina(id, ordine) {
  const destinazione = await save({
    defaultPath: "riordinato.pdf",
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (!destinazione) return null;
  await invoke("riordina", { id, ordine, destinazione });
  return destinazione;
}
