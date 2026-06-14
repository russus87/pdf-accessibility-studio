# PDF Accessibility Studio

App desktop (Rust + Tauri + Svelte) per lavorare con i **PDF accessibili**:
visore a schede, validazione dell'accessibilita', sintesi vocale stile screen
reader, confronto tra PDF (testo / pixel / tag) con report, ed export dei tag.

> Stato attuale: **Fase 1** — visore multi-scheda con rendering delle pagine e
> build automatiche multipiattaforma. Le feature avanzate arrivano nelle fasi
> successive (vedi `IMPLEMENTAZIONI.md`).

## Architettura

- `core/` — logica pura in Rust (apertura, rendering; in futuro validazione,
  confronto, TTS). Non dipende da Tauri: riusabile anche lato server/web.
- `src-tauri/` — app desktop Tauri: stato condiviso e comandi che la UI invoca.
- `src/` — interfaccia Svelte 5 (barra schede, toolbar, visore).

Il rendering usa **Pdfium** (libreria nativa). Non e' versionata: si scarica con
lo script `scripts/fetch-pdfium.sh` (la CI la scarica e impacchetta da sola).

## Sviluppo

```bash
npm install
bash scripts/fetch-pdfium.sh pdfium   # scarica libpdfium nella root (per il dev)
npm run tauri dev
```

L'app cerca Pdfium in: `$PDFIUM_LIB_PATH`, accanto all'eseguibile (`pdfium/`),
nelle risorse del bundle, in `./pdfium`, infine tra le librerie di sistema.

## Build e release

Push di un tag `vX.Y.Z` ⇒ GitHub Actions compila Windows/macOS/Linux (+ pacchetto
Arch `.pkg.tar.zst`) e pubblica gli artefatti nella release di questo repo.
Essendo il repo pubblico, l'auto-update firmato legge direttamente da qui (niente
repo separato). Serve solo il secret `TAURI_SIGNING_PRIVATE_KEY` (+ `..._PASSWORD`)
per la firma degli aggiornamenti.
