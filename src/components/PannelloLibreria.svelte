<script>
  // Libreria: ricerca full-text in tutti i PDF di una cartella (Fase 39).
  import { schede } from "../lib/schede.svelte.js";
  import { ricercaMulti } from "../lib/api.js";

  let query = $state("");
  let risultati = $state([]);
  let inCorso = $state(false);
  let cercato = $state(false);
  let errore = $state(null);

  async function cerca() {
    if (!query.trim()) return;
    errore = null;
    inCorso = true;
    risultati = [];
    try {
      const r = await ricercaMulti(query.trim());
      if (r) { risultati = r; cercato = true; }
    } catch (e) {
      errore = String(e);
    } finally {
      inCorso = false;
    }
  }

  function nomeFile(p) {
    return p.split(/[\\/]/).pop();
  }

  async function apri(r) {
    await schede.apri(r.file);
    schede.vaiAPagina(r.pagina);
  }
</script>

<div class="pannello">
  <header><h3>Libreria — ricerca multi-file</h3></header>
  <div class="cerca">
    <input type="text" bind:value={query} placeholder="Testo da cercare in una cartella…" onkeydown={(e) => e.key === "Enter" && cerca()} />
    <button class="vai" onclick={cerca} disabled={inCorso}>{inCorso ? "…" : "Cerca cartella…"}</button>
  </div>

  {#if errore}<p class="err">{errore}</p>{/if}
  {#if inCorso}<p class="info">Ricerca in corso…</p>{/if}
  {#if cercato && !inCorso}
    <p class="info">{risultati.length} risultati{risultati.length >= 500 ? " (max 500)" : ""}.</p>
  {/if}

  <ul>
    {#each risultati as r}
      <li>
        <button class="riga" onclick={() => apri(r)}>
          <span class="file">{nomeFile(r.file)} · p.{r.pagina + 1}</span>
          <span class="ctx">{r.contesto}</span>
        </button>
      </li>
    {/each}
  </ul>
</div>

<style>
  .pannello { display: flex; flex-direction: column; height: 100%; overflow: hidden; }
  header { padding: 12px 14px; border-bottom: 1px solid var(--bordo); }
  h3 { margin: 0; font-size: 15px; }
  .cerca { display: flex; gap: 6px; padding: 10px 14px; }
  .cerca input { flex: 1; background: var(--scheda); color: var(--testo); border: 1px solid var(--bordo); border-radius: 6px; padding: 7px 9px; font-size: 14px; }
  .vai { background: var(--accento); color: #fff; border: none; border-radius: 8px; padding: 7px 12px; cursor: pointer; font-size: 13px; white-space: nowrap; }
  .vai:disabled { opacity: 0.5; }
  .info, .err { padding: 4px 14px; font-size: 12px; color: var(--testo-soft); margin: 0; }
  .err { color: var(--errore); }
  ul { list-style: none; margin: 0; padding: 0 8px 16px; overflow: auto; }
  .riga { display: flex; flex-direction: column; gap: 2px; width: 100%; text-align: left; background: transparent; border: none; border-radius: 6px; padding: 7px 8px; cursor: pointer; color: var(--testo); }
  .riga:hover { background: var(--scheda); }
  .file { font-size: 12px; color: var(--accento); font-weight: 600; }
  .ctx { font-size: 12px; color: var(--testo-soft); }
</style>
