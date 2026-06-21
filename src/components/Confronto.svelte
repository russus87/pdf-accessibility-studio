<script>
  // Vista di confronto tra due PDF aperti: testo, tag e immagine pixel-to-pixel,
  // con esportazione del report in HTML/PDF.
  import { schede } from "../lib/schede.svelte.js";
  import { confronta, confrontaImmagine, salvaReport, sovrapponiConfronto } from "../lib/api.js";

  // Solo le schede PDF possono essere confrontate.
  const pdf = $derived(schede.schede.filter((s) => s.tipo === "pdf"));

  let idA = $state(schede.schede.filter((s) => s.tipo === "pdf")[0]?.id ?? "");
  let idB = $state(schede.schede.filter((s) => s.tipo === "pdf")[1]?.id ?? "");
  let modalita = $state("testo"); // "testo" | "tag" | "immagine"

  let comb = $state(null); // { testo, tag }
  let caricamento = $state(false);
  let errore = $state(null);
  let esito = $state(null);

  let pagina = $state(0);
  let img = $state(null);
  let imgCaricamento = $state(false);
  let imgSovr = $state(null);

  async function sovrapponi() {
    if (!idA || !idB || idA === idB) return;
    imgCaricamento = true;
    imgSovr = null;
    try {
      imgSovr = await sovrapponiConfronto(idA, idB, pagina, 900);
    } catch (e) {
      errore = String(e);
    } finally {
      imgCaricamento = false;
    }
  }

  // Ricarica il confronto testo+tag quando cambiano A/B.
  $effect(() => {
    if (!idA || !idB || idA === idB) {
      comb = null;
      return;
    }
    const a = idA, b = idB;
    let annullato = false;
    caricamento = true;
    errore = null;
    img = null;
    confronta(a, b)
      .then((r) => !annullato && (comb = r))
      .catch((e) => !annullato && (errore = String(e)))
      .finally(() => !annullato && (caricamento = false));
    return () => (annullato = true);
  });

  async function confrontaImg() {
    if (!idA || !idB || idA === idB) return;
    imgCaricamento = true;
    img = null;
    try {
      img = await confrontaImmagine(idA, idB, pagina, 900);
    } catch (e) {
      errore = String(e);
    } finally {
      imgCaricamento = false;
    }
  }

  async function salva(formato) {
    esito = null;
    try {
      const ok = await salvaReport(idA, idB, formato);
      if (ok) esito = `Report ${formato.toUpperCase()} salvato.`;
    } catch (e) {
      esito = `Errore: ${e}`;
    }
  }

  const diffCorr = $derived(modalita === "tag" ? comb?.tag : comb?.testo);
</script>

