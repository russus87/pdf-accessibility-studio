<script>
  // Accessibilità moduli (AcroForm): mostra i campi del modulo e permette di
  // impostare il tooltip /TU (etichetta letta dagli screen reader) e l'ordine di
  // tabulazione strutturale. Salva una copia accessibile.
  import { schede } from "../lib/schede.svelte.js";
  import { leggiForm, applicaForm } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);

  let campi = $state([]);
  let tooltipMod = $state({}); // riferimento -> tooltip
  let tabsStruttura = $state(true);
  let caricamento = $state(false);
  let errore = $state(null);
  let esito = $state(null);

  // Etichette leggibili per il tipo di campo (/FT).
  const TIPI = { Tx: "Testo", Btn: "Pulsante/Checkbox", Ch: "Scelta", Sig: "Firma" };

  $effect(() => {
    if (!s) return;
    const id = s.id;
    let annullato = false;
    caricamento = true;
    errore = null;
    esito = null;
    campi = [];
    tooltipMod = {};
    leggiForm(id)
      .then((r) => {
        if (annullato) return;
        campi = r;
        const valori = {};
        for (const c of r) valori[c.riferimento] = c.tooltip || "";
        tooltipMod = valori;
      })
      .catch((e) => !annullato && (errore = String(e)))
      .finally(() => !annullato && (caricamento = false));
    return () => (annullato = true);
  });

  const senzaTooltip = $derived(campi.filter((c) => !c.tooltip).length);

  async function applica() {
    esito = null;
    // Invia solo i campi con un tooltip compilato.
    const mod = Object.entries(tooltipMod)
      .filter(([, t]) => t && t.trim())
      .map(([riferimento, tooltip]) => ({ riferimento, tooltip: tooltip.trim() }));
    if (!mod.length && !tabsStruttura) {
      esito = { ok: false, msg: "Compila almeno un'etichetta o attiva l'ordine di tabulazione." };
      return;
    }
    try {
      const r = await applicaForm(s.id, mod, tabsStruttura);
      if (r) esito = { ok: true, dest: r.dest, n: r.n };
    } catch (e) {
      esito = { ok: false, msg: String(e) };
    }
  }

  function apriCopia() {
    if (esito?.ok) schede.apri(esito.dest);
  }
</script>

<div class="pannello">
  <header><h3>Moduli accessibili</h3></header>

  {#if caricamento}
    <p class="info">Lettura campi del modulo…</p>
  {:else if errore}
    <p class="err">{errore}</p>
  {:else if !campi.length}
    <p class="info">Questo PDF non contiene un modulo compilabile (AcroForm).</p>
  {:else}
    <p class="nota">
      Dai a ogni campo un'<b>etichetta</b> (tooltip <code>/TU</code>): è il testo che
      lo screen reader annuncia. {#if senzaTooltip}<b>{senzaTooltip}</b> camp{senzaTooltip === 1 ? "o" : "i"} senza etichetta.{/if}
    </p>

    <ul>
      {#each campi as c, i}
        <li>
          <div class="capo">
            <span class="tipo">{TIPI[c.tipo] || c.tipo || "Campo"}</span>
            {#if c.nome}<span class="nome">{c.nome}</span>{/if}
            {#if !c.tooltip}<span class="manca">manca</span>{/if}
            {#if c.pagina != null}<button class="vai" onclick={() => schede.vaiAPagina(c.pagina)}>p.{c.pagina + 1}</button>{/if}
          </div>
          <input type="text" bind:value={tooltipMod[c.riferimento]} placeholder="Etichetta del campo (es. «Indirizzo email»)" />
        </li>
      {/each}
    </ul>

    <label class="check">
      <input type="checkbox" bind:checked={tabsStruttura} />
      Imposta l'ordine di tabulazione sulla struttura (<code>/Tabs /S</code>)
    </label>

    <button class="applica" onclick={applica}>Salva copia accessibile…</button>

    {#if esito?.ok}
      <div class="esito ok">
        Copia salvata ({esito.n} etichett{esito.n === 1 ? "a" : "e"}).
        <button class="link" onclick={apriCopia}>Aprila in una nuova scheda</button>
      </div>
    {:else if esito && !esito.ok}
      <div class="esito err">{esito.msg}</div>
    {/if}
  {/if}
</div>

<style>
  .pannello {
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: 100%;
    overflow: auto;
    padding-bottom: 16px;
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
    margin: 0 14px;
    font-size: 13px;
    color: var(--testo-soft);
  }
  code {
    font-size: 12px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .capo {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
    font-size: 13px;
  }
  .tipo {
    color: var(--accento);
    font-weight: 600;
  }
  .nome {
    color: var(--testo-soft);
    font-family: ui-monospace, monospace;
    font-size: 12px;
  }
  .manca {
    background: #4a2326;
    color: #ff9a9a;
    border-radius: 10px;
    padding: 1px 8px;
    font-size: 11px;
  }
  .vai {
    margin-left: auto;
    background: var(--scheda);
    border: 1px solid var(--bordo);
    color: var(--testo-soft);
    border-radius: 10px;
    padding: 1px 8px;
    font-size: 11px;
    cursor: pointer;
  }
  input[type="text"] {
    width: 100%;
    box-sizing: border-box;
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 7px 9px;
    font-size: 14px;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 14px;
    font-size: 13px;
    color: var(--testo);
  }
  .check input {
    accent-color: var(--accento);
    width: 16px;
    height: 16px;
  }
  .applica {
    margin: 0 14px;
    background: var(--accento);
    color: #fff;
    border: none;
    border-radius: 8px;
    padding: 10px 16px;
    cursor: pointer;
    font-size: 14px;
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
  .info,
  .err {
    padding: 12px 14px;
    color: var(--testo-soft);
  }
  .err {
    color: var(--errore);
  }
</style>
