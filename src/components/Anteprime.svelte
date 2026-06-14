<script>
  // Striscia verticale di anteprime delle pagine. Cliccando una miniatura il
  // visore salta a quella pagina. Le miniature si caricano in sequenza (la cache
  // del backend rende veloci le richieste ripetute).
  import { schede } from "../lib/schede.svelte.js";
  import { renderPagina } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);
  let srcs = $state([]);
  let token = 0;

  $effect(() => {
    if (!s) {
      srcs = [];
      return;
    }
    const id = s.id;
    const n = s.pagine;
    const mio = ++token;
    srcs = new Array(n).fill(null);
    (async () => {
      for (let i = 0; i < n; i++) {
        if (mio !== token) return;
        try {
          const u = await renderPagina(id, i, 160);
          if (mio !== token) return;
          srcs[i] = u;
        } catch (_) {
          /* salta la miniatura non renderizzabile */
        }
      }
    })();
  });
</script>

<aside class="anteprime">
  {#if s}
    {#each srcs as src, i}
      <button
        class="thumb"
        class:corrente={i === s.pagina}
        onclick={() => schede.vaiAPagina(i)}
        title={`Pagina ${i + 1}`}
      >
        {#if src}
          <img {src} alt={`Anteprima pagina ${i + 1}`} />
        {:else}
          <div class="placeholder">{i + 1}</div>
        {/if}
        <span class="num">{i + 1}</span>
      </button>
    {/each}
  {/if}
</aside>

<style>
  .anteprime {
    width: 150px;
    flex: none;
    overflow-y: auto;
    background: var(--barra);
    border-right: 1px solid var(--bordo);
    padding: 10px 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
  }
  .thumb {
    position: relative;
    width: 110px;
    background: transparent;
    border: 2px solid transparent;
    border-radius: 4px;
    padding: 0;
    cursor: pointer;
    line-height: 0;
  }
  .thumb.corrente {
    border-color: var(--accento);
  }
  .thumb img {
    width: 110px;
    border-radius: 2px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
    background: #fff;
  }
  .placeholder {
    width: 110px;
    height: 142px;
    display: grid;
    place-items: center;
    background: var(--scheda);
    color: var(--testo-soft);
    border-radius: 2px;
    line-height: normal;
  }
  .num {
    position: absolute;
    bottom: 4px;
    right: 6px;
    background: rgba(0, 0, 0, 0.6);
    color: #fff;
    font-size: 11px;
    padding: 1px 5px;
    border-radius: 8px;
    line-height: normal;
  }
</style>
