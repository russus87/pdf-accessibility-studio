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

## Fase 2 — Validazione accessibilità ✅

- ✅ Lettura structure tree / tag (StructTreeRoot, ruoli, RoleMap, Alt, Lang) — `core/struttura.rs` (`lopdf`)
- ✅ Motore di regole PDF/UA + WCAG — `core/validazione.rs`: documento taggato,
  `/Lang`, titolo nei metadati, DisplayDocTitle, alt-text figure, TH nelle tabelle,
  testo dei link, presenza di intestazioni
- ✅ Pannello esiti per gravità (errore/avviso/ok) — `components/PannelloValidazione.svelte`
- ✅ Comandi `valida` / `albero_tag`

## Fase 3 — Sintesi vocale (stile screen reader) ✅

- ✅ Estrazione testo per pagina — `core/documento.rs::testo_pagine`
- ✅ Lettura frase per frase con evidenziazione, click per saltare a una frase —
  `components/LettoreVocale.svelte` (Web Speech API del webview)
- ✅ Controlli play/pausa/stop, scelta voce, velocità
- ⏳ Futuro: lettura nell'esatto ordine dei tag (oggi: ordine di pagina) e highlight per parola

## Fase 4 — Confronto PDF ✅

- ✅ Confronto **testuale** (diff con `similar`) — `core/confronto.rs`
- ✅ Confronto **immagine** pixel-to-pixel (rendering + heatmap rossa) — `core/confronto.rs`
- ✅ Confronto **per tag/struttura** (sequenza ruoli indentata)
- ✅ Report in **HTML** e **PDF** dallo stesso modello (`printpdf` via `from_html`)
- ✅ Vista `components/Confronto.svelte` + comandi `confronta` / `confronta_immagine` / `report_html` / `salva_report`

## Fase 5 — Export tag ✅

- ✅ Export dell'albero dei tag in **JSON** e **XML** — `core/export.rs`
- ✅ Pannello struttura + pulsanti export — `components/PannelloTag.svelte` + comandi `esporta_tag_stringa` / `salva_tag`

## Fase 6 — Versione semi-completa ✅

- ✅ **Cache documenti**: i PDF aperti restano in memoria (`PdfDocument` Send+Sync),
  niente riapertura ad ogni render — `core/documento.rs`; eviction alla chiusura scheda
- ✅ **Anteprime pagine** (thumbnail navigabili) — `components/Anteprime.svelte`
- ✅ **Pannello struttura cliccabile**: ogni tag conosce la sua pagina (`/Pg`) e
  cliccandolo il visore ci salta — `core/struttura.rs` + `components/PannelloTag.svelte`
- ✅ **Correzione assistita**: imposta lingua, titolo e DisplayDocTitle e salva una
  copia corretta (l'originale resta intatto) — `core/correzione.rs` (`lopdf`) +
  `components/PannelloCorrezione.svelte`. Verificato: validazione da 2 errori → 1.

## Fase 7 — Completamento ✅

- ✅ **Editor Alt per singole figure**: ogni `NodoTag` ha un `riferimento` (ObjectId);
  il pannello correzione elenca le figure (con badge "manca") e scrive l'Alt sul
  giusto oggetto salvando una copia — `core/correzione.rs`, `core/struttura.rs`.
  Verificato da test (`core/tests/pipeline.rs`): round-trip su PDF taggato minimale.
- ✅ **Indice / segnalibri navigabili** (outline Pdfium) — `core/segnalibri.rs` +
  `components/PannelloSegnalibri.svelte`; click → vai alla pagina.
- ✅ **Lettura in ordine logico + Alt immagini + evidenziazione per parola**:
  il lettore intercala il testo alternativo delle figure e usa gli eventi
  `onboundary` per evidenziare la parola in lettura; opzione "fai seguire il visore".

## Idee future ⏳

- ⏳ Ordine di lettura editabile (riordino dello structure tree)
- ⏳ Estrazione testo nell'esatto ordine MCID dei tag (oggi: ordine di pagina/contenuto)
- ⏳ Tabelle: editor intestazioni (TH/scope); ruoli/heading correggibili dall'app
