// Wrapper sottile sui comandi Tauri del backend Rust.
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

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
