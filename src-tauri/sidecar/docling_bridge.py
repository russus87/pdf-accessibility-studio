#!/usr/bin/env python3
"""Ponte Docling per PDF Accessibility Studio.

Converte un PDF *non taggato* con Docling (la libreria IBM) ed emette su stdout
una struttura semantica JSON *normalizzata*, indipendente dalla versione di
Docling, che il core Rust (`pdfa_core::doclang`) trasforma in proposte di tag
PDF/UA.

Contratto di output (UTF-8, una sola riga JSON su stdout)::

    {
      "lingua": "it" | null,
      "blocchi": [
        {"label": "section_header", "testo": "...", "pagina": 1,
         "bbox": [l, t, r, b] | null, "livello": 1 | null},
        ...
      ],
      "doclang": "<serializzazione DocLang/DocTags>" | (assente),
      "doclang_fmt": "export_to_doctags" | (assente)
    }

Codici di uscita:
    0  ok
    2  uso errato (manca l'argomento)
    3  Docling non installato (`pip install docling`)
    4  errore durante la conversione

Uso::

    docling_bridge.py <input.pdf>
"""

import json
import sys


def _attr(obj, nome, default=None):
    """getattr difensivo: l'API di Docling cambia tra le versioni."""
    return getattr(obj, nome, default)


def _label_str(item):
    """Etichetta semantica come stringa (gestisce gli enum DocItemLabel)."""
    label = _attr(item, "label")
    if label is None:
        return "text"
    # Gli enum espongono il nome leggibile in `.value` (es. "section_header").
    return str(_attr(label, "value", label))


def _provenienza(item):
    """Ritorna (pagina_1based, [l, t, r, b], coord) dal primo record di prov."""
    prov = _attr(item, "prov") or []
    if not prov:
        return None, None, None
    p0 = prov[0]
    pagina = _attr(p0, "page_no")
    bbox = _attr(p0, "bbox")
    riquadro = None
    coord = None
    if bbox is not None:
        riquadro = [
            _attr(bbox, "l"),
            _attr(bbox, "t"),
            _attr(bbox, "r"),
            _attr(bbox, "b"),
        ]
        # Origine delle coordinate: enum CoordOrigin (BOTTOMLEFT/TOPLEFT).
        origine = _attr(bbox, "coord_origin")
        coord = str(_attr(origine, "value", origine)) if origine is not None else None
    return pagina, riquadro, coord


def _livello(item):
    """Livello gerarchico per i titoli, se Docling lo espone."""
    for nome in ("level", "heading_level"):
        v = _attr(item, nome)
        if isinstance(v, int):
            return v
    return None


def _lingua(doc):
    """Lingua principale, best-effort (Docling non sempre la fornisce)."""
    for nome in ("language", "lang"):
        v = _attr(doc, nome)
        if isinstance(v, str) and v.strip():
            return v.strip()
    return None


def converti(pdf_path):
    from docling.document_converter import DocumentConverter

    conv = DocumentConverter()
    risultato = conv.convert(pdf_path)
    doc = risultato.document

    blocchi = []
    # `iterate_items` restituisce gli elementi nell'ordine di lettura inferito.
    for item, _livello_albero in doc.iterate_items():
        label = _label_str(item)
        testo = (_attr(item, "text", "") or "").strip()
        pagina, bbox, coord = _provenienza(item)
        # Salta i blocchi vuoti che non sono contenitori visivi (figure/tabelle).
        if not testo and label not in ("picture", "figure", "image", "table"):
            continue
        blocchi.append(
            {
                "label": label,
                "testo": testo,
                "pagina": pagina,
                "bbox": bbox,
                "coord": coord,
                "livello": _livello(item),
            }
        )

    out = {"lingua": _lingua(doc), "blocchi": blocchi}

    # --- PoC DocLang -------------------------------------------------------
    # Docling sa esportare verso DocLang/DocTags: lo conserviamo se l'API e'
    # disponibile, ma e' solo dimostrativo (le proposte di tag derivano dai
    # blocchi qui sopra, piu' stabili tra le versioni).
    for metodo in ("export_to_doclang", "export_to_doctags"):
        fn = _attr(doc, metodo)
        if callable(fn):
            try:
                out["doclang"] = fn()
                out["doclang_fmt"] = metodo
                break
            except Exception:  # noqa: BLE001 - best-effort, non bloccante
                continue

    return out


def main(argv):
    if len(argv) < 2:
        print("uso: docling_bridge.py <input.pdf>", file=sys.stderr)
        return 2

    try:
        import docling  # noqa: F401
    except Exception as e:  # noqa: BLE001
        print(f"docling non installato: {e}", file=sys.stderr)
        return 3

    try:
        out = converti(argv[1])
    except Exception as e:  # noqa: BLE001
        print(f"conversione fallita: {e}", file=sys.stderr)
        return 4

    json.dump(out, sys.stdout, ensure_ascii=False)
    sys.stdout.flush()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
