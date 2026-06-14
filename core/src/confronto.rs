//! Confronto tra due PDF (Fase 4): testuale, immagine pixel-to-pixel, per tag.
//! Più la generazione dei report in HTML e PDF dallo stesso modello di dati.

use std::path::Path;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use image::Rgba;
use serde::Serialize;
use similar::{ChangeTag, TextDiff};

use crate::documento::{render_immagine, testo_pagine};
use crate::errore::{Errore, Risultato};
use crate::struttura::{analizza, NodoTag};

/// Una riga di diff: tipo = "uguale" | "aggiunta" | "rimossa".
#[derive(Debug, Clone, Serialize)]
pub struct RigaDiff {
    pub tipo: String,
    pub testo: String,
}

/// Risultato di un diff a righe (testo o tag).
#[derive(Debug, Clone, Serialize)]
pub struct Diff {
    pub uguali: usize,
    pub aggiunte: usize,
    pub rimosse: usize,
    pub identici: bool,
    pub righe: Vec<RigaDiff>,
}

/// Risultato del confronto pixel-to-pixel di una pagina.
#[derive(Debug, Clone, Serialize)]
pub struct ConfrontoImmagine {
    pub pagina: i32,
    pub larghezza: u32,
    pub altezza: u32,
    pub pixel_totali: u64,
    pub pixel_diversi: u64,
    pub percentuale: f64,
    /// Heatmap delle differenze come PNG in base64 (data URL lato UI).
    pub diff_png_base64: String,
}

/// Confronto testuale tra due PDF (tutto il testo, pagina dopo pagina).
pub fn confronta_testo(a: &Path, b: &Path) -> Risultato<Diff> {
    let ta = testo_pagine(a)?.join("\n");
    let tb = testo_pagine(b)?.join("\n");
    Ok(diff_righe(&ta, &tb))
}

/// Confronto della struttura dei tag (sequenza ruoli indentata).
pub fn confronta_tag(a: &Path, b: &Path) -> Risultato<Diff> {
    let mut va = Vec::new();
    let mut vb = Vec::new();
    appiattisci(&analizza(a)?.radice, &mut va, 0);
    appiattisci(&analizza(b)?.radice, &mut vb, 0);
    Ok(diff_righe(&va.join("\n"), &vb.join("\n")))
}

/// Confronto pixel-to-pixel di una pagina (indice 0-based) renderizzata alla
/// stessa larghezza. Produce statistiche e una heatmap delle differenze.
pub fn confronta_immagine(a: &Path, b: &Path, pagina: i32, larghezza: i32) -> Risultato<ConfrontoImmagine> {
    let ia = render_immagine(a, pagina, larghezza)?.to_rgba8();
    let ib = render_immagine(b, pagina, larghezza)?.to_rgba8();

    let w = ia.width().max(ib.width());
    let h = ia.height().max(ib.height());
    let mut diff = image::RgbaImage::new(w, h);
    let mut diversi: u64 = 0;

    for y in 0..h {
        for x in 0..w {
            let pa = if x < ia.width() && y < ia.height() { Some(*ia.get_pixel(x, y)) } else { None };
            let pb = if x < ib.width() && y < ib.height() { Some(*ib.get_pixel(x, y)) } else { None };
            let differente = pa != pb;
            if differente {
                diversi += 1;
                diff.put_pixel(x, y, Rgba([220, 40, 40, 255]));
            } else {
                // Originale schiarito, cosi' il rosso delle differenze risalta.
                let base = pa.unwrap_or(Rgba([255, 255, 255, 255]));
                diff.put_pixel(
                    x,
                    y,
                    Rgba([
                        128 + base[0] / 2,
                        128 + base[1] / 2,
                        128 + base[2] / 2,
                        255,
                    ]),
                );
            }
        }
    }

    let totali = w as u64 * h as u64;
    let mut buf = Vec::new();
    image::DynamicImage::ImageRgba8(diff)
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)?;

    Ok(ConfrontoImmagine {
        pagina,
        larghezza: w,
        altezza: h,
        pixel_totali: totali,
        pixel_diversi: diversi,
        percentuale: if totali == 0 { 0.0 } else { diversi as f64 * 100.0 / totali as f64 },
        diff_png_base64: STANDARD.encode(&buf),
    })
}

// --- Report ------------------------------------------------------------------

