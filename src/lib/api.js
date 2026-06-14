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
export function alberoTag(id) {
  return invoke("albero_tag", { id });
}

// --- Fase 3: testo per la lettura vocale ---
export function testoDocumento(id) {
  return invoke("testo_documento", { id });
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
export async function correggi(id, { lang, titolo, displayDocTitle }) {
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
    destinazione,
  });
  return destinazione;
}
