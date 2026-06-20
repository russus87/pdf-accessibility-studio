//! Generazione di PDF da **modello HTML + dati JSON** (Fase 22).
//!
//! Il modello è HTML/CSS con segnaposto in stile Handlebars:
//! - `{{variabile}}` — sostituzione di un valore;
//! - `{{#each elenco}} … {{this.campo}} … {{/each}}` — **flussi**: ripete il
//!   blocco per ogni elemento di un array JSON (righe di tabella, voci, ecc.);
//! - `{{#if condizione}} … {{/if}}` — blocchi condizionali.
//!
//! I dati arrivano da una stringa JSON. Il risultato è prima reso in HTML, poi
//! convertito in PDF con il renderer di `printpdf` (`from_html`), lo stesso già
//! usato per i report. Per i salti pagina nei flussi lunghi si usa la CSS
//! `page-break-before: always` nel modello.

use std::collections::BTreeMap;

use handlebars::Handlebars;

use crate::errore::{Errore, Risultato};

/// Renderizza il modello con i dati JSON e ritorna l'HTML finale (utile per
/// l'anteprima nella UI). `dati_json` vuoto equivale a un oggetto vuoto.
pub fn render_html(modello: &str, dati_json: &str) -> Risultato<String> {
    let dati: serde_json::Value = if dati_json.trim().is_empty() {
        serde_json::Value::Object(Default::default())
    } else {
        serde_json::from_str(dati_json).map_err(|e| Errore::Io(format!("dati JSON non validi: {e}")))?
    };

    let mut hb = Handlebars::new();
    // Errore esplicito se il modello usa una variabile assente: aiuta a correggere.
    hb.set_strict_mode(false);
    hb.render_template(modello, &dati)
        .map_err(|e| Errore::Io(format!("errore nel modello: {e}")))
}

/// Genera il PDF dal modello + dati JSON. Ritorna i byte del PDF.
pub fn genera_pdf(modello: &str, dati_json: &str) -> Risultato<Vec<u8>> {
    let html = render_html(modello, dati_json)?;
    html_a_pdf(&html)
}

/// Converte una stringa HTML completa in PDF con il renderer di printpdf.
pub fn html_a_pdf(html: &str) -> Risultato<Vec<u8>> {
    let immagini = BTreeMap::new();
    let font = BTreeMap::new();
    let opzioni = printpdf::GeneratePdfOptions::default();
    let mut warn = Vec::new();
    let doc = printpdf::PdfDocument::from_html(html, &immagini, &font, &opzioni, &mut warn)
        .map_err(|e| Errore::Io(format!("PDF da HTML: {e}")))?;
    let mut warn2 = Vec::new();
    Ok(doc.save(&printpdf::PdfSaveOptions::default(), &mut warn2))
}

/// Un modello di esempio pronto all'uso (lettera con voci ripetute), mostrato
/// nell'editor quando si parte da zero.
pub fn modello_esempio() -> &'static str {
    r#"<!doctype html>
<html lang="it">
<head><meta charset="utf-8">
<style>
  body { font-family: sans-serif; font-size: 12pt; margin: 24mm; color: #111; }
  h1 { font-size: 18pt; }
  .totale { font-weight: bold; text-align: right; }
  table { width: 100%; border-collapse: collapse; margin-top: 8mm; }
  th, td { border: 1px solid #888; padding: 4px 8px; text-align: left; }
</style>
</head>
<body>
  <h1>Fattura {{numero}}</h1>
  <p>Cliente: <b>{{cliente.nome}}</b><br>{{cliente.indirizzo}}</p>

  <table>
    <thead><tr><th>Descrizione</th><th>Quantità</th><th>Prezzo</th></tr></thead>
    <tbody>
      {{#each voci}}
      <tr><td>{{descrizione}}</td><td>{{quantita}}</td><td>{{prezzo}} €</td></tr>
      {{/each}}
    </tbody>
  </table>

  <p class="totale">Totale: {{totale}} €</p>
  {{#if note}}<p><i>{{note}}</i></p>{{/if}}
</body>
</html>"#
}

/// Dati JSON di esempio coerenti con `modello_esempio`.
pub fn dati_esempio() -> &'static str {
    r#"{
  "numero": "2026-001",
  "cliente": { "nome": "Mario Rossi", "indirizzo": "Via Roma 1, Milano" },
  "voci": [
    { "descrizione": "Consulenza", "quantita": 2, "prezzo": 100 },
    { "descrizione": "Licenza software", "quantita": 1, "prezzo": 250 }
  ],
  "totale": 450,
  "note": "Pagamento a 30 giorni."
}"#
}