/// Genera il report di confronto (testo + tag) in HTML autoconsistente.
pub fn report_html(nome_a: &str, nome_b: &str, testo: &Diff, tag: &Diff) -> String {
    let mut h = String::new();
    h.push_str("<!doctype html><html lang=\"it\"><head><meta charset=\"utf-8\">");
    h.push_str("<title>Report confronto PDF</title><style>");
    h.push_str(
        "body{font-family:Segoe UI,system-ui,sans-serif;margin:32px;color:#222}
         h1{font-size:22px} h2{margin-top:28px;border-bottom:2px solid #e0922f;padding-bottom:4px}
         .meta{color:#555} .stat{display:inline-block;margin-right:16px;font-weight:bold}
         .diff{border:1px solid #ddd;border-radius:6px;overflow:auto;font-family:ui-monospace,monospace;font-size:13px}
         .riga{padding:1px 8px;white-space:pre-wrap}
         .uguale{color:#444} .aggiunta{background:#e6ffed;color:#063} .rimossa{background:#ffeef0;color:#900}
         .vuoto{color:#888;padding:8px}",
    );
    h.push_str("</style></head><body>");
    h.push_str("<h1>Report di confronto PDF</h1>");
    h.push_str(&format!(
        "<p class=\"meta\">A: <b>{}</b> &nbsp;|&nbsp; B: <b>{}</b></p>",
        esc(nome_a),
        esc(nome_b)
    ));

    sezione(&mut h, "Confronto testuale", testo);
    sezione(&mut h, "Confronto dei tag (struttura)", tag);

    h.push_str("</body></html>");
    h
}

/// Genera il report PDF a partire dallo stesso HTML (via il renderer di printpdf).
pub fn report_pdf(nome_a: &str, nome_b: &str, testo: &Diff, tag: &Diff) -> Risultato<Vec<u8>> {
    use std::collections::BTreeMap;
    let html = report_html(nome_a, nome_b, testo, tag);
    let immagini = BTreeMap::new();
    let font = BTreeMap::new();
    let opzioni = printpdf::GeneratePdfOptions::default();
    let mut warn = Vec::new();
    let doc = printpdf::PdfDocument::from_html(&html, &immagini, &font, &opzioni, &mut warn)
        .map_err(|e| Errore::Io(format!("report PDF: {e}")))?;
    let mut warn2 = Vec::new();
    Ok(doc.save(&printpdf::PdfSaveOptions::default(), &mut warn2))
}

fn sezione(h: &mut String, titolo: &str, d: &Diff) {
    h.push_str(&format!("<h2>{}</h2>", esc(titolo)));
    if d.identici {
        h.push_str("<p class=\"stat\" style=\"color:#063\">Identici (nessuna differenza)</p>");
        return;
    }
    h.push_str(&format!(
        "<p><span class=\"stat\">Uguali: {}</span><span class=\"stat\" style=\"color:#063\">Aggiunte: {}</span><span class=\"stat\" style=\"color:#900\">Rimosse: {}</span></p>",
        d.uguali, d.aggiunte, d.rimosse
    ));
    h.push_str("<div class=\"diff\">");
    for r in &d.righe {
        // Mostra solo le righe non identiche per tenere il report leggibile.
        if r.tipo == "uguale" {
            continue;
        }
        let segno = if r.tipo == "aggiunta" { "+" } else { "−" };
        h.push_str(&format!(
            "<div class=\"riga {}\">{} {}</div>",
            r.tipo,
            segno,
            esc(&r.testo)
        ));
    }
    h.push_str("</div>");
}

// --- Helper -------------------------------------------------------------------

/// Diff a righe tra due testi.
fn diff_righe(a: &str, b: &str) -> Diff {
    let td = TextDiff::from_lines(a, b);
    let mut righe = Vec::new();
    let (mut u, mut agg, mut rim) = (0, 0, 0);
    for change in td.iter_all_changes() {
        let tipo = match change.tag() {
            ChangeTag::Equal => {
                u += 1;
                "uguale"
            }
            ChangeTag::Insert => {
                agg += 1;
                "aggiunta"
            }
            ChangeTag::Delete => {
                rim += 1;
                "rimossa"
            }
        };
        righe.push(RigaDiff {
            tipo: tipo.into(),
            testo: change.value().trim_end_matches('\n').to_string(),
        });
    }
    Diff {
        uguali: u,
        aggiunte: agg,
        rimosse: rim,
        identici: agg == 0 && rim == 0,
        righe,
    }
}

/// Appiattisce l'albero dei tag in righe indentate "ruolo [alt: ...]".
fn appiattisci(nodi: &[NodoTag], out: &mut Vec<String>, prof: usize) {
    for n in nodi {
        let mut riga = "  ".repeat(prof);
        riga.push_str(&n.ruolo);
        if let Some(a) = &n.alt {
            riga.push_str(&format!(" [alt: {a}]"));
        }
        out.push(riga);
        appiattisci(&n.figli, out, prof + 1);
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
