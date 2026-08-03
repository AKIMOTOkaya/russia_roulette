const elements = {
  setupPanel: document.querySelector("#setup-panel"),
  setupForm: document.querySelector("#setup-form"),
  gamePanel: document.querySelector("#game-panel"),
  connection: document.querySelector("#connection-status"),
  board: document.querySelector("#board"),
  players: document.querySelector("#players"),
  records: document.querySelector("#records"),
  turnTitle: document.querySelector("#turn-title"),
  revision: document.querySelector("#revision"),
  toast: document.querySelector("#toast"),
  newGame: document.querySelector("#new-game-button"),
};

const terrainSymbols = {
  empty: "",
  wall: "▤",
  crate: "▦",
  water: "≈",
  high_ground: "△",
  mine: "◆",
  medkit: "✚",
};

let view = null;
let mode = "move";
let busy = false;
let toastTimer = null;

elements.setupForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const seedText = document.querySelector("#seed").value.trim();
  await request("/api/game/new", {
    player_name: document.querySelector("#player-name").value,
    bot_count: Number(document.querySelector("#bot-count").value),
    seed: seedText === "" ? null : Number(seedText),
  });
});

elements.newGame.addEventListener("click", () => {
  elements.gamePanel.classList.add("hidden");
  elements.setupPanel.classList.remove("hidden");
  elements.connection.textContent = "等待开局";
});

document.querySelectorAll("[data-mode]").forEach((button) => {
  button.addEventListener("click", () => setMode(button.dataset.mode));
});

document.querySelectorAll("[data-direction]").forEach((button) => {
  button.addEventListener("click", () => sendDirection(button.dataset.direction));
});

document.querySelectorAll("[data-command]").forEach((button) => {
  button.addEventListener("click", () => sendCommand({ type: button.dataset.command }));
});

