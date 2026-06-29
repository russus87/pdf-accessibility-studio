// Stato globale delle schede aperte, con runes Svelte 5.
// Una scheda puo' essere un PDF (tipo "pdf") oppure un foglio di creazione da
// modello HTML (tipo "creatore"). La vista centrale dipende dal tipo della
// scheda attiva e da `modoCentro`; i pannelli laterali da `pannello`.
import { apriPdf, chiudiDocumento, scegliPdf } from "./api.js";

// Estrae il nome file da un percorso completo (Win o Unix).
function nomeFile(percorso) {
  const parti = percorso.split(/[\\/]/);
  return parti[parti.length - 1] || percorso;
}

class GestoreSchede {
  /** @type {Array<object>} schede aperte (PDF o creatore) */
  schede = $state([]);
  /** id della scheda attiva */
  attiva = $state(null);
  /** messaggio di errore eventuale */
  errore = $state(null);
  /** pannello laterale attivo (sidebar) o null:
   *  "anteprime"|"pagine"|"metadati"|"valida"|"correggi"|"tag"|"autotag"|
   *  "moduli"|"indice"|"leggi"|"ai"|"cerca"|"strumenti"|"libreria" */
  pannello = $state(null);
  /** modalità richiesta per il pannello appena aperto (consumata dal pannello,
   *  es. PannelloTag "alt"|"ruoli"|"tabelle"); poi azzerata. */
  pannelloModo = $state(null);
  /** vista centrale speciale per le schede PDF: null | "editor" | "confronta" */
  modoCentro = $state(null);
  /** mostra il righello metrico e lo strumento di misura sul visore */
  misura = $state(false);
  /** evidenziazioni da disegnare sul visore: { id, pagina, rettangoli, tipo } */
  evidenziazioni = $state(null);
  /** mostra la procedura guidata di remediation accessibilità */
  guida = $state(false);

  _contatore = 0;

  /** Apre/chiude un pannello laterale (toggle). */
  mostraPannello(nome) {
    this.pannello = this.pannello === nome ? null : nome;
  }

  /** Apre un pannello laterale impostandone la modalità iniziale (per i flussi
   *  guidati, es. dalla validazione → Tag in modalità Alt). */
  apriPannello(nome, modo = null) {
    this.pannello = nome;
    this.pannelloModo = modo;
  }

  /** Imposta (o azzera) la vista centrale speciale per i PDF. */
  mostraCentro(nome) {
    this.modoCentro = this.modoCentro === nome ? null : nome;
  }

  /** Evidenzia dei rettangoli sulla pagina data della scheda attiva e ci salta. */
  evidenzia(pagina, rettangoli, tipo = "ricerca") {
    const s = this.schedaAttiva;
    if (!s || pagina == null) return;
    this.evidenziazioni = { id: s.id, pagina, rettangoli: rettangoli || [], tipo };
    this.vaiAPagina(pagina);
  }

  /** Rimuove le evidenziazioni correnti. */
  pulisciEvidenziazioni() {
    this.evidenziazioni = null;
  }

  /** Imposta la pagina corrente della scheda attiva (usato da anteprime e tag). */
  vaiAPagina(indice) {
    const s = this.schedaAttiva;
    if (s && s.tipo === "pdf" && indice != null && indice >= 0 && indice < s.pagine) s.pagina = indice;
  }

  /** Apre uno o piu' PDF scelti dall'utente. */
  async apriDaDialogo() {
    this.errore = null;
    let percorsi;
    try {
      percorsi = await scegliPdf();
    } catch (e) {
      this.errore = String(e);
      return;
    }
    for (const percorso of percorsi) {
      await this.apri(percorso);
    }
  }

  /** Apre un singolo PDF dato il percorso. */
  async apri(percorso) {
    try {
      const info = await apriPdf(percorso);
      this.schede.push({
        id: info.id,
        tipo: "pdf",
        percorso: info.percorso,
        nome: info.titolo || nomeFile(info.percorso),
        pagine: info.pagine,
        titolo: info.titolo,
        // Stato di visualizzazione per scheda: pagina corrente (0-based) e
        // larghezza di rendering in pixel (livello di zoom).
        pagina: 0,
        zoom: 900,
      });
      this.attiva = info.id;
      this.modoCentro = null;
    } catch (e) {
      this.errore = `Impossibile aprire ${nomeFile(percorso)}: ${e}`;
    }
  }

  /** Crea una nuova scheda "Nuovo da modello" e la rende attiva. */
  nuovoCreatore() {
    const id = "crea-" + ++this._contatore;
    this.schede.push({
      id,
      tipo: "creatore",
      nome: "Nuovo documento",
      // Contenuto persistente della scheda (l'editor li popola dall'esempio).
      modello: "", // corpo (frammento HTML)
      header: "", // banda intestazione (frammento HTML)
      footer: "", // banda piè di pagina (frammento HTML)
      stile: "", // CSS condiviso tra le regioni
      dati: "",
      // Opzioni di impaginazione del PDF (camelCase = struct Rust OpzioniPagina).
      opzioni: {
        larghezzaMm: 210,
        altezzaMm: 297,
        margineAlto: 20,
        margineBasso: 18,
        margineSx: 18,
        margineDx: 18,
        intestazione: "",
        piePagina: "",
        numeriPagina: false,
        saltaPrima: false,
        headerMm: 0, // 0 = banda intestazione disattivata
        footerMm: 0, // 0 = banda piè di pagina disattivata
      },
    });
    this.attiva = id;
    this.modoCentro = null;
  }

  /** Chiude la scheda con l'id dato. */
  async chiudi(id) {
    const i = this.schede.findIndex((s) => s.id === id);
    if (i === -1) return;
    const scheda = this.schede[i];
    this.schede.splice(i, 1);
    if (scheda.tipo === "pdf") {
      try {
        await chiudiDocumento(id);
      } catch (_) {
        // se il backend non la conosce, pazienza
      }
    }
    if (this.attiva === id) {
      const prossima = this.schede[i] || this.schede[i - 1];
      this.attiva = prossima ? prossima.id : null;
    }
  }

  /** Ritorna la scheda attualmente attiva (o undefined). */
  get schedaAttiva() {
    return this.schede.find((s) => s.id === this.attiva);
  }

  /** Ritorna la scheda attiva solo se e' un PDF (altrimenti undefined). */
  get pdfAttivo() {
    const s = this.schedaAttiva;
    return s && s.tipo === "pdf" ? s : undefined;
  }

  /** Numero di PDF aperti (serve a "Confronta"). */
  get numeroPdf() {
    return this.schede.filter((s) => s.tipo === "pdf").length;
  }
}

export const schede = new GestoreSchede();
