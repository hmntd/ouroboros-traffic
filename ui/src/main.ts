import './style.css';
import init, { Game } from '../../pkg/ouroboros_traffic.js';

interface Stats {
  time: number;
  vehicles: number;
  citizens: number;
  peds: number;
  buildings: number;
  parked: number;
  tripsCar: number;
  tripsWalk: number;
  spawnRate: number;
  avgSpeed: number;
  speedRatio: number;
  flow: number;
  jammedEdges: number;
  maxJam: number;
  queued: number;
  blocked: number;
  removed: number;
  arrived: number;
  phase: string;
  progress: number;
  candidates: number;
  generation: number;
  mods: number[];
  expansions: number;
  changesCount: number;
  isMaxCity: boolean;
  maxNodes: number;
  nodes: number;
  edges: number;
  hVeh: number[];
  hSpeed: number[];
  hFlow: number[];
  log: string[];
}

interface ChangeItem {
  id: number;
  time: number;
  kind: number; // 0: Expand, 1: Widen, 2: Roundabout, 3: Bypass
  title: string;
  desc: string;
  x: number;
  y: number;
  radius: number;
}

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const canvas = $<HTMLCanvasElement>('sim-canvas');

function resize() {
  const dpr = window.devicePixelRatio || 1;
  canvas.width = Math.floor(window.innerWidth * dpr);
  canvas.height = Math.floor(window.innerHeight * dpr);
}
window.addEventListener('resize', resize);
resize();

function fmtTime(t: number) {
  const m = Math.floor(t / 60);
  return `${m}:${String(Math.floor(t % 60)).padStart(2, '0')}`;
}

function drawChart(c: HTMLCanvasElement, s: Stats) {
  const ctx = c.getContext('2d')!;
  ctx.clearRect(0, 0, c.width, c.height);
  const series: [number[], string][] = [
    [s.hVeh, '#8ecae6'],
    [s.hSpeed, '#ffd166'],
    [s.hFlow, '#b5e48c'],
  ];
  for (const [data, color] of series) {
    if (data.length < 2) continue;
    const max = Math.max(...data, 1);
    ctx.beginPath();
    data.forEach((v, i) => {
      const x = (i / 149) * c.width;
      const y = c.height - 4 - (v / max) * (c.height - 8);
      i ? ctx.lineTo(x, y) : ctx.moveTo(x, y);
    });
    ctx.strokeStyle = color;
    ctx.lineWidth = 1.5;
    ctx.stroke();
  }
}

let activeChangeId: number | null = null;
let lastRenderedChangesCount = -1;

const TAG_CONFIG = [
  { label: 'EXPANSION', class: 'tag-expand' },
  { label: 'WIDEN', class: 'tag-widen' },
  { label: 'ROUNDABOUT', class: 'tag-roundabout' },
  { label: 'BYPASS', class: 'tag-bypass' },
];

function renderChanges(changes: ChangeItem[], game: Game) {
  if (changes.length === lastRenderedChangesCount) {
    return;
  }
  lastRenderedChangesCount = changes.length;

  $('changes-count').textContent = `${changes.length} updates`;
  const list = $('changes-list');
  list.innerHTML = '';

  for (const item of changes) {
    const li = document.createElement('li');
    li.className = `change-card${activeChangeId === item.id ? ' active' : ''}`;
    const tag = TAG_CONFIG[item.kind] || { label: 'CHANGE', class: 'tag-widen' };

    li.innerHTML = `
      <div class="card-header">
        <span class="card-tag ${tag.class}">${tag.label}</span>
        <span class="card-time">${fmtTime(item.time)}</span>
      </div>
      <div class="card-title">${item.title}</div>
      <div class="card-desc">${item.desc}</div>
    `;

    li.onclick = () => {
      activeChangeId = item.id;
      game.focus_change(item.id);
      document
        .querySelectorAll('.change-card')
        .forEach((el) => el.classList.remove('active'));
      li.classList.add('active');
    };

    list.appendChild(li);
  }
}

