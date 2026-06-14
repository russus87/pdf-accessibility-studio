<script>
  // Correzione assistita: imposta lingua, titolo e "mostra titolo", poi salva
  // una copia corretta del PDF (l'originale resta intatto).
  import { schede } from "../lib/schede.svelte.js";
  import { correggi, alberoTag } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);

  let lang = $state("it-IT");
  let titolo = $state("");
  let displayDocTitle = $state(true);
  let esito = $state(null);
  let inCorso = $state(false);

  // Figure presenti nel documento e testo Alt modificabile (ref -> testo).
  let figure = $state([]);
  let altValori = $state({});

  // Prefill del titolo con quello attuale della scheda quando cambia.
  $effect(() => {
    titolo = s?.titolo || "";
  });

  // Carica le figure (con riferimento) per l'editor Alt.
  $effect(() => {
    if (!s) return;
    const id = s.id;
    let annullato = false;
    alberoTag(id)
      .then((info) => {
        if (annullato) return;
        const trovate = [];
        const valori = {};
        const cammina = (nodi) => {
          for (const n of nodi) {
            if (n.ruolo === "Figure" && n.riferimento) {
              trovate.push({ riferimento: n.riferimento, pagina: n.pagina, altAttuale: n.alt });
              valori[n.riferimento] = n.alt || "";
            }
            cammina(n.figli);
          }
        };
        cammina(info.radice);
        figure = trovate;
        altValori = valori;
      })
      .catch(() => {
        figure = [];
      });
    return () => (annullato = true);
  });

  const lingue = [
    ["it-IT", "Italiano"],
    ["en-US", "Inglese (US)"],
    ["en-GB", "Inglese (UK)"],
    ["fr-FR", "Francese"],
    ["de-DE", "Tedesco"],
    ["es-ES", "Spagnolo"],
  ];

  async function applica() {
    if (!s) return;
    esito = null;
    inCorso = true;
    try {
      // Includi solo gli Alt effettivamente compilati.
      const alt = Object.entries(altValori)
        .filter(([, testo]) => testo && testo.trim())
        .map(([riferimento, testo]) => ({ riferimento, testo: testo.trim() }));
      const dest = await correggi(s.id, { lang, titolo, displayDocTitle, alt });
      if (dest) {
        esito = { ok: true, dest };
      }
    } catch (e) {
      esito = { ok: false, msg: String(e) };
    } finally {
      inCorso = false;
    }
  }

  function apriCorretto() {
    if (esito?.ok) schede.apri(esito.dest);
  }
</script>

<div class="pannello">
  <header><h3>Correzione assistita</h3></header>

  <p class="nota">
    Applica correzioni a livello documento e salva una <b>copia corretta</b>.
    L'originale non viene modificato.
  </p>

  <label>
    Lingua del documento
    <select bind:value={lang}>
      {#each lingue as [code, nome]}<option value={code}>{nome} ({code})</option>{/each}
    </select>
  </label>

  <label>
    Titolo
    <input type="text" bind:value={titolo} placeholder="Titolo del documento" />
  </label>

  <label class="check">
    <input type="checkbox" bind:checked={displayDocTitle} />
    Mostra il titolo nella barra (DisplayDocTitle)
  </label>

  {#if figure.length}
    <div class="figure">
      <div class="titolo-sez">Testo alternativo immagini ({figure.length})</div>
      {#each figure as f, i}
        <label class="alt">
          <span class="capo">
            Figura {i + 1}
            {#if f.pagina != null}<button class="vai" onclick={() => schede.vaiAPagina(f.pagina)}>pag. {f.pagina + 1}</button>{/if}
            {#if !f.altAttuale}<span class="manca">manca</span>{/if}
          </span>
          <input type="text" bind:value={altValori[f.riferimento]} placeholder="Descrizione dell'immagine" />
        </label>
      {/each}
    </div>
  {/if}

  <button class="applica" onclick={applica} disabled={!s || inCorso}>
    {inCorso ? "Salvataggio…" : "Salva copia corretta…"}
  </button>

  {#if esito?.ok}
    <div class="esito ok">
      Copia corretta salvata.
      <button class="link" onclick={apriCorretto}>Aprila in una nuova scheda</button>
    </div>
  {:else if esito && !esito.ok}
    <div class="esito err">{esito.msg}</div>
  {/if}

  <p class="nota piccola">
    L'aggiunta del testo alternativo alle singole immagini arriverà in seguito.
  </p>
</div>

<style>
  .pannello {
    display: flex;
    flex-direction: column;
    gap: 14px;
    height: 100%;
    overflow: auto;
    padding: 0 0 16px;
  }
  header {
    padding: 12px 14px;
    border-bottom: 1px solid var(--bordo);
  }
  h3 {
    margin: 0;
    font-size: 15px;
  }
  .nota {
    color: var(--testo-soft);
    font-size: 13px;
    margin: 0 14px;
  }
  .nota.piccola {
    font-size: 12px;
    font-style: italic;
  }
  .figure {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 0 14px;
    padding-top: 10px;
    border-top: 1px solid var(--bordo);
  }
  .titolo-sez {
    font-weight: 600;
    font-size: 13px;
  }
  .alt .capo {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .alt .vai {
    background: var(--scheda);
    border: 1px solid var(--bordo);
    color: var(--testo-soft);
    border-radius: 10px;
    padding: 1px 8px;
    font-size: 11px;
    cursor: pointer;
  }
  .manca {
    background: #4a2326;
    color: #ff9a9a;
    border-radius: 10px;
    padding: 1px 8px;
    font-size: 11px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0 14px;
    font-size: 13px;
    color: var(--testo-soft);
  }
  label.check {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    color: var(--testo);
  }
  select,
  input[type="text"] {
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 7px 9px;
    font-size: 14px;
  }
  input[type="checkbox"] {
    accent-color: var(--accento);
    width: 16px;
    height: 16px;
  }
  .applica {
    margin: 4px 14px 0;
    background: var(--accento);
    color: #fff;
    border: none;
    border-radius: 8px;
    padding: 10px 16px;
    cursor: pointer;
    font-size: 14px;
  }
  .applica:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .esito {
    margin: 0 14px;
    padding: 10px 12px;
    border-radius: 8px;
    font-size: 13px;
  }
  .esito.ok {
    background: #173021;
    color: #7ad08f;
  }
  .esito.err {
    background: var(--errore-sfondo);
    color: var(--errore);
  }
  .link {
    display: block;
    margin-top: 6px;
    background: none;
    border: none;
    color: var(--accento);
    cursor: pointer;
    padding: 0;
    text-decoration: underline;
    font-size: 13px;
  }
</style>
