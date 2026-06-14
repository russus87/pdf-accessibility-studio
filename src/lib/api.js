// Wrapper sottile sui comandi Tauri del backend Rust.
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";

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

/** Chiude un documento nel backend. */
export function chiudiDocumento(id) {
  return invoke("chiudi_documento", { id });
}

// --- Fase 2: validazione ---
export function valida(id) {
  return invoke("valida", { id });
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
