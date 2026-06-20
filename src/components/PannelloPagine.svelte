<script>
  // Operazioni sulle pagine: ruota / elimina / estrai (su selezione), riordina,
  // unisci con altri PDF. Ogni operazione salva una copia.
  import { schede } from "../lib/schede.svelte.js";
  import { ruotaPagine, eliminaPagine, estraiPagine, riordinaPagine, unisciPdf, ritagliaPagine, inserisciPagine, splitPdf } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);
  let modo = $state("selezione"); // "selezione" | "riordina"
  let sel = $state([]);
  let ordine = $state([]);
  let esito = $state(null);
  let crop = $state({ x: 10, y: 10, w: 150, h: 200 });
  let mostraCrop = $state(false);
  let split = $state({ mostra: false, modo: "ogni_n", n: 1 });

  async function dividi() {
    esito = null;
    try {
      const r = await splitPdf(s.id, split.modo, split.n);
      if (r) esito = { ok: true, msg: `${r.length} file creati nella cartella.` };
    } catch (e) {
      esito = { ok: false, msg: String(e) };
    }
  }

  $effect(() => {
    if (!s) return;
    const n = s.pagine;
    sel = Array(n).fill(false);
    ordine = Array.from({ length: n }, (_, i) => i + 1);
    esito = null;
  });

  const selezionate = $derived(sel.map((v, i) => (v ? i + 1 : null)).filter((x) => x != null));

  function tutto(v) {
    sel = sel.map(() => v);
  }

  async function esegui(promessa, richiedeSelezione = true) {
    if (richiedeSelezione && selezionate.length === 0) {
      esito = { ok: false, msg: "Seleziona almeno una pagina." };
      return;
    }
    esito = null;
    try {
      const dest = await promessa();
      if (dest) esito = { ok: true, dest };
    } catch (e) {
      esito = { ok: false, msg: String(e) };
    }
  }

  function muovi(i, d) {
    const j = i + d;
    if (j < 0 || j >= ordine.length) return;
    const a = [...ordine];
    [a[i], a[j]] = [a[j], a[i]];
    ordine = a;
  }
</script>

