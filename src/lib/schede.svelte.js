// Stato globale delle schede aperte (una scheda = un PDF), con runes Svelte 5.
import { apriPdf, chiudiDocumento, scegliPdf } from "./api.js";

// Estrae il nome file da un percorso completo (Win o Unix).
function nomeFile(percorso) {
  const parti = percorso.split(/[\\/]/);
  return parti[parti.length - 1] || percorso;
}

class GestoreSchede {
  /** @type {Array<{id:string, percorso:string, nome:string, pagine:number, titolo:string|null}>} */
  schede = $state([]);
  /** id della scheda attiva */
  attiva = $state(null);
  /** messaggio di errore eventuale */
  errore = $state(null);

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
    } catch (e) {
      this.errore = `Impossibile aprire ${nomeFile(percorso)}: ${e}`;
    }
  }

  /** Chiude la scheda con l'id dato. */
  async chiudi(id) {
    const i = this.schede.findIndex((s) => s.id === id);
    if (i === -1) return;
    this.schede.splice(i, 1);
    try {
      await chiudiDocumento(id);
    } catch (_) {
      // se il backend non la conosce, pazienza
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
}

export const schede = new GestoreSchede();
