<script>
  // Strumento di misura avanzato sopra la pagina renderizzata.
  //
  // Tre modalità:
  //  - "righello": un righello semitrasparente trascinabile, orientabile in
  //    orizzontale o verticale (icone), con scala in cm/mm per misure precise;
  //  - "distanza": misura la distanza (e l'angolo) tra due punti cliccati;
  //  - "area": misura larghezza, altezza e area di un rettangolo.
  //
  // Tutto è in frazioni 0..1 della pagina (indipendente dallo zoom). Le dimensioni
  // in punti PDF (1 pt = 1/72") arrivano dal backend; i pixel sono quelli reali
  // del rendering corrente. Unità selezionabile: cm, mm, pollici, punti.

  let { imgEl = null, larghezzaPt, altezzaPt } = $props();

  const PT_PER_CM = 72 / 2.54; // ≈ 28.3465

  let box = $state(null);
  let modo = $state("righello"); // righello | calibro | distanza | area
  let unita = $state("cm"); // cm | mm | in | pt
  let orientamento = $state("orizz"); // orizz | vert (righello e calibro)
  let rigPos = $state(0.5); // posizione del righello lungo l'asse perpendicolare (0..1)
  let trascina = $state(false);
  let punti = $state([]); // per distanza/area
  let cursore = $state(null); // {fx, fy}

  // Calibro: due ganasce lungo l'asse di misura (0..1) trascinabili.
  let calA = $state(0.4);
  let calB = $state(0.6);
  let trascCal = $state(null); // "A" | "B" | null

  // Azzera misura e ricentra strumenti quando cambia pagina.
  $effect(() => {
    void larghezzaPt;
    void altezzaPt;
    punti = [];
    rigPos = 0.5;
    calA = 0.4;
    calB = 0.6;
  });

  const cmLarghezza = $derived(larghezzaPt / PT_PER_CM);
  const cmAltezza = $derived(altezzaPt / PT_PER_CM);

  // Converte una lunghezza in punti nell'unità scelta.
  function conv(distPt) {
    switch (unita) {
      case "mm": return distPt / PT_PER_CM * 10;
      case "in": return distPt / 72;
      case "pt": return distPt;
      default: return distPt / PT_PER_CM;
    }
  }
  const decimali = $derived(unita === "pt" ? 1 : unita === "mm" ? 1 : 2);
  const fmt = (n, d = decimali) => n.toLocaleString("it-IT", { minimumFractionDigits: d, maximumFractionDigits: d });
  const fmtU = (distPt) => `${fmt(conv(distPt))} ${unita}`;

  // --- Righello: tacche ogni mm lungo l'asse, numeri ai cm. ---
  function tacche(totPt) {
    const totCm = totPt / PT_PER_CM;
    const out = [];
    const nMm = Math.floor(totCm * 10 + 1e-4);
    for (let mm = 0; mm <= nMm; mm++) {
      const cm = mm / 10;
      out.push({ pos: (cm * PT_PER_CM / totPt) * 100, major: mm % 10 === 0, mid: mm % 5 === 0, label: mm % 10 === 0 ? mm / 10 : null });
    }
    return out;
  }
  const taccheRig = $derived(tacche(orientamento === "orizz" ? larghezzaPt : altezzaPt));
  // Posizione del righello nell'unità scelta (distanza dal bordo).
  const posRigPt = $derived(rigPos * (orientamento === "orizz" ? altezzaPt : larghezzaPt));

  function frazioni(e) {
    const r = box.getBoundingClientRect();
    return {
      fx: Math.min(Math.max((e.clientX - r.left) / r.width, 0), 1),
      fy: Math.min(Math.max((e.clientY - r.top) / r.height, 0), 1),
    };
  }

  function suMove(e) {
    cursore = frazioni(e);
    if (trascina) {
      rigPos = orientamento === "orizz" ? cursore.fy : cursore.fx;
    }
    if (trascCal) {
      const v = orientamento === "orizz" ? cursore.fx : cursore.fy;
      if (trascCal === "A") calA = v;
      else calB = v;
    }
  }
  function calDown(quale, e) {
    trascCal = quale;
    e.currentTarget.setPointerCapture?.(e.pointerId);
    e.stopPropagation();
  }
  function calUp() {
    trascCal = null;
  }
  // Regolazione fine di una ganascia con le frecce (0.1% / 1% con Shift).
  function calTasto(quale, e) {
    const tot = orientamento === "orizz" ? larghezzaPt : altezzaPt;
    const passo = (e.shiftKey ? 0.01 : 0.001) * (tot > 0 ? 1 : 1);
    let d = 0;
    if (e.key === "ArrowLeft" || e.key === "ArrowUp") d = -passo;
    else if (e.key === "ArrowRight" || e.key === "ArrowDown") d = passo;
    else return;
    e.preventDefault();
    if (quale === "A") calA = Math.min(1, Math.max(0, calA + d));
    else calB = Math.min(1, Math.max(0, calB + d));
  }
  function rigDown(e) {
    trascina = true;
    e.currentTarget.setPointerCapture?.(e.pointerId);
    e.stopPropagation();
  }
  function rigUp() {
    trascina = false;
  }

  // Etichetta di un punto: A, B, C, … (oltre Z continua con A1, B1…).
  const etichetta = (i) => (i < 26 ? String.fromCharCode(65 + i) : String.fromCharCode(65 + (i % 26)) + Math.floor(i / 26));

  function suClic(e) {
    if (modo === "righello" || modo === "calibro") return; // si trascinano, non si cliccano
    const { fx, fy } = frazioni(e);
    if (modo === "area") {
      // Due angoli opposti del rettangolo.
      punti = punti.length >= 2 ? [{ x: fx, y: fy }] : [...punti, { x: fx, y: fy }];
    } else {
      // Distanza: catena di punti annotati (A, B, C, …).
      punti = [...punti, { x: fx, y: fy }];
    }
  }

  function annullaUltimo() {
    punti = punti.slice(0, -1);
  }

  function azzera() {
    punti = [];
  }

  // Percorso di misura: segmenti tra punti consecutivi + lunghezza totale.
  const percorso = $derived.by(() => {
    if (modo !== "distanza" || punti.length < 2) return null;
    const pw = imgEl?.naturalWidth || larghezzaPt;
    const ph = imgEl?.naturalHeight || altezzaPt;
    const segmenti = [];
    let totPt = 0;
    for (let i = 1; i < punti.length; i++) {
      const a = punti[i - 1];
      const b = punti[i];
      const dxPt = (b.x - a.x) * larghezzaPt;
      const dyPt = (b.y - a.y) * altezzaPt;
      const distPt = Math.hypot(dxPt, dyPt);
      totPt += distPt;
      const angolo = (((Math.atan2(-dyPt, dxPt) * 180) / Math.PI) % 360 + 360) % 360;
      segmenti.push({ da: etichetta(i - 1), a: etichetta(i), distPt, angolo });
    }
    const dpi = larghezzaPt > 0 ? pw / (larghezzaPt / 72) : 0;
    void ph;
    return { segmenti, totPt, dpi };
  });
  // Stringa "x,y x,y …" per la polilinea SVG (viewBox 0..1000).
  const polilinea = $derived(punti.map((p) => `${p.x * 1000},${p.y * 1000}`).join(" "));

  // Calibro: distanza fra le due ganasce lungo l'asse scelto.
  const calibro = $derived.by(() => {
    if (modo !== "calibro") return null;
    const tot = orientamento === "orizz" ? larghezzaPt : altezzaPt;
    const distPt = Math.abs(calB - calA) * tot;
    const mmPrec = (distPt / PT_PER_CM) * 10;
    return { distPt, min: Math.min(calA, calB) * 100, max: Math.max(calA, calB) * 100, mmPrec };
  });

  const area = $derived.by(() => {
    if (modo !== "area" || punti.length < 2) return null;
    const [a, b] = punti;
    const wPt = Math.abs(b.x - a.x) * larghezzaPt;
    const hPt = Math.abs(b.y - a.y) * altezzaPt;
    const areaCm2 = (wPt / PT_PER_CM) * (hPt / PT_PER_CM);
    return { wPt, hPt, areaCm2,
      left: Math.min(a.x, b.x) * 100, top: Math.min(a.y, b.y) * 100,
      w: Math.abs(b.x - a.x) * 100, h: Math.abs(b.y - a.y) * 100 };
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_click_events_have_key_events a11y_no_noninteractive_tabindex -->
<div
  class="misura"
  class:trascinando={trascina}
  bind:this={box}
  role="application"
  aria-label="Strumento di misura"
  tabindex="0"
  onpointermove={suMove}
  onclick={suClic}
  onkeydown={(e) => { if (e.key === "Escape") azzera(); }}
>
  <!-- Righello trascinabile -->
  {#if modo === "righello"}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="righello-band {orientamento}"
      style={orientamento === "orizz" ? `top:${rigPos * 100}%` : `left:${rigPos * 100}%`}
      role="slider"
      tabindex="0"
      aria-label="Righello (trascina per spostare)"
      aria-valuenow={Math.round(conv(posRigPt))}
      onpointerdown={rigDown}
      onpointerup={rigUp}
      onpointermove={suMove}
      onkeydown={(e) => {
        const step = e.shiftKey ? 0.05 : 0.005;
        if (e.key === "ArrowUp" || e.key === "ArrowLeft") rigPos = Math.max(0, rigPos - step);
        if (e.key === "ArrowDown" || e.key === "ArrowRight") rigPos = Math.min(1, rigPos + step);
      }}
    >
      <div class="linea-rif"></div>
      {#each taccheRig as t}
        <div class="rt" class:maj={t.major} class:mid={t.mid} style={orientamento === "orizz" ? `left:${t.pos}%` : `top:${t.pos}%`}>
          {#if t.label != null}<span class="rn">{t.label}</span>{/if}
        </div>
      {/each}
      <div class="rig-pos">{fmt(conv(posRigPt))} {unita}</div>
    </div>
  {/if}

  <!-- Calibro: due ganasce trascinabili lungo l'asse -->
  {#if modo === "calibro"}
    {#if calibro}
      <div
        class="cal-dim {orientamento}"
        style={orientamento === "orizz"
          ? `left:${calibro.min}%;width:${calibro.max - calibro.min}%`
          : `top:${calibro.min}%;height:${calibro.max - calibro.min}%`}
        aria-hidden="true"
      >
        <span class="cal-val">{fmtU(calibro.distPt)}</span>
      </div>
    {/if}
    {#each [["A", calA], ["B", calB]] as [q, pos]}
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div
        class="ganascia {orientamento}"
        style={orientamento === "orizz" ? `left:${pos * 100}%` : `top:${pos * 100}%`}
        role="slider"
        tabindex="0"
        aria-label={`Ganascia ${q}`}
        aria-valuenow={Math.round(conv(pos * (orientamento === "orizz" ? larghezzaPt : altezzaPt)))}
        onpointerdown={(e) => calDown(q, e)}
        onpointerup={calUp}
        onpointermove={suMove}
        onkeydown={(e) => calTasto(q, e)}
      >
        <span class="gan-eti">{q}</span>
      </div>
    {/each}
  {/if}

  <!-- Distanza: polilinea tra i punti annotati -->
  {#if modo === "distanza" && punti.length >= 2}
    <svg class="disegno" viewBox="0 0 1000 1000" preserveAspectRatio="none" aria-hidden="true">
      <polyline points={polilinea} vector-effect="non-scaling-stroke" />
    </svg>
  {/if}

  <!-- Area -->
  {#if area}
    <div class="area-box" style={`left:${area.left}%;top:${area.top}%;width:${area.w}%;height:${area.h}%`} aria-hidden="true"></div>
  {/if}

  {#if modo !== "righello"}
    {#each punti as p, i}
      <div class="punto" style={`left:${p.x * 100}%;top:${p.y * 100}%`} aria-hidden="true">
        {#if modo === "distanza"}<span class="punto-eti">{etichetta(i)}</span>{/if}
      </div>
    {/each}
  {/if}

  <!-- Pannello comandi + valori -->
  <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
  <div class="pannello" onpointerdown={(e) => e.stopPropagation()} onclick={(e) => e.stopPropagation()}>
    <div class="tit">📏 Misura</div>

    <div class="modi">
      <button class:on={modo === "righello"} onclick={() => (modo = "righello")}>Righello</button>
      <button class:on={modo === "calibro"} onclick={() => (modo = "calibro")}>Calibro</button>
      <button class:on={modo === "distanza"} onclick={() => { modo = "distanza"; punti = []; }}>Distanza</button>
      <button class:on={modo === "area"} onclick={() => { modo = "area"; punti = []; }}>Area</button>
    </div>

    <div class="riga2">
      <select bind:value={unita} aria-label="Unità">
        <option value="cm">cm</option>
        <option value="mm">mm</option>
        <option value="in">in</option>
        <option value="pt">pt</option>
      </select>
      {#if modo === "righello" || modo === "calibro"}
        <div class="orient">
          <button class:on={orientamento === "orizz"} title="Orizzontale (parallelo)" aria-label="Orientamento orizzontale" onclick={() => (orientamento = "orizz")}>━</button>
          <button class:on={orientamento === "vert"} title="Verticale (90°)" aria-label="Orientamento verticale" onclick={() => (orientamento = "vert")}>┃</button>
        </div>
      {/if}
    </div>

    {#if modo === "righello"}
      <div class="val"><span class="k">Posizione</span><b>{fmt(conv(posRigPt))}</b><span class="u">{unita}</span></div>
      <div class="hint">Trascina il righello. Usa le icone per orientarlo.</div>
    {:else if modo === "calibro"}
      {#if calibro}
        <div class="val grande"><span class="k">Apertura</span><b>{fmtU(calibro.distPt)}</b></div>
        <div class="hint">≈ {fmt(calibro.mmPrec, 2)} mm · trascina le ganasce A e B; frecce per la regolazione fine (Shift = passo grande).</div>
      {/if}
    {:else if modo === "distanza"}
      {#if percorso}
        <div class="val grande"><span class="k">Totale</span><b>{fmtU(percorso.totPt)}</b></div>
        <div class="segmenti">
          {#each percorso.segmenti as s}
            <div class="seg"><span class="seg-n">{s.da}→{s.a}</span><b>{fmtU(s.distPt)}</b><span class="u">{fmt(s.angolo, 0)}°</span></div>
          {/each}
        </div>
        <div class="azioni">
          <button onclick={annullaUltimo}>Annulla punto</button>
          <button onclick={azzera}>Azzera</button>
        </div>
        <div class="hint">Clicca per aggiungere altri punti (A, B, C…).</div>
      {:else}
        <div class="hint">Clicca i punti da misurare: vengono annotati A, B, C… con distanza per segmento e totale.</div>
      {/if}
    {:else if modo === "area"}
      {#if area}
        <div class="val"><span class="k">Largh.</span><b>{fmtU(area.wPt)}</b></div>
        <div class="val"><span class="k">Alt.</span><b>{fmtU(area.hPt)}</b></div>
        <div class="val"><span class="k">Area</span><b>{fmt(area.areaCm2)}</b><span class="u">cm²</span></div>
        <button onclick={azzera}>Azzera</button>
      {:else}
        <div class="hint">Clicca due angoli opposti.</div>
      {/if}
    {/if}

    <div class="dim">
      {#if cursore}Cursore {fmt(conv(cursore.fx * larghezzaPt))} × {fmt(conv(cursore.fy * altezzaPt))} {unita} · {/if}
      Pagina {fmt(cmLarghezza, 1)} × {fmt(cmAltezza, 1)} cm
    </div>
  </div>
</div>

<style>
  .misura { position: absolute; inset: 0; cursor: crosshair; z-index: 5; }
  .misura.trascinando { cursor: grabbing; }

  /* Righello trascinabile semitrasparente */
  .righello-band {
    position: absolute;
    background: linear-gradient(rgba(255, 214, 102, 0.30), rgba(255, 214, 102, 0.16));
    border: 1px solid rgba(180, 120, 0, 0.7);
    box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.25);
    cursor: grab;
    touch-action: none;
  }
  .righello-band:active { cursor: grabbing; }
  .righello-band.orizz { left: 0; right: 0; height: 30px; transform: translateY(-50%); }
  .righello-band.vert { top: 0; bottom: 0; width: 30px; transform: translateX(-50%); }
  /* Linea di riferimento centrale (la misura) */
  .linea-rif { position: absolute; background: #d11; }
  .orizz .linea-rif { left: 0; right: 0; top: 50%; height: 1px; }
  .vert .linea-rif { top: 0; bottom: 0; left: 50%; width: 1px; }
  /* Tacche */
  .rt { position: absolute; background: rgba(90, 60, 0, 0.85); }
  .orizz .rt { bottom: 0; width: 1px; height: 5px; }
  .orizz .rt.mid { height: 8px; }
  .orizz .rt.maj { height: 13px; background: rgba(60, 40, 0, 1); }
  .vert .rt { right: 0; height: 1px; width: 5px; }
  .vert .rt.mid { width: 8px; }
  .vert .rt.maj { width: 13px; background: rgba(60, 40, 0, 1); }
  .rn { position: absolute; font-size: 8px; color: #3a2800; font-weight: 600; line-height: 1; }
  .orizz .rn { top: 1px; left: 2px; }
  .vert .rn { left: 2px; top: 1px; }
  .rig-pos {
    position: absolute; background: #d11; color: #fff; font-size: 10px; padding: 1px 5px;
    border-radius: 4px; font-variant-numeric: tabular-nums; white-space: nowrap;
  }
  .orizz .rig-pos { left: 4px; top: 50%; transform: translateY(-50%); }
  .vert .rig-pos { top: 4px; left: 50%; transform: translateX(-50%); }

  .disegno { position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none; }
  .disegno polyline { stroke: #ff3b30; stroke-width: 2; fill: none; }
  .area-box { position: absolute; border: 2px solid #ff3b30; background: rgba(255, 59, 48, 0.12); pointer-events: none; }
  .punto {
    position: absolute; width: 12px; height: 12px; margin: -6px 0 0 -6px; border-radius: 50%;
    background: #ff3b30; border: 2px solid #fff; box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.4); pointer-events: none;
  }
  .punto-eti {
    position: absolute; left: 12px; top: -6px; font-size: 12px; font-weight: 700; color: #fff;
    background: #ff3b30; border-radius: 4px; padding: 0 5px; line-height: 16px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.5);
  }

  /* Calibro: ganasce trascinabili + banda di apertura */
  .ganascia {
    position: absolute; background: #2563eb; box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.5);
    touch-action: none; z-index: 6;
  }
  .ganascia.orizz { top: 0; bottom: 0; width: 3px; transform: translateX(-50%); cursor: ew-resize; }
  .ganascia.vert { left: 0; right: 0; height: 3px; transform: translateY(-50%); cursor: ns-resize; }
  .ganascia:focus-visible { outline: 2px solid #fff; }
  .gan-eti {
    position: absolute; background: #2563eb; color: #fff; font-size: 11px; font-weight: 700;
    padding: 1px 5px; border-radius: 4px; line-height: 1.3;
  }
  .ganascia.orizz .gan-eti { top: 6px; left: 4px; }
  .ganascia.vert .gan-eti { left: 6px; top: 4px; }
  .cal-dim { position: absolute; pointer-events: none; z-index: 5; }
  .cal-dim.orizz { top: 50%; height: 0; border-top: 2px dashed #2563eb; transform: translateY(-50%); }
  .cal-dim.vert { left: 50%; width: 0; border-left: 2px dashed #2563eb; transform: translateX(-50%); }
  .cal-val {
    position: absolute; left: 50%; top: 50%; transform: translate(-50%, -50%);
    background: #2563eb; color: #fff; font-size: 12px; font-weight: 600; padding: 2px 8px;
    border-radius: 5px; white-space: nowrap; font-variant-numeric: tabular-nums;
  }

  /* Pannello — layout pulito, niente sovrapposizioni */
  .pannello {
    position: absolute; top: 14px; right: 14px; width: 256px;
    display: flex; flex-direction: column; gap: 9px;
    padding: 14px 16px; background: rgba(22, 22, 26, 0.94); color: #fff;
    border-radius: 12px; box-shadow: 0 8px 28px rgba(0, 0, 0, 0.55);
    font-size: 14px; pointer-events: auto; cursor: default;
    backdrop-filter: blur(3px);
  }
  .pannello .tit { font-size: 12px; text-transform: uppercase; letter-spacing: 0.06em; opacity: 0.7; font-weight: 600; }
  .modi { display: flex; gap: 5px; }
  .modi button {
    flex: 1; background: rgba(255, 255, 255, 0.1); color: #fff; border: 1px solid rgba(255, 255, 255, 0.18);
    border-radius: 7px; padding: 7px 2px; cursor: pointer; font-size: 12.5px;
  }
  .modi button.on { background: var(--accento, #3b82f6); border-color: transparent; }
  .riga2 { display: flex; align-items: center; gap: 8px; }
  .riga2 select {
    flex: 1; background: rgba(255, 255, 255, 0.1); color: #fff;
    border: 1px solid rgba(255, 255, 255, 0.2); border-radius: 7px; padding: 6px 8px; font-size: 13px;
  }
  .orient { display: flex; gap: 5px; }
  .orient button {
    width: 34px; background: rgba(255, 255, 255, 0.1); color: #fff;
    border: 1px solid rgba(255, 255, 255, 0.2); border-radius: 7px; padding: 6px 0; cursor: pointer; font-size: 15px;
  }
  .orient button.on { background: var(--accento, #3b82f6); border-color: transparent; }
  .val { display: flex; align-items: baseline; gap: 8px; }
  .val .k { opacity: 0.6; font-size: 12px; min-width: 60px; }
  .val b { font-variant-numeric: tabular-nums; font-size: 17px; }
  .val.grande b { font-size: 22px; color: #ffd666; }
  .val .u { opacity: 0.7; font-size: 13px; }
  .segmenti {
    display: flex; flex-direction: column; gap: 3px; max-height: 168px; overflow-y: auto;
    padding: 6px 0; border-top: 1px solid rgba(255, 255, 255, 0.14);
  }
  .seg { display: flex; align-items: baseline; gap: 8px; font-size: 13px; }
  .seg-n {
    min-width: 46px; font-weight: 700; font-size: 12px; color: #ffd666;
    font-variant-numeric: tabular-nums;
  }
  .seg b { flex: 1; font-variant-numeric: tabular-nums; }
  .seg .u { opacity: 0.55; font-size: 11px; }
  .azioni { display: flex; gap: 6px; }
  .hint { opacity: 0.78; font-size: 12.5px; line-height: 1.4; }
  .dim {
    margin-top: 2px; padding-top: 8px; border-top: 1px solid rgba(255, 255, 255, 0.18);
    font-size: 11px; opacity: 0.65; line-height: 1.45;
  }
  .pannello button:not(.modi button):not(.orient button) {
    flex: 1; margin-top: 0; background: rgba(255, 255, 255, 0.14); color: #fff;
    border: 1px solid rgba(255, 255, 255, 0.25); border-radius: 7px; padding: 7px 8px; cursor: pointer; font-size: 12.5px;
  }
  .pannello button:not(.modi button):not(.orient button):hover { background: rgba(255, 255, 255, 0.22); }
</style>
