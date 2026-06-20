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

## Fase 8 — Tag avanzati ✅

- ✅ **Lettura in ordine logico (MCID)**: `core/lettura.rs` parsa i content stream
  (BDC/EMC + Tj/TJ) decodificando con i font di lopdf e ricostruisce la sequenza
  di lettura per elemento; il lettore vocale la usa (fallback all'ordine di pagina),
  con etichette di ruolo. Verificato dal test (estrae "Ciao mondo" da un P/MCID).
- ✅ **Editor ruoli** (P→H1, marcare TH, ecc.): cambia `/S` sull'elemento e salva
  una copia — `core/correzione.rs` + pannello Tag (modalità "Ruoli").
- ✅ **Ordine di lettura editabile**: riordino degli elementi di primo livello
  (riscrive `/K` dello StructTreeRoot) — `core/correzione.rs::riordina` + pannello
  Tag (modalità "Riordina", frecce su/giù).

Test `core/tests/pipeline.rs` copre: struttura, lettura MCID, validazione,
Alt, cambio ruolo e riordino su un PDF taggato costruito al volo.

## Fase 9 — Validazione profonda ✅

- ✅ Nuove regole PDF/UA + WCAG in `core/validazione.rs`:
  ordine dei titoli (Hn senza salti, inizio da H1), struttura liste (L→LI),
  righe tabella (Table→TR), font con `ToUnicode`, identificatore **PDF/UA** in XMP,
  rilevamento **PDF scansionati** (nessun testo estraibile → suggerisce OCR).
- ✅ Dati extra nel parser (`core/struttura.rs`): conteggio font/ToUnicode, flag PDF/UA.
- ✅ **Report di validazione esportabile** in HTML e PDF — `validazione::report_html/pdf`
  + pannello Validazione (pulsanti Report HTML/PDF).
- ✅ Test unit (`validazione.rs`): ordine titoli, liste, PDF scansionato.

## Fase 10 — Metadati + segnalibri automatici ✅

- ✅ **Editor metadati** (autore, soggetto, parole chiave) nel pannello Correzione,
  scritti nel dizionario Info (UTF-16) — `core/correzione.rs`.
- ✅ **Genera segnalibri dai titoli**: costruisce l'outline `/Outlines` annidato dai
  tag Hn (testo+pagina dall'ordine MCID) — `core/correzione.rs::genera_segnalibri`
  + pulsante nel pannello Segnalibri. Test verificato (crea /Outlines).

## Fase 11 — Suggerimento Alt con AI (Claude vision) ✅

- ✅ `src-tauri/ia.rs`: chiamata raw HTTP all'API Claude (`/v1/messages`, blocco immagine
  base64) per generare il testo alternativo di una figura. Chiave API e modello salvati
  in locale (config dir), modello configurabile (default `claude-opus-4-8`).
- ✅ Pannello Correzione: configurazione chiave AI + pulsante "AI" per figura che
  compila l'Alt suggerito. Comandi `stato_ai` / `imposta_ai` / `suggerisci_alt`.
- ✅ CI: `libssl-dev` (Linux) e `openssl` (Arch) per reqwest native-tls.

## Fase 12 — OCR PDF scansionati ✅

- ✅ `core/ocr.rs`: render delle pagine + `tesseract` esterno → PDF con livello testo
  ricercabile. Rilevamento disponibilità e lingue installate. Verificato: PDF immagine
  (0 testo) → testo estraibile dopo OCR.
- ✅ Pannello Validazione: selezione lingua + "Crea PDF ricercabile". Comandi
  `ocr_info` / `esegui_ocr`.

## Fase 13 — Contrasto colori WCAG ✅

- ✅ `core/contrasto.rs`: rapporto di contrasto esatto (luminanza WCAG) + stima
  per pagina dei colori dominanti (istogramma) con esito AA testo normale/grande.
  Test: nero su bianco = 21:1, colori uguali = 1:1.
- ✅ Pannello Validazione: "Analizza" contrasto della pagina con campioni colore ed esiti.

## Fase 14 — Rifiniture ✅

- ✅ Apertura PDF via **drag & drop** nella finestra (`App.svelte`, evento Tauri).

## Fase 15 — Operazioni sulle pagine ✅

- ✅ `core/pagine.rs` (lopdf): **ruota / elimina / estrai / riordina / unisci** PDF;
  ogni operazione salva una copia. Test: elimina 3→2, estrai→2, riordina→3, unisci 3+2→5, ruota→90°.
- ✅ Pannello "Pagine": griglia con selezione (ruota/elimina/estrai/unisci) e modalità riordina.

## Fase 16 — Ricerca testo ✅

- ✅ `core/ricerca.rs`: ricerca case-insensitive per pagina con estratto di contesto.
- ✅ Pannello "Cerca" (toolbar) con risultati cliccabili che saltano alla pagina.

## Fase 17 — Rifiniture app ✅

- ✅ **File recenti** (salvati nelle impostazioni, mostrati nello stato vuoto).
- ✅ **Scorciatoie**: Ctrl+O apri, Ctrl+W chiudi scheda, Ctrl+F cerca, frecce/PgUp-PgDn pagina.
- ✅ **Tema chiaro/scuro** (persistito in localStorage, toggle in toolbar).
- ✅ **Apri esterno / Stampa**: apre il PDF nel programma di sistema (plugin opener).

## Fase 18 — Marcatura Artifact ✅

- ✅ `core/artifact.rs`: marca uno o piu' StructElem come **Artifact** e salva una
  copia. Stacca l'elemento (con il sotto-albero) dal `/K` del genitore e riscrive
  il suo marked content nei content stream da `/Ruolo <</MCID n>> BDC` a
  `/Artifact BDC` (eliminando l'MCID), cosi' gli screen reader lo saltano.
- ✅ Comando `marca_artifact` + wrapper `marcaArtifact` (api.js).
- ✅ Pannello Tag: modalità **Artifact** (caselle di selezione per gli elementi
  decorativi tipo intestazioni/piè di pagina/numeri di pagina) → salva copia.
- ✅ Test (`core/tests/artifact.rs`): marcato un paragrafo, sparisce dall'albero e
  dall'ordine di lettura, il content stream contiene `/Artifact`, il resto resta leggibile.
- ⏳ Nota: le voci nel ParentTree restano (orfane, innocue). Rifinitura possibile.

## Fase 19 — Editor tabelle ✅

- ✅ `core/tabelle.rs`: scrive sulle celle (TH/TD) l'attributo `/A <</O /Table …>>`
  con **Scope** (Row/Column/Both) per le intestazioni e **RowSpan/ColSpan** per le
  celle unite, fondendo con un attributo Table già presente. Salva una copia.
- ✅ `core/struttura.rs`: il `NodoTag` espone ora `scope`/`row_span`/`col_span`
  (letti da /A Table), per mostrare e pre-compilare lo stato nella UI.
- ✅ Comando `applica_tabella` + wrapper `applicaTabella` (api.js; i campi annidati
  usano snake_case `row_span`/`col_span` perché Tauri non li converte).
- ✅ Pannello Tag: modalità **Tabelle** con, per ogni cella, select Scope (solo TH)
  e input RowSpan/ColSpan → salva copia.
- ✅ Test (`core/tests/tabelle.rs`): Table>TR>[TH,TD], applica Scope=Column al TH e
  ColSpan=2 al TD, rilettura conferma i valori.
- ⏳ Rifinitura futura: associazione esplicita TD→TH via Headers/ID (richiede gestione
  dell'IDTree) e una regola di validazione "TH senza Scope".

## Fase 20 — Form accessibili (AcroForm) ✅

- ✅ `core/form.rs`: legge i campi dell'AcroForm (riferimento, nome /T, tipo /FT,
  tooltip /TU, pagina — mappata via /Annots), gestendo campi gerarchici e
  widget uniti. Scrive il **tooltip /TU** (etichetta letta dagli screen reader)
  sui campi e, opzionalmente, l'ordine di tabulazione strutturale `/Tabs /S` sulle
  pagine. Salva una copia.
- ✅ Comandi `leggi_form` / `applica_form` + wrapper `leggiForm` / `applicaForm`.
- ✅ Nuovo pannello **Moduli** (`PannelloModuli.svelte`): elenco campi con badge
  "manca", input per l'etichetta, opzione /Tabs, salva copia. Registrato in
  `App.svelte` e nel rail (gruppo Accessibilità).
- ✅ Test (`core/tests/form.rs`): legge un campo Tx senza /TU, imposta tooltip e
  /Tabs /S, rilettura conferma.
- ⏳ Rifinitura futura: taggare i campi nello StructTree (Form element + OBJR) per la
  piena conformità PDF/UA.

## Fase 21 — Auto-tag: rifinitura mappatura ruoli ✅

- ✅ `core/doclang.rs::mappa_ruolo`: ampliata la copertura delle etichette Docling
  verso ruoli PDF/UA dedicati invece del generico `P`:
  `document_index`→**TOC**, `reference`→**BibEntry**, `form`/`checkbox_*`→**Form**.
  Confermato che header/footer (`page_header`/`page_footer`→`Artifact`) vengono già
  esclusi dall'albero dei tag dal generatore (`docling.rs` filtra `ruolo != "Artifact"`).
- ✅ Test estesi (`doclang::test::mappa_ruoli_estesi`).
- ⏳ Restano: estrazione MCID con font CID complessi; tagging campi modulo nello StructTree.

## Fase 22 — Creazione PDF da modello (HTML + variabili + flussi JSON) ✅

- ✅ `core/modello.rs` (crate **handlebars**): modello HTML con `{{variabili}}`,
  flussi `{{#each}}` e condizioni `{{#if}}` alimentati da dati JSON → HTML →
  PDF via `printpdf::from_html`. Include modello e dati di esempio.
- ✅ Comandi `modello_esempio` / `anteprima_modello` / `genera_da_modello` + wrapper.
- ✅ Nuova vista **Creatore** (`Creatore.svelte`): editor a schede (Modello/Dati),
  anteprima HTML live in iframe, "Genera PDF…". Pulsante *Crea → Nuovo da modello*
  nel rail (sempre attivo, anche senza documento aperto).
- ✅ Test (`core/tests/modello.rs`): variabili+flussi+if, JSON invalido → errore,
  generazione PDF dall'esempio (header %PDF).

## Fase 24 — Motore di overlay: testo / immagini / filigrana ✅ (core+comando)

- ✅ `core/sovrapposizione.rs`: aggiunge a un PDF esistente, via **Pdfium**, oggetti
  pagina nativi — **testo** (font, colore, opacità, rotazione), **immagini** (PNG con
  opacità), **filigrana** (testo o immagine, centrata, su tutte le pagine). Coordinate
  in **mm con origine in alto a sinistra** (come i righelli), convertite nel sistema PDF.
- ✅ Comando `sovrapponi` (+ wrapper) con elementi e filigrana, immagini in base64,
  colore `#rrggbb`.
- ✅ Test a runtime (`core/tests/sovrapposizione.rs`, usa libpdfium): filigrana "RISERVATO"
  + testo → entrambi estraibili dal PDF salvato.
## Fase 23 — Editor visuale con righelli mm ✅

- ✅ Nuova vista **Editor** (`Editor.svelte`): canvas della pagina (render Pdfium)
  con **righelli orizzontale e verticale in millimetri**, lettura coordinate live
  sotto il puntatore, navigazione pagine. Posizionamento **a clic** degli elementi
  (origine in alto a sinistra, coerente col core). Pulsante *Crea → Editor PDF* nel rail.
- ✅ Strumenti: Testo, Immagine, Firma, Campo + sezione Filigrana. Lista elementi
  con rimozione e marcatori sovrapposti alla pagina. Salvataggio unico via `salva_editor`.

## Fase 25 — Libreria firme ✅

- ✅ `src-tauri/firme.rs`: salva le firme come PNG nella cartella dati dell'app;
  comandi `firme_elenco` / `firma_salva` / `firma_elimina`. Nell'editor: galleria
  firme con anteprima, import di una nuova firma, selezione e applicazione a clic
  (come immagine). Le firme restano disponibili tra le sessioni.

## Fase 26 — Campi modulo editabili ✅

- ✅ `core/campi.rs`: crea campi di testo AcroForm (widget) posizionati in mm,
  con tooltip `/TU`, `NeedAppearances` e font di default (DR/Helv) così i lettori
  ne disegnano l'aspetto. Comando `aggiungi_campi`; nell'editor strumento "Campo".
- ✅ Test (`core/tests/campi.rs`): crea un campo testo, riletto da `form::leggi`
  (Fase 20) con nome/tipo/tooltip/pagina corretti.
- ⏳ Rifinitura futura: checkbox/scelte (richiedono appearance streams /AP).

> Salvataggio combinato editor (`salva_editor`): applica overlay (Fase 24) e poi i
> campi (Fase 26) producendo un unico PDF.

## Fase 27 — Protezione/cifratura ✅

- ✅ `core/cifratura.rs` (lopdf encryption): `proteggi` (password apertura + proprietario,
  permessi stampa/copia/modifica/annotazioni, 128 bit; copia per accessibilità sempre
  consentita), `rimuovi_protezione` (via `load_with_password`), `e_protetto`. Genera un
  /ID se assente. Comandi `proteggi_pdf`/`sblocca_pdf`/`pdf_protetto`.

## Fase 28 — Redazione (GDPR) ✅

- ✅ `core/redazione.rs` (Pdfium): rimuove **definitivamente** gli oggetti che
  intersecano le aree indicate e li copre con un tassello nero. Comando `redigi`;
  strumento **Redazione** nell'Editor (box rossi, salvataggio dedicato).
- ⚠️ Gli oggetti rimossi sono volutamente "dimenticati" (non liberati) per evitare
  un use-after-free interno di Pdfium: piccola perdita di memoria per operazione.

## Fase 29 — Conversione immagini ✅

- ✅ `core/conversione.rs` (Pdfium + image): `pdf_a_immagini` (PNG/JPEG per pagina) e
  `immagini_a_pdf` (un'immagine per pagina). Comandi `esporta_immagini`/`crea_pdf_da_immagini`.
  Abilitato il feature `jpeg` del crate `image`.

## Fase 30 — Numerazione / intestazioni ✅

- ✅ `core/intestazioni.rs`: applica un testo a tutte le pagine in 6 posizioni, con
  segnaposto `{n}`/`{tot}` e numero iniziale; si appoggia al motore di overlay.
  Comando `numera_pagine`.

> Pannello **Strumenti PDF** (`PannelloStrumenti.svelte`): protezione, esporta/crea da
> immagini, numerazione. Redazione integrata nell'Editor. Test: `core/tests/evolutive.rs`
> (cifratura, conversione, intestazioni, redazione; i test Pdfium serializzati con mutex).

## Idee future ⏳

- ⏳ Anteprime con drag&drop per riordino, miniature nel pannello Pagine
- ⏳ Estrazione testo MCID con font CID complessi