<div class="confronto">
  <header>
    <div class="scelte">
      <label>A
        <select bind:value={idA}>
          {#each pdf as s}<option value={s.id}>{s.nome}</option>{/each}
        </select>
      </label>
      <span class="vs">vs</span>
      <label>B
        <select bind:value={idB}>
          {#each pdf as s}<option value={s.id}>{s.nome}</option>{/each}
        </select>
      </label>
    </div>

    <div class="modi">
      <button class:on={modalita === "testo"} onclick={() => (modalita = "testo")}>Testo</button>
      <button class:on={modalita === "tag"} onclick={() => (modalita = "tag")}>Tag</button>
      <button class:on={modalita === "immagine"} onclick={() => (modalita = "immagine")}>Immagine</button>
      <button class:on={modalita === "sovrapposizione"} onclick={() => (modalita = "sovrapposizione")}>Sovrapposizione</button>
    </div>

    <div class="report">
      <button onclick={() => salva("html")} disabled={!comb}>Report HTML</button>
      <button onclick={() => salva("pdf")} disabled={!comb}>Report PDF</button>
    </div>
  </header>

  {#if esito}<p class="esito">{esito}</p>{/if}

  {#if idA === idB}
    <p class="info">Scegli due documenti diversi.</p>
  {:else if errore}
    <p class="err">{errore}</p>
  {:else if modalita === "immagine"}
    <div class="barra-img">
      <label>Pagina
        <input type="number" min="1" bind:value={pagina} oninput={(e) => (pagina = Math.max(0, +e.target.value - 1))} style="width:64px" />
      </label>
      <button onclick={confrontaImg}>Confronta pagina {pagina + 1}</button>
      {#if img}<span class="perc">{img.percentuale.toFixed(2)}% pixel diversi ({img.pixel_diversi.toLocaleString()})</span>{/if}
    </div>
    <div class="area-img">
      {#if imgCaricamento}
        <p class="info">Rendering e confronto…</p>
      {:else if img}
        <img src={`data:image/png;base64,${img.diff_png_base64}`} alt="Differenze pagina" />
      {:else}
        <p class="info">Premi “Confronta pagina”. Le differenze appariranno in rosso.</p>
      {/if}
    </div>
  {:else if modalita === "sovrapposizione"}
    <div class="barra-img">
      <label>Pagina
        <input type="number" min="1" bind:value={pagina} oninput={(e) => (pagina = Math.max(0, +e.target.value - 1))} style="width:64px" />
      </label>
      <button onclick={sovrapponi}>Sovrapponi pagina {pagina + 1}</button>
      <span class="perc">Rosso = solo nel 1°, Blu = solo nel 2°, Nero = uguale</span>
    </div>
    <div class="area-img">
      {#if imgCaricamento}
        <p class="info">Rendering e sovrapposizione…</p>
      {:else if imgSovr}
        <img src={imgSovr} alt="Sovrapposizione pagine" />
      {:else}
        <p class="info">Premi “Sovrapponi pagina”.</p>
      {/if}
    </div>
  {:else if caricamento}
    <p class="info">Confronto in corso…</p>
  {:else if diffCorr}
    <div class="stat">
      {#if diffCorr.identici}
        <span class="ok">Identici ✔</span>
      {:else}
        <span>Uguali: {diffCorr.uguali}</span>
        <span class="add">+{diffCorr.aggiunte}</span>
        <span class="del">−{diffCorr.rimosse}</span>
      {/if}
    </div>
    <div class="diff">
      {#each diffCorr.righe as r}
        <div class="riga {r.tipo}">{r.tipo === "aggiunta" ? "+" : r.tipo === "rimossa" ? "−" : " "} {r.testo}</div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .confronto {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--tela);
  }
  header {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    align-items: center;
    padding: 10px 14px;
    background: var(--barra);
    border-bottom: 1px solid var(--bordo);
  }
  label {
    color: var(--testo-soft);
    font-size: 12px;
    display: inline-flex;
    gap: 6px;
    align-items: center;
  }
  select,
  input {
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 4px 8px;
  }
  .vs {
    color: var(--testo-soft);
  }
  .modi,
  .report {
    display: flex;
    gap: 6px;
  }
  .report {
    margin-left: auto;
  }
  header button {
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 6px 12px;
    cursor: pointer;
  }
  header button.on {
    background: var(--accento);
    border-color: var(--accento);
    color: #fff;
  }
  .stat {
    display: flex;
    gap: 16px;
    padding: 10px 14px;
    font-weight: bold;
  }
  .stat .add,
  .riga.aggiunta {
    color: #6ee08a;
  }
  .stat .del,
  .riga.rimossa {
    color: #ff8d8d;
  }
  .stat .ok {
    color: #6ee08a;
  }
  .diff {
    flex: 1;
    overflow: auto;
    font-family: ui-monospace, monospace;
    font-size: 13px;
    padding: 0 8px 16px;
  }
  .riga {
    white-space: pre-wrap;
    padding: 1px 6px;
  }
  .riga.aggiunta {
    background: #13301d;
  }
  .riga.rimossa {
    background: #34191b;
  }
  .barra-img {
    display: flex;
    gap: 14px;
    align-items: center;
    padding: 10px 14px;
  }
  .barra-img button {
    background: var(--accento);
    color: #fff;
    border: none;
    border-radius: 6px;
    padding: 6px 12px;
    cursor: pointer;
  }
  .perc {
    color: var(--testo-soft);
  }
  .area-img {
    flex: 1;
    overflow: auto;
    display: flex;
    justify-content: center;
    padding: 16px;
  }
  .area-img img {
    align-self: flex-start;
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.5);
  }
  .info,
  .err,
  .esito {
    padding: 14px;
    color: var(--testo-soft);
  }
  .err {
    color: var(--errore);
  }
  .esito {
    color: #7ad08f;
  }
</style>
