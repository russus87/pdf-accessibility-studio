# Diario delle implementazioni — PDF Accessibility Studio

Registro di ciò che viene implementato, fase per fase. Ogni fase compila
(`cargo check --workspace`) e costruisce il frontend (`npm run build`) prima di
passare oltre.

Legenda: ✅ fatto · 🚧 in corso · ⏳ pianificato · ❌ fuori scope

---

## Fase 1 — Visore multi-scheda + build automatiche ✅

- ✅ Workspace Cargo `core` (logica pura) + `src-tauri` (app), come oxiterm/rustman
- ✅ Apertura PDF e lettura info base (numero pagine, titolo) — `core/documento.rs`
- ✅ Rendering pagine in PNG via **Pdfium** (`pdfium-render`) — `core/documento.rs`
- ✅ Caricamento libreria nativa Pdfium da più percorsi — `core/pdfium.rs`
- ✅ Comandi Tauri `apri_pdf` / `render_pagina` / `chiudi_documento` — `src-tauri/src/comandi.rs`
- ✅ UI Svelte: barra schede (più PDF insieme), toolbar (zoom, navigazione), visore
- ✅ Script `scripts/fetch-pdfium.sh` (dev + CI)
- ✅ GitHub Actions: build Win/Mac/Linux, pacchetto Arch `.pkg.tar.zst`, repo `-dist` per auto-update
- ✅ Icone, PKGBUILD, file `.desktop`

## Fase 2 — Validazione accessibilità ⏳

- ⏳ Lettura structure tree / tag (StructTreeRoot, ruoli) — `lopdf`
- ⏳ Regole base (PDF/UA + WCAG): documento taggato, `/Lang`, titolo nei metadati,
  alt-text immagini, intestazioni tabelle, ordine di lettura
- ⏳ Pannello report con esiti per regola e collegamento all'elemento

## Fase 3 — Sintesi vocale (stile screen reader) ⏳

- ⏳ Lettura nell'ordine logico dei tag con evidenziazione — crate `tts`
- ⏳ Controlli voce/velocità, play/pausa, navigazione per elemento

## Fase 4 — Confronto PDF ⏳

- ⏳ Confronto **testuale** (diff con `similar`)
- ⏳ Confronto **immagine** pixel-to-pixel (rendering + heatmap differenze)
- ⏳ Confronto **per tag/struttura**
- ⏳ Report in **HTML** e **PDF** (`printpdf`/`genpdf` dallo stesso modello di diff)

## Fase 5 — Export tag e rifiniture ⏳

- ⏳ Export dell'albero dei tag (es. JSON/XML)
- ⏳ Pannello struttura navigabile, segnalibri, anteprime pagine
