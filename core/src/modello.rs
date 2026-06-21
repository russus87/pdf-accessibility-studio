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

/// Opzioni di impaginazione del PDF (dimensione pagina, margini, intestazione,
/// piè di pagina e numeri di pagina). Tutti i campi sono opzionali: i valori
/// assenti usano i default (A4, margini gestiti dal CSS del modello).
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct OpzioniPagina {
    pub larghezza_mm: Option<f32>,
    pub altezza_mm: Option<f32>,
    pub margine_alto: Option<f32>,
    pub margine_basso: Option<f32>,
    pub margine_sx: Option<f32>,
    pub margine_dx: Option<f32>,
    pub intestazione: Option<String>,
    pub pie_pagina: Option<String>,
    pub numeri_pagina: Option<bool>,
    pub salta_prima: Option<bool>,
    /// Altezza della banda intestazione (mm); 0/None = nessuna banda.
    pub header_mm: Option<f32>,
    /// Altezza della banda piè di pagina (mm); 0/None = nessuna banda.
    pub footer_mm: Option<f32>,
}

fn non_vuota(s: &Option<String>) -> Option<String> {
    s.as_ref().map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

impl OpzioniPagina {
    fn a_printpdf(&self) -> printpdf::GeneratePdfOptions {
        let mut o = printpdf::GeneratePdfOptions::default();
        if let Some(v) = self.larghezza_mm { o.page_width = Some(v); }
        if let Some(v) = self.altezza_mm { o.page_height = Some(v); }
        if let Some(v) = self.margine_alto { o.margin_top = Some(v); }
        if let Some(v) = self.margine_basso { o.margin_bottom = Some(v); }
        if let Some(v) = self.margine_sx { o.margin_left = Some(v); }
        if let Some(v) = self.margine_dx { o.margin_right = Some(v); }
        o.header_text = non_vuota(&self.intestazione);
        o.footer_text = non_vuota(&self.pie_pagina);
        o.show_page_numbers = self.numeri_pagina;
        o.skip_first_page = self.salta_prima;
        o
    }
}

/// Genera il PDF dal modello + dati JSON. Ritorna i byte del PDF.
pub fn genera_pdf(modello: &str, dati_json: &str) -> Risultato<Vec<u8>> {
    genera_pdf_opz(modello, dati_json, &OpzioniPagina::default())
}

/// Come `genera_pdf` ma con opzioni di impaginazione.
pub fn genera_pdf_opz(modello: &str, dati_json: &str, opz: &OpzioniPagina) -> Risultato<Vec<u8>> {
    let html = render_html(modello, dati_json)?;
    html_a_pdf_opz(&html, opz)
}

/// Stampa unione: renderizza il modello per ogni record di un array JSON e
/// ritorna un PDF (byte) per record. Il chiamante decide se salvarli separati o
/// unirli in un unico file.
pub fn genera_unione(modello: &str, dati_array_json: &str) -> Risultato<Vec<Vec<u8>>> {
    genera_unione_opz(modello, dati_array_json, &OpzioniPagina::default())
}

/// Come `genera_unione` ma con opzioni di impaginazione applicate a ogni record.
pub fn genera_unione_opz(modello: &str, dati_array_json: &str, opz: &OpzioniPagina) -> Risultato<Vec<Vec<u8>>> {
    let dati: serde_json::Value =
        serde_json::from_str(dati_array_json).map_err(|e| Errore::Io(format!("dati JSON non validi: {e}")))?;
    let record = match dati {
        serde_json::Value::Array(a) => a,
        // Un singolo oggetto viene trattato come elenco di un elemento.
        altro => vec![altro],
    };
    if record.is_empty() {
        return Err(Errore::Io("nessun record nei dati".into()));
    }

    let mut hb = Handlebars::new();
    hb.set_strict_mode(false);

    let mut out = Vec::with_capacity(record.len());
    for (i, rec) in record.iter().enumerate() {
        let html = hb
            .render_template(modello, rec)
            .map_err(|e| Errore::Io(format!("record {}: {e}", i + 1)))?;
        out.push(html_a_pdf_opz(&html, opz)?);
    }
    Ok(out)
}

/// Inietta i margini come **padding CSS** sul `body` (i margini di printpdf
/// spostano soltanto il contenuto, non lo fanno rientrare): così il testo va a
/// capo correttamente dentro l'area utile. Lo stile è messo in fondo a `<head>`
/// per prevalere su quello del modello.
fn inietta_margini(html: &str, mt: f32, mr: f32, mb: f32, ml: f32) -> String {
    let stile = format!(
        "<style>html,body{{margin:0}}body{{box-sizing:border-box;padding:{mt}mm {mr}mm {mb}mm {ml}mm}}</style>"
    );
    if let Some(pos) = html.to_lowercase().find("</head>") {
        format!("{}{}{}", &html[..pos], stile, &html[pos..])
    } else if let Some(pos) = html.to_lowercase().find("<body") {
        format!("{}{}{}", &html[..pos], stile, &html[pos..])
    } else {
        format!("{stile}{html}")
    }
}

/// Genera il PDF con bande **intestazione** e **piè di pagina** ripetute su ogni
/// pagina. Header/footer sono HTML resi a parte come strisce (largh. = area
/// contenuto, alt. = `header_mm`/`footer_mm`), rasterizzati e sovrapposti a ogni
/// pagina del corpo; i segnaposto `{{PAGENUM}}` e `{{TTLPAGES}}` sono sostituiti
/// pagina per pagina. Se header e footer sono vuoti equivale a `genera_pdf_opz`.
/// Salva in `dest` e ritorna il numero di pagine.
pub fn genera_impaginato(
    dest: &std::path::Path,
    modello: &str,
    dati: &str,
    header: &str,
    footer: &str,
    stile: &str,
    opz: &OpzioniPagina,
) -> Risultato<usize> {
    let page_w = opz.larghezza_mm.unwrap_or(210.0);
    let page_h = opz.altezza_mm.unwrap_or(297.0);
    let m_top = opz.margine_alto.unwrap_or(0.0);
    let m_bot = opz.margine_basso.unwrap_or(0.0);
    let m_sx = opz.margine_sx.unwrap_or(0.0);
    let m_dx = opz.margine_dx.unwrap_or(0.0);
    let h_mm = opz.header_mm.unwrap_or(0.0);
    let f_mm = opz.footer_mm.unwrap_or(0.0);
    let ha_header = h_mm > 0.5 && !header.trim().is_empty();
    let ha_footer = f_mm > 0.5 && !footer.trim().is_empty();

    // Corpo: stile condiviso + frammento; margini come padding CSS; quelli
    // sup/inf riservano spazio per le bande.
    let body_src = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><style>{stile}</style></head><body>{modello}</body></html>"
    );
    let body_html = render_html(&body_src, dati)?;
    let body_html = inietta_margini(
        &body_html,
        m_top + if ha_header { h_mm } else { 0.0 },
        m_dx,
        m_bot + if ha_footer { f_mm } else { 0.0 },
        m_sx,
    );
    let page_opz = OpzioniPagina {
        larghezza_mm: Some(page_w),
        altezza_mm: Some(page_h),
        ..Default::default()
    };
    let body_pdf = html_a_pdf_opz(&body_html, &page_opz)?;

    if !ha_header && !ha_footer {
        std::fs::write(dest, &body_pdf).map_err(|e| Errore::Io(e.to_string()))?;
        return Ok(crate::apri(dest)?.pagine as usize);
    }

    let dir = std::env::temp_dir().join(format!("pdfa_imp_{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| Errore::Io(e.to_string()))?;
    let body_path = dir.join("body.pdf");
    std::fs::write(&body_path, &body_pdf).map_err(|e| Errore::Io(e.to_string()))?;

    let n = crate::apri(&body_path)?.pagine as usize;
    let content_w = (page_w - m_sx - m_dx).max(10.0);
    let dpi = 200.0_f32;
    let salta = opz.salta_prima == Some(true);

    // Rende una banda (HTML) in PNG per la pagina `indice` (0-based).
    let rendi = |html: &str, banda_mm: f32, indice: usize| -> Risultato<Vec<u8>> {
        let con_stile = format!(
            "<!doctype html><html><head><meta charset=\"utf-8\"><style>{stile}</style></head><body>{html}</body></html>"
        );
        let sostituito = con_stile
            .replace("{{PAGENUM}}", &(indice + 1).to_string())
            .replace("{{TTLPAGES}}", &n.to_string());
        let strip_opz = OpzioniPagina {
            larghezza_mm: Some(content_w),
            altezza_mm: Some(banda_mm),
            ..Default::default()
        };
        let strip_html = inietta_margini(&sostituito, 0.0, 0.0, 0.0, 0.0);
        let strip_pdf = html_a_pdf_opz(&strip_html, &strip_opz)?;
        let strip_path = dir.join(format!("strip_{indice}.pdf"));
        std::fs::write(&strip_path, &strip_pdf).map_err(|e| Errore::Io(e.to_string()))?;
        let px = (((content_w / 25.4) * dpi).round() as i32).max(64);
        crate::documento::render_pagina(&strip_path, 0, px)
    };

    let mk = |pagina: usize, png: Vec<u8>, y_mm: f32, banda_mm: f32| crate::sovrapposizione::Elemento::Immagine {
        pagina: pagina as u16,
        x_mm: m_sx,
        y_mm,
        larghezza_mm: content_w,
        altezza_mm: banda_mm,
        png,
        opacita: 255,
        rotazione: 0.0,
    };

    let mut elementi = Vec::new();
    let token = |s: &str| s.contains("{{PAGENUM}}") || s.contains("{{TTLPAGES}}");
    let per_pagina = (ha_header && token(header)) || (ha_footer && token(footer));

    if per_pagina {
        for p in 0..n {
            if salta && p == 0 { continue; }
            if ha_header { elementi.push(mk(p, rendi(header, h_mm, p)?, m_top, h_mm)); }
            if ha_footer { elementi.push(mk(p, rendi(footer, f_mm, p)?, page_h - m_bot - f_mm, f_mm)); }
        }
    } else {
        let head_png = if ha_header { Some(rendi(header, h_mm, 0)?) } else { None };
        let foot_png = if ha_footer { Some(rendi(footer, f_mm, 0)?) } else { None };
        for p in 0..n {
            if salta && p == 0 { continue; }
            if let Some(ref png) = head_png { elementi.push(mk(p, png.clone(), m_top, h_mm)); }
            if let Some(ref png) = foot_png { elementi.push(mk(p, png.clone(), page_h - m_bot - f_mm, f_mm)); }
        }
    }

    crate::sovrapposizione::applica(&body_path, dest, &elementi, None)?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(n)
}

/// Converte una stringa HTML completa in PDF con il renderer di printpdf.
pub fn html_a_pdf(html: &str) -> Risultato<Vec<u8>> {
    html_a_pdf_opz(html, &OpzioniPagina::default())
}

/// Come `html_a_pdf` ma con opzioni di impaginazione.
pub fn html_a_pdf_opz(html: &str, opz: &OpzioniPagina) -> Risultato<Vec<u8>> {
    let immagini = BTreeMap::new();
    let font = BTreeMap::new();
    let opzioni = opz.a_printpdf();
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
  body { font-family: sans-serif; font-size: 12pt; margin: 0; color: #111; }
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