document.addEventListener("keydown", (event) => {
  if (event.target.matches("input, select") || !canAct()) return;
  const directions = { ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right" };
  if (directions[event.key]) {
    event.preventDefault();
    sendCommand({ type: event.shiftKey ? "shoot" : "move", direction: directions[event.key] });
  } else if (event.code === "Space") {
    event.preventDefault();
    sendCommand({ type: "wait" });
  }
});

function setMode(nextMode) {
  mode = nextMode;
  document.querySelectorAll("[data-mode]").forEach((button) => {
    button.classList.toggle("active", button.dataset.mode === mode);
  });
}

function sendDirection(direction) {
  sendCommand({ type: mode, direction });
}

async function sendCommand(command) {
  if (!canAct()) return;
  await request("/api/game/command", { expected_revision: view.revision, command });
}

async function request(url, body) {
  if (busy) return;
  setBusy(true);
  try {
    const response = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    const data = await response.json();
    if (!response.ok) throw new Error(data.error || "请求失败");
    view = data;
    render();
  } catch (error) {
    showToast(error.message);
  } finally {
    setBusy(false);
  }
}

function render() {
  elements.setupPanel.classList.add("hidden");
  elements.gamePanel.classList.remove("hidden");
  elements.connection.textContent = view.status.state === "running" ? `种子 ${view.seed}` : "对局结束";
  elements.revision.textContent = `REV ${view.revision}`;
  renderBoard();
  renderPlayers();
  renderRecords();

  if (view.status.state === "finished") {
    const winner = view.players.find((player) => player.id === view.status.winner_id);
    elements.turnTitle.textContent = winner ? `${winner.name} 获胜` : "无人幸存";
  } else {
    const active = view.players.find((player) => player.id === view.current_player_id);
    elements.turnTitle.textContent = active?.kind === "human" ? "轮到你行动" : `${active?.name || "Bot"} 行动中`;
  }
  updateControls();
}

function renderBoard() {
  elements.board.style.setProperty("--board-size", view.map_size);
  const playersByCell = new Map();
  view.players.filter((player) => player.status === "alive").forEach((player) => {
    playersByCell.set(`${player.position.x}:${player.position.y}`, player);
  });

  elements.board.replaceChildren(...view.cells.map((cell) => {
    const node = document.createElement("div");
    node.className = `cell ${cell.terrain}`;
    node.title = `(${cell.position.x}, ${cell.position.y}) · ${terrainName(cell.terrain)}`;
    node.textContent = terrainSymbols[cell.terrain] || "";
    const player = playersByCell.get(`${cell.position.x}:${cell.position.y}`);
    if (player) {
      const token = document.createElement("span");
      token.className = `player-token ${player.kind} ${player.id === view.current_player_id ? "active" : ""}`;
      token.textContent = player.kind === "human" ? "你" : String(player.id);
      token.title = player.name;
      node.append(token);
    }
    return node;
  }));
}

function renderPlayers() {
  elements.players.replaceChildren(...view.players.map((player) => {
    const card = document.createElement("div");
    card.className = `player-card ${player.id === view.current_player_id ? "active" : ""} ${player.status}`;
    const badge = player.has_shield ? " · 护盾" : "";
    card.innerHTML = `<div class="player-name"><span>${escapeHtml(player.name)}</span><span>${player.kind === "human" ? "YOU" : `#${player.id}`}</span></div><div class="player-meta">${player.status === "alive" ? "存活" : "出局"}${badge} · 连射 ${player.consecutive_shots}</div>`;
    return card;
  }));
}

function renderRecords() {
  elements.records.replaceChildren(...[...view.records].reverse().map((record) => {
    const item = document.createElement("li");
    item.className = `record ${record.category}`;

    const badge = document.createElement("span");
    badge.className = "record-badge";
    badge.textContent = ({ action: "行动", event: "事件", turn: "回合", notification: "通知" })[record.category];

    const content = document.createElement("span");
    content.className = "record-content";
    content.textContent = recordText(record);

    const sequence = document.createElement("span");
    sequence.className = "record-sequence";
    sequence.textContent = `#${record.sequence}`;

    item.append(badge, content, sequence);
    return item;
  }));
}

function recordText(record) {
  const name = playerName;
  switch (record.category) {
    case "action": return `${name(record.actor_id)} · ${commandText(record.command)}`;
    case "event": return eventText(record.event);
    case "turn": return `第 ${record.round} 回合 · 轮到 ${name(record.player_id)}`;
    case "notification": return notificationText(record.notification);
    default: return record.category;
  }
}

function eventText(event) {
  const name = playerName;
  switch (event.type) {
    case "moved": return `${name(event.actor_id)} 移动到 (${event.to.x}, ${event.to.y})`;
    case "move_blocked": return `${name(event.actor_id)} 的移动被阻挡`;
    case "elbow_duel": return `${name(event.attacker_id)} 与 ${name(event.defender_id)} 发生肘击，${name(event.winner_id)} 胜出`;
    case "shot_fired": return `${name(event.actor_id)} 向${directionName(event.direction)}射击`;
    case "empty_chamber": return `${name(event.actor_id)} 扣动扳机——空膛`;
    case "shot_missed": return `${name(event.actor_id)} 的子弹没有命中`;
    case "shield_consumed": return `${name(event.player_id)} 的护盾挡下致命伤害`;
    case "player_eliminated": return `${name(event.player_id)} 出局（${causeName(event.cause)}）`;
    case "terrain_changed": return `(${event.position.x}, ${event.position.y}) 的地形发生变化`;
    case "item_collected": return `${name(event.player_id)} 获得护盾`;
    default: return event.type;
  }
}

function commandText(command) {
  switch (command.type) {
    case "move": return `向${directionName(command.direction)}移动`;
    case "shoot": return `向${directionName(command.direction)}射击`;
    case "wait": return "等待";
    case "suicide": return "结束自己";
    default: return command.type;
  }
}

function notificationText(notification) {
  switch (notification.type) {
    case "match_started": return `对局开始 · 随机种子 ${notification.seed}`;
    case "match_finished": return notification.winner_id ? `${playerName(notification.winner_id)} 成为最后幸存者` : "对局结束 · 无人幸存";
    default: return notification.type;
  }
}

function playerName(id) {
  return view.players.find((player) => player.id === id)?.name || `玩家 ${id}`;
}

function canAct() {
  if (!view || busy || view.status.state !== "running") return false;
  return view.players.some((player) => player.id === view.current_player_id && player.kind === "human" && player.status === "alive");
}

function setBusy(nextBusy) {
  busy = nextBusy;
  updateControls();
}

function updateControls() {
  const disabled = !canAct();
  document.querySelectorAll(".controls button").forEach((button) => { button.disabled = disabled; });
  elements.newGame.disabled = busy;
  const submit = elements.setupForm.querySelector("button[type=submit]");
  submit.disabled = busy;
  submit.textContent = busy ? "正在开桌…" : "开始新对局";
}

function showToast(message) {
  elements.toast.textContent = message;
  elements.toast.classList.remove("hidden");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => elements.toast.classList.add("hidden"), 4200);
}

function escapeHtml(value) {
  return value.replace(/[&<>'"]/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", "'": "&#39;", '"': "&quot;" })[character]);
}

function directionName(direction) { return ({ up: "上", down: "下", left: "左", right: "右" })[direction]; }
function causeName(cause) { return ({ shot: "射击", elbow_duel: "肘击", mine: "地雷", drowned: "溺水", suicide: "主动结束" })[cause] || cause; }
function terrainName(terrain) { return ({ empty: "空地", wall: "墙", crate: "木箱", water: "水域", high_ground: "高地", mine: "地雷", medkit: "护盾" })[terrain]; }

fetch("/api/game")
  .then((response) => response.ok ? response.json() : null)
  .then((game) => { if (game) { view = game; render(); } })
  .catch(() => showToast("无法连接本地游戏服务"));