<div class="pannello">
  <header>
    <h3>Pagine</h3>
    <div class="modi">
      <button class:on={modo === "selezione"} onclick={() => (modo = "selezione")}>Modifica</button>
      <button class:on={modo === "riordina"} onclick={() => (modo = "riordina")}>Riordina</button>
    </div>
  </header>

  {#if esito?.ok}
    <p class="esito">{esito.msg || "Copia salvata."}
      {#if esito.dest}<button class="link" onclick={() => schede.apri(esito.dest)}>Aprila</button>{/if}
    </p>
  {:else if esito && !esito.ok}
    <p class="err">{esito.msg}</p>
  {/if}

  {#if !s}
    <p class="info">Nessun documento.</p>
  {:else if modo === "selezione"}
    <div class="azioni">
      <button onclick={() => esegui(() => ruotaPagine(s.id, selezionate, 90))}>Ruota ⟳90</button>
      <button onclick={() => esegui(() => ruotaPagine(s.id, selezionate, -90))}>Ruota ⟲90</button>
      <button onclick={() => esegui(() => ruotaPagine(s.id, selezionate, 180))}>180°</button>
      <button onclick={() => esegui(() => eliminaPagine(s.id, selezionate))}>Elimina</button>
      <button onclick={() => esegui(() => estraiPagine(s.id, selezionate))}>Estrai</button>
      <button class="unisci" onclick={() => esegui(() => unisciPdf(s.id), false)}>Unisci con…</button>
      <button onclick={() => esegui(() => inserisciPagine(s.id, selezionate[0] ?? (s.pagine + 1)), false)}>Inserisci PDF…</button>
      <button onclick={() => (mostraCrop = !mostraCrop)}>Ritaglia…</button>
      <button onclick={() => (split.mostra = !split.mostra)}>Dividi…</button>
    </div>
    {#if split.mostra}
      <div class="crop">
        <span class="ct">Dividi il PDF in più file</span>
        <div class="cr">
          <label style="flex:2">Modo
            <select bind:value={split.modo}>
              <option value="ogni_n">Ogni N pagine</option>
              <option value="segnalibri">Per segnalibri</option>
            </select>
          </label>
          {#if split.modo === "ogni_n"}<label>N<input type="number" min="1" bind:value={split.n} /></label>{/if}
        </div>
        <button onclick={dividi}>Dividi in cartella…</button>
      </div>
    {/if}
    {#if mostraCrop}
      <div class="crop">
        <span class="ct">Area di ritaglio (mm, da alto-sx) sulle pagine selezionate</span>
        <div class="cr">
          <label>X<input type="number" bind:value={crop.x} /></label>
          <label>Y<input type="number" bind:value={crop.y} /></label>
          <label>L<input type="number" bind:value={crop.w} /></label>
          <label>A<input type="number" bind:value={crop.h} /></label>
        </div>
        <button onclick={() => esegui(() => ritagliaPagine(s.id, selezionate, crop))}>Applica ritaglio…</button>
      </div>
    {/if}
    <div class="seltutto">
      <button onclick={() => tutto(true)}>Seleziona tutto</button>
      <button onclick={() => tutto(false)}>Deseleziona</button>
      <span>{selezionate.length} selezionate</span>
    </div>
    <div class="griglia">
      {#each sel as v, i}
        <label class="cella" class:on={v}>
          <input type="checkbox" bind:checked={sel[i]} />
          {i + 1}
        </label>
      {/each}
    </div>
  {:else}
    <p class="suggerimento">Sposta le pagine nel nuovo ordine di lettura.</p>
    <ul>
      {#each ordine as p, i}
        <li>
          <span>Pagina {p}</span>
          <span class="frecce">
            <button onclick={() => muovi(i, -1)} disabled={i === 0}>▲</button>
            <button onclick={() => muovi(i, 1)} disabled={i === ordine.length - 1}>▼</button>
          </span>
        </li>
      {/each}
    </ul>
    <button class="salva" onclick={() => esegui(() => riordinaPagine(s.id, ordine), false)}>Salva nuovo ordine…</button>
  {/if}
</div>

<style>
  .crop { display: flex; flex-direction: column; gap: 6px; margin: 0 14px 8px; padding: 8px; background: var(--scheda); border: 1px solid var(--bordo); border-radius: 8px; }
  .crop .ct { font-size: 11px; color: var(--testo-soft); }
  .crop .cr { display: flex; gap: 6px; }
  .crop .cr label { display: flex; flex-direction: column; gap: 2px; font-size: 11px; color: var(--testo-soft); flex: 1; }
  .crop .cr input { width: 100%; box-sizing: border-box; background: var(--sfondo); color: var(--testo); border: 1px solid var(--bordo); border-radius: 6px; padding: 4px 6px; }
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
    margin: 0 0 8px;
    font-size: 15px;
  }
  .modi {
    display: flex;
    gap: 6px;
  }
  .modi button {
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 5px 10px;
    cursor: pointer;
    font-size: 12px;
  }
  .modi button.on {
    background: var(--accento);
    border-color: var(--accento);
    color: #fff;
  }
  .azioni {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 10px 14px;
  }
  .azioni button {
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 6px 10px;
    cursor: pointer;
    font-size: 12px;
  }
  .azioni button:hover {
    border-color: var(--accento);
  }
  .azioni .unisci {
    margin-left: auto;
    color: var(--accento);
  }
  .seltutto {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 0 14px 8px;
    font-size: 12px;
    color: var(--testo-soft);
  }
  .seltutto button {
    background: none;
    border: none;
    color: var(--accento);
    cursor: pointer;
    padding: 0;
    text-decoration: underline;
  }
  .griglia {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(54px, 1fr));
    gap: 6px;
    padding: 0 14px 16px;
  }
  .cella {
    display: flex;
    align-items: center;
    gap: 4px;
    justify-content: center;
    border: 1px solid var(--bordo);
    border-radius: 6px;
    padding: 8px 4px;
    cursor: pointer;
    font-size: 13px;
    background: var(--scheda);
  }
  .cella.on {
    border-color: var(--accento);
    background: var(--accento-soft);
  }
  .cella input {
    accent-color: var(--accento);
  }
  .suggerimento {
    margin: 0;
    padding: 8px 14px 0;
    font-size: 12px;
    color: var(--testo-soft);
    font-style: italic;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 8px 14px;
  }
  li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 4px 8px;
    border-bottom: 1px solid var(--bordo);
  }
  .frecce button {
    background: var(--scheda);
    color: var(--testo);
    border: 1px solid var(--bordo);
    border-radius: 4px;
    cursor: pointer;
    padding: 1px 7px;
  }
  .frecce button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .salva {
    margin: 8px 14px 16px;
    background: var(--accento);
    color: #fff;
    border: none;
    border-radius: 8px;
    padding: 9px 16px;
    cursor: pointer;
  }
  .info,
  .esito,
  .err {
    padding: 12px 14px;
    color: var(--testo-soft);
  }
  .esito {
    color: #7ad08f;
  }
  .err {
    color: var(--errore);
  }
  .link {
    background: none;
    border: none;
    color: var(--accento);
    cursor: pointer;
    text-decoration: underline;
    padding: 0 0 0 6px;
  }
</style>
