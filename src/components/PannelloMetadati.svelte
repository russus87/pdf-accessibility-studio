<script>
  // Metadati del documento: mostra e permette di modificare titolo, autore,
  // soggetto, parole chiave, creatore, produttore, lingua e DisplayDocTitle.
  // Al salvataggio chiede se sovrascrivere l'originale o creare una copia.
  import { schede } from "../lib/schede.svelte.js";
  import { metadati, salvaMetadati } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);

  let caricamento = $state(false);
  let errore = $state(null);
  let esito = $state(null);
  let salvataggio = $state(false);

  // Sola lettura (informativi).
  let creato = $state(null);
  let modificato = $state(null);
  let taggato = $state(false);

  // Campi editabili.
  let m = $state(vuoto());
  function vuoto() {
    return {
      titolo: "",
      autore: "",
      soggetto: "",
      paroleChiave: "",
      creatore: "",
      produttore: "",
      lang: "",
      displayDocTitle: false,
    };
  }

  $effect(() => {
    if (!s) return;
    const id = s.id;
    let annullato = false;
    caricamento = true;
    errore = null;
    esito = null;
    metadati(id)
      .then((d) => {
        if (annullato) return;
        m = {
          titolo: d.titolo || "",
          autore: d.autore || "",
          soggetto: d.soggetto || "",
          paroleChiave: d.parole_chiave || "",
          creatore: d.creatore || "",
          produttore: d.produttore || "",
          lang: d.lang || "",
          displayDocTitle: !!d.display_doc_title,
        };
        creato = d.creato;
        modificato = d.modificato;
        taggato = d.taggato;
      })
      .catch((e) => !annullato && (errore = String(e)))
      .finally(() => !annullato && (caricamento = false));
    return () => (annullato = true);
  });

  async function salva() {
    if (!s) return;
    esito = null;
    salvataggio = true;
    try {
      const r = await salvaMetadati(s.id, s.percorso, { ...m });
      if (r) {
        esito = r.sovrascritto
          ? "Metadati salvati nell'originale."
          : `Copia salvata: ${r.destinazione}`;
        // Se ho aggiornato il titolo dell'originale, aggiorna anche la scheda.
        if (r.sovrascritto && m.titolo.trim()) s.nome = m.titolo.trim();
      }
    } catch (e) {
      esito = `Errore: ${e}`;
    } finally {
      salvataggio = false;
    }
  }
</script>

<div class="pannello">
  <header>
    <h3>Metadati del documento</h3>
  </header>

  {#if caricamento}
    <p class="info">Lettura metadati…</p>
  {:else if errore}
    <p class="err">{errore}</p>
  {:else if s}
    {#if esito}<p class="esito">{esito}</p>{/if}

    <div class="form">
      <label>
        Titolo
        <input type="text" bind:value={m.titolo} placeholder="(nessun titolo)" />
      </label>
      <label>
        Autore
        <input type="text" bind:value={m.autore} />
      </label>
      <label>
        Soggetto
        <input type="text" bind:value={m.soggetto} />
      </label>
      <label>
        Parole chiave
        <input type="text" bind:value={m.paroleChiave} placeholder="separate da virgola" />
      </label>
      <label>
        Lingua
        <input type="text" bind:value={m.lang} placeholder="es. it, en-US" />
      </label>
      <div class="due">
        <label>
          Creatore
          <input type="text" bind:value={m.creatore} />
        </label>
        <label>
          Produttore
          <input type="text" bind:value={m.produttore} />
        </label>
      </div>
      <label class="check">
        <input type="checkbox" bind:checked={m.displayDocTitle} />
        Mostra il titolo nella barra del lettore (DisplayDocTitle)
      </label>
    </div>

    <dl class="info-extra">
      <div><dt>Creato</dt><dd>{creato || "—"}</dd></div>
      <div><dt>Modificato</dt><dd>{modificato || "—"}</dd></div>
      <div><dt>Taggato</dt><dd>{taggato ? "sì" : "no"}</dd></div>
    </dl>

    <button class="salva" onclick={salva} disabled={salvataggio}>
      {salvataggio ? "Salvataggio…" : "Salva metadati…"}
    </button>
  {:else}
    <p class="info">Apri un PDF per vederne i metadati.</p>
  {/if}
</div>

<style>
  .pannello {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: auto;
  }
  header {
    padding: 12px 14px;
    border-bottom: 1px solid var(--bordo);
  }
  h3 {
    margin: 0;
    font-size: 15px;
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
  }
  .form label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--testo-soft);
  }
  .due {
    display: flex;
    gap: 10px;
  }
  .due label {
    flex: 1;
  }
  .form input[type="text"] {
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 7px 9px;
    font-size: 13px;
  }
  .form input[type="text"]:focus {
    outline: none;
    border-color: var(--accento);
  }
  label.check {
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }
  label.check input {
    accent-color: var(--accento);
  }
  .info-extra {
    margin: 0;
    padding: 0 14px 8px;
    font-size: 12px;
    color: var(--testo-soft);
  }
  .info-extra div {
    display: flex;
    justify-content: space-between;
    padding: 3px 0;
    border-bottom: 1px solid var(--bordo);
  }
  .info-extra dt {
    font-weight: 600;
  }
  .info-extra dd {
    margin: 0;
  }
  .salva {
    margin: 8px 14px 18px;
    background: var(--accento);
    color: #fff;
    border: none;
    border-radius: 8px;
    padding: 10px 16px;
    cursor: pointer;
  }
  .salva:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .info,
  .err,
  .esito {
    padding: 12px 14px;
    color: var(--testo-soft);
  }
  .err {
    color: var(--errore);
  }
  .esito {
    color: #7ad08f;
    word-break: break-word;
  }
</style>