function render(s: Stats) {
  $('s-time').textContent = fmtTime(s.time);
  $('s-gen').textContent = String(s.generation);
  $('s-veh').textContent = String(s.vehicles);
  $('s-cit').textContent = String(s.citizens);
  $('s-ped').textContent = String(s.peds);
  $('s-bldg').textContent = String(s.buildings);
  $('s-park').textContent = String(s.parked);
  $('s-speed').textContent = s.avgSpeed.toFixed(1);
  $('s-flow').textContent = String(s.flow);
  $('s-jam').textContent = String(s.jammedEdges);

  $('mod-exp').textContent = String(s.expansions);
  $('mod-a').textContent = String(s.mods[0]);
  $('mod-b').textContent = String(s.mods[1]);
  $('mod-c').textContent = String(s.mods[2]);
  $('s-nodes').textContent = String(s.nodes);
  const maxLabel = $('s-nodes-max');
  if (maxLabel) {
    maxLabel.textContent = s.isMaxCity ? 'MAX' : `Nodes (≤${s.maxNodes})`;
    maxLabel.classList.toggle('max-reached', s.isMaxCity);
  }

  const ph = $('phase');
  ph.textContent = s.phase;
  ph.className =
    'badge' +
    (s.phase.startsWith('Shadow')
      ? ' analysis'
      : s.phase === 'Recovering'
        ? ' recover'
        : '');

  const cong = Math.min(1, s.maxJam / 0.6) * 100;
  $('m-cong').style.width = `${cong}%`;
  $('analysis').classList.toggle('hidden', !s.phase.startsWith('Shadow'));
  $('m-prog').style.width = `${s.progress * 100}%`;

  $('log').innerHTML = s.log
    .map((l) => {
      const cls = l.includes('BUILD')
        ? 'build'
        : l.includes('JAM')
          ? 'jam'
          : l.includes('expanded')
            ? 'build'
            : '';
      return `<li class="${cls}">${l}</li>`;
    })
    .join('');

  drawChart($<HTMLCanvasElement>('chart'), s);
}

async function main() {
  await init();
  let game = new Game('sim-canvas', (Math.random() * 1e9) | 0);

  let paused = false;
  const play = $('btn-play');
  const togglePause = () => {
    paused = !paused;
    game.set_paused(paused);
    play.textContent = paused ? '▶ Play' : '⏸ Pause';
  };
  play.onclick = togglePause;
  window.addEventListener('keydown', (e) => {
    if (e.code === 'Space') {
      e.preventDefault();
      togglePause();
    }
  });

  document.querySelectorAll<HTMLButtonElement>('#speeds button').forEach((b) => {
    b.onclick = () => {
      game.set_speed(Number(b.dataset.speed));
      document
        .querySelectorAll('#speeds button')
        .forEach((x) => x.classList.remove('active'));
      b.classList.add('active');
    };
  });

  const cb = (id: string, fn: (v: boolean) => void) => {
    const el = $<HTMLInputElement>(id);
    el.onchange = () => fn(el.checked);
  };
  cb('cb-heat', (v) => game.set_show_heat(v));
  cb('cb-nodes', (v) => game.set_show_nodes(v));
  cb('cb-paths', (v) => game.set_show_paths(v));
  cb('cb-solver', (v) => game.set_solver_enabled(v));

  $('btn-view').onclick = () => game.reset_view();
  $('btn-new').onclick = () => {
    lastRenderedChangesCount = -1;
    activeChangeId = null;
    game.reset((Math.random() * 1e9) | 0);
  };

  let dragging = false;
  canvas.addEventListener('pointerdown', (e) => {
    dragging = true;
    canvas.classList.add('drag');
    canvas.setPointerCapture(e.pointerId);
  });
  canvas.addEventListener('pointerup', () => {
    dragging = false;
    canvas.classList.remove('drag');
  });
  canvas.addEventListener('pointermove', (e) => {
    if (dragging) {
      const d = window.devicePixelRatio || 1;
      game.pan(e.movementX * d, e.movementY * d);
    }
  });
  canvas.addEventListener(
    'wheel',
    (e) => {
      e.preventDefault();
      const d = window.devicePixelRatio || 1;
      game.zoom_at(Math.exp(-e.deltaY * 0.0015), e.clientX * d, e.clientY * d);
    },
    { passive: false },
  );

  let last = performance.now();
  let lastStats = 0;
  let lastChanges = 0;

  const frame = (now: number) => {
    const dt = (now - last) / 1000;
    last = now;
    game.update(dt);
    game.draw();

    if (now - lastStats > 150) {
      lastStats = now;
      const stats = JSON.parse(game.stats_json()) as Stats;
      render(stats);
    }

    if (now - lastChanges > 600) {
      lastChanges = now;
      const changes = JSON.parse(game.changes_json()) as ChangeItem[];
      renderChanges(changes, game);
    }

    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
}

main();
