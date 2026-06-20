//! Test del motore template (Fase 22): variabili, flussi e generazione PDF.

#[test]
fn render_variabili_e_flussi() {
    let modello = "Ciao {{nome}}! Ordini: {{#each ordini}}[{{prodotto}} x{{q}}]{{/each}}{{#if vip}} (VIP){{/if}}";
    let dati = r#"{
        "nome": "Anna",
        "vip": true,
        "ordini": [ {"prodotto":"Penna","q":3}, {"prodotto":"Quaderno","q":1} ]
    }"#;
    let html = pdfa_core::modello::render_html(modello, dati).expect("render");
    assert!(html.contains("Ciao Anna!"), "variabile sostituita: {html}");
    assert!(html.contains("[Penna x3]"), "primo elemento del flusso: {html}");
    assert!(html.contains("[Quaderno x1]"), "secondo elemento del flusso: {html}");
    assert!(html.contains("(VIP)"), "blocco condizionale: {html}");
}

#[test]
fn dati_json_invalidi_danno_errore() {
    let r = pdfa_core::modello::render_html("{{x}}", "{ non valido");
    assert!(r.is_err(), "JSON malformato deve dare errore");
}

#[test]
fn genera_pdf_dal_modello_di_esempio() {
    let pdf = pdfa_core::modello::genera_pdf(
        pdfa_core::modello::modello_esempio(),
        pdfa_core::modello::dati_esempio(),
    )
    .expect("genera pdf");
    assert!(pdf.len() > 100, "PDF non vuoto");
    assert_eq!(&pdf[0..4], b"%PDF", "intestazione PDF valida");
}
