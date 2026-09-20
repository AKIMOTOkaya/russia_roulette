let tabId = sessionStorage.getItem("roulette_tab_id");
if (!tabId) {
  tabId = crypto.randomUUID?.() || `tab-${Date.now()}-${Math.random().toString(16).slice(2)}`;
  sessionStorage.setItem("roulette_tab_id", tabId);
}
const pageInstanceId = crypto.randomUUID?.() || `page-${Date.now()}-${Math.random().toString(16).slice(2)}`;
const identityChannel = "BroadcastChannel" in window ? new BroadcastChannel("roulette-tab-identity") : null;
const identityReady = new Promise((resolve) => {
  if (!identityChannel) {
    resolve();
    return;
  }
  identityChannel.addEventListener("message", (event) => {
    const message = event.data;
    if (message?.tab_id !== tabId || message.instance_id === pageInstanceId) return;
    if (message.type === "probe") {
      identityChannel.postMessage({ type: "present", tab_id: tabId, instance_id: pageInstanceId, target: message.instance_id });
    } else if (message.type === "present" && message.target === pageInstanceId) {
      tabId = crypto.randomUUID?.() || `tab-${Date.now()}-${Math.random().toString(16).slice(2)}`;
      sessionStorage.setItem("roulette_tab_id", tabId);
      sessionStorage.removeItem("roulette_room_id");
      roomId = null;
    }
  });
  identityChannel.postMessage({ type: "probe", tab_id: tabId, instance_id: pageInstanceId });
  setTimeout(resolve, 80);
});

const elements = Object.fromEntries([
  "landing-panel", "lobby-panel", "room-panel", "brand-home", "browse-lobby", "back-to-landing",
  "connection-dot", "connection-status", "tab-identity", "settings-tab-identity", "landing-room-count",
  "room-count", "create-room-form", "join-room-form", "create-password-enabled",
  "create-password-field", "room-list", "empty-lobby", "refresh-lobby", "founder-auth-form",
  "founder-badge", "lan-toggle", "lan-description", "room-code", "room-title", "room-phase",
  "room-password", "leave-room", "member-count", "members", "add-bot", "start-room",
  "dissolve-room", "room-hint", "waiting-room", "game-workspace", "room-seed", "board",
  "records", "turn-title", "revision", "weather-badge", "stalemate-badge", "solo-form", "toast",
  "step-panel", "next-step", "step-hint", "controls-shortcut",
  "header-open-event-debug", "open-event-debug", "event-debug-modal",
  "event-debug-content", "debug-trace-count", "refresh-debug-view",
  "sidebar-expand-debug", "sidebar-trace-count", "sidebar-event-debug-content",
  "room-event-inspector",
  "terrain-help-dialog", "terrain-help-list", "terrain-present-count",
].map((id) => [id.replaceAll("-", "_"), document.getElementById(id)]));

const terrainSymbols = { plain: "", empty: "", water: "≈", ice: "❄", high_ground: "△" };
const objectSymbols = { wall: "▤", crate: "▦", mine: "◆", shield: "✚", medkit: "✚" };

const defaultTerrainRules = [
  {
    terrain: "plain",
    name: "平地",
    symbol: "",
    description: "平整坚实的常规地面，无移动阻碍与特殊物理效果。"
  },
  {
    terrain: "water",
    name: "水域",
    symbol: "≈",
    description: "低洼深水区域，移动涉入时会遭受深水阻滞。若连续停留两回合将溺水淘汰。弹道直接掠过不受阻挡。暴风雪天气下相变为冰面。"
  },
  {
    terrain: "ice",
    name: "冰面",
    symbol: "❄",
    description: "极度光滑的低温冰面，踏入极易触发滑行冲刺或失控打滑。离开冰面蹬地施力有 30% 概率震碎薄冰使其相变为深水。热浪下融化为水。"
  },
  {
    terrain: "high_ground",
    name: "高地",
    symbol: "△",
    description: "开阔的战术制高点，居高临下视野极佳。占据高地射击时有效射程额外增加 1 格。弹道可掠过高地。"
  }
];

const defaultObjectRules = [
  {
    object: "wall",
    kind: "cover",
    name: "墙体",
    symbol: "▤",
    hardness: 2,
    blocks_movement: true,
    blocks_bullets: true,
    description: "坚硬砖石掩体，阻挡角色移动与普通弹道。普通子弹无法击穿；高温穿甲弹可直接击碎消除。撞击时有极高概率致死。消除后底层地面完好保留。"
  },
  {
    object: "crate",
    kind: "cover",
    name: "木箱",
    symbol: "▦",
    hardness: 1,
    blocks_movement: true,
    blocks_bullets: true,
    description: "轻质木制掩体，阻挡角色移动。普通子弹可击碎破坏后停下；穿甲弹击碎后可继续贯穿前行。击碎时可能掉落物资。消除后底层地面完好保留。"
  },
  {
    object: "mine",
    kind: "trap",
    name: "地雷",
    symbol: "◆",
    hardness: null,
    blocks_movement: false,
    blocks_bullets: false,
    description: "触发式暗雷陷阱。子弹从上方飞掠不受阻挡；角色踩入进入引爆判定。触发后暗雷清除，底层地面完好保留。"
  },
  {
    object: "shield",
    kind: "pickup",
    name: "护盾",
    symbol: "✚",
    hardness: null,
    blocks_movement: false,
    blocks_bullets: false,
    description: "单兵便携充能护盾补给。子弹飞掠不受阻挡；角色进入时拾取激活护盾，完全抵消一次致命伤害；拾取后道具清除，底层地面完好保留。"
  }
];

let cachedTerrainRules = null;
let cachedObjectRules = null;

async function loadTerrainRules() {
  try {
    const [tRes, oRes] = await Promise.all([
      fetch("/api/rules/terrains"),
      fetch("/api/rules/objects"),
    ]);
    if (tRes.ok) cachedTerrainRules = await tRes.json();
    if (oRes.ok) cachedObjectRules = await oRes.json();
  } catch {}
}

let room = null;
let roomId = sessionStorage.getItem("roulette_room_id");
let mode = "move";
let operationBusy = false;
let pollBusy = false;
let toastTimer = null;
let surface = "landing";

document.querySelectorAll("[data-open-dialog]").forEach((button) => {
  button.addEventListener("click", () => openDialog(button.dataset.openDialog));
});
document.querySelectorAll("[data-close-dialog]").forEach((button) => {
  button.addEventListener("click", () => button.closest("dialog").close());
});
document.querySelectorAll("dialog").forEach((dialog) => {
  dialog.addEventListener("click", (event) => { if (event.target === dialog) dialog.close(); });
});

elements.brand_home.addEventListener("click", showLanding);
elements.back_to_landing.addEventListener("click", showLanding);
elements.browse_lobby.addEventListener("click", showLobby);
elements.header_open_event_debug?.addEventListener("click", toggleEventDebugModal);
elements.open_event_debug?.addEventListener("click", toggleEventDebugModal);
elements.refresh_debug_view?.addEventListener("click", renderEventDebugModal);
elements.sidebar_expand_debug?.addEventListener("click", toggleEventDebugModal);

elements.create_password_enabled.addEventListener("change", () => {
  const enabled = elements.create_password_enabled.checked;
  elements.create_password_field.classList.toggle("hidden", !enabled);
  document.getElementById("create-room-password").required = enabled;
});

elements.create_room_form.addEventListener("submit", async (event) => {
  event.preventDefault();
  const playerName = value("create-player-name");
  rememberPlayerName(playerName);
  const created = await mutate("/api/rooms", {
    tab_id: tabId,
    room_name: value("room-name"),
    player_name: playerName,
    password: elements.create_password_enabled.checked ? value("create-room-password") : null,
  });
  if (created) enterRoom(created);
});

elements.solo_form.addEventListener("submit", async (event) => {
  event.preventDefault();
  const playerName = value("solo-player-name");
  const botCount = Number(value("solo-bot-count"));
  const seedText = value("solo-seed");
  rememberPlayerName(playerName);
  const created = await mutate("/api/rooms", {
    tab_id: tabId,
    room_name: "单人模拟",
    player_name: playerName,
    password: null,
  });
  if (!created) return;
  enterRoom(created);
  for (let index = 0; index < botCount; index += 1) {
    if (!await roomMutation("bots/add", { tab_id: tabId })) return;
  }
  await roomMutation("start", { tab_id: tabId, seed: seedText ? Number(seedText) : null });
});

elements.join_room_form.addEventListener("submit", async (event) => {
  event.preventDefault();
  const playerName = value("join-player-name");
  rememberPlayerName(playerName);
  const joined = await mutate("/api/rooms/join", {
    tab_id: tabId,
    room_id: value("join-room-code").toUpperCase(),
    player_name: playerName,
    password: value("join-room-password") || null,
  });
  if (joined) enterRoom(joined);
});

elements.founder_auth_form.addEventListener("submit", async (event) => {
  event.preventDefault();
  const result = await mutate("/api/founder/auth", { tab_id: tabId, password: value("founder-password") });
  if (result?.authenticated) {
    document.getElementById("founder-password").value = "";
    showToast("创始人身份已在当前标签页生效");
    await loadServerSettings();
  }
});

elements.lan_toggle.addEventListener("change", async () => {
  const settings = await mutate("/api/server/settings", { tab_id: tabId, expose_lan: elements.lan_toggle.checked });
  if (settings) renderServerSettings(settings);
});

elements.refresh_lobby.addEventListener("click", loadLobby);
elements.leave_room.addEventListener("click", async () => {
  if (!roomId) return;
  const lobby = await mutate(`/api/rooms/${encodeURIComponent(roomId)}/leave`, { tab_id: tabId });
  if (lobby) returnToLobby(lobby);
});
elements.add_bot.addEventListener("click", () => roomMutation("bots/add", { tab_id: tabId }));
elements.start_room.addEventListener("click", () => {
  const seedText = elements.room_seed.value.trim();
  roomMutation("start", { tab_id: tabId, seed: seedText ? Number(seedText) : null });
});
elements.dissolve_room.addEventListener("click", async () => {
  if (!confirm("确定解散这个房间？所有成员都会返回大厅。")) return;
  const lobby = await mutate(`/api/rooms/${encodeURIComponent(roomId)}/dissolve`, { tab_id: tabId });
  if (lobby) returnToLobby(lobby);
});
elements.next_step.addEventListener("click", triggerNextStep);

document.querySelectorAll("[data-mode]").forEach((button) => button.addEventListener("click", () => setMode(button.dataset.mode)));
document.querySelectorAll("[data-direction]").forEach((button) => button.addEventListener("click", () => sendDirection(button.dataset.direction)));
document.querySelectorAll("[data-command]").forEach((button) => button.addEventListener("click", () => sendCommand({ type: button.dataset.command })));

document.addEventListener("keydown", (event) => {
  if (event.key === "F2") {
    event.preventDefault();
    toggleEventDebugModal();
    return;
  }
  if (event.target.matches("input, select")) return;
  if (canStepBot()) {
    if (event.code === "Space" || event.key === "Enter") {
      event.preventDefault();
      triggerNextStep();
      return;
    }
  }
  if (!canAct()) return;
  const directions = { ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right" };
  if (directions[event.key]) {
    event.preventDefault();
    sendCommand({ type: event.shiftKey ? "shoot" : "move", direction: directions[event.key] });
  } else if (event.code === "Space") {
    event.preventDefault();
    sendCommand({ type: "wait" });
  }
});

async function loadInitialState() {
  setConnection("正在连接", false);
  await loadServerSettings();
  if (roomId) {
    const current = await getJson(`/api/rooms/${encodeURIComponent(roomId)}?tab_id=${encodeURIComponent(tabId)}`, true);
    if (current) {
      enterRoom(current);
      setConnection("服务已连接", true);
      return;
    }
  }
  await loadLobby();
}

async function loadLobby() {
  const lobby = await getJson(`/api/lobby?tab_id=${encodeURIComponent(tabId)}`);
  if (!lobby) return;
  if (lobby.current_room_id) {
    roomId = lobby.current_room_id;
    sessionStorage.setItem("roulette_room_id", roomId);
    const current = await getJson(`/api/rooms/${encodeURIComponent(roomId)}?tab_id=${encodeURIComponent(tabId)}`);
    if (current) enterRoom(current);
    return;
  }
  renderLobby(lobby);
  setConnection("服务已连接", true);
}

async function loadServerSettings() {
  const settings = await getJson(`/api/server/settings?tab_id=${encodeURIComponent(tabId)}`, true);
  if (settings) renderServerSettings(settings);
}

function renderServerSettings(settings) {
  if (settings.server_mode === "public") {
    elements.founder_badge.textContent = "公网服务器";
    elements.founder_badge.classList.add("verified");
    elements.lan_toggle.disabled = true;
    elements.lan_toggle.checked = true;
    elements.lan_description.textContent = "公网模式：开放公网反向代理流量";
  } else {
    elements.founder_badge.textContent = settings.founder_authenticated ? "已认证" : "未认证";
    elements.founder_badge.classList.toggle("verified", settings.founder_authenticated);
    elements.lan_toggle.disabled = !settings.founder_authenticated;
    elements.lan_toggle.checked = settings.lan_exposed;
    elements.lan_description.textContent = settings.lan_exposed ? "局域网设备当前可以访问" : "当前仅本机可访问";
  }
}


function renderLobby(lobby) {
  room = null;
  roomId = null;
  sessionStorage.removeItem("roulette_room_id");
  elements.room_panel.classList.add("hidden");
  const count = String(lobby.rooms.length).padStart(2, "0");
  elements.room_count.textContent = count;
  elements.landing_room_count.textContent = count;
  elements.room_list.replaceChildren(...lobby.rooms.map(roomCard));
  elements.empty_lobby.classList.toggle("hidden", lobby.rooms.length > 0);
  elements.landing_panel.classList.toggle("hidden", surface !== "landing");
  elements.lobby_panel.classList.toggle("hidden", surface !== "lobby");
}

function roomCard(summary) {
  const card = document.createElement("article");
  card.className = "room-card";
  const total = summary.human_count + summary.bot_count;
  card.innerHTML = `
    <div class="room-card-code">${escapeHtml(summary.id)}</div>
    <div class="room-card-body">
      <div><h3>${escapeHtml(summary.name)}</h3><p>${phaseName(summary.phase)} · ${summary.password_required ? "需要密码" : "公开房间"}</p></div>
      <div class="seat-count"><strong>${total}</strong><span>/ ${summary.capacity}<br />席位</span></div>
    </div>
    <div class="room-card-meta"><span>真人 ${summary.human_count}</span><span>BOT ${summary.bot_count}</span></div>`;
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = summary.is_member ? "返回房间" : summary.phase === "waiting" ? "加入房间" : "对局进行中";
  button.disabled = summary.phase !== "waiting" && !summary.is_member;
  button.addEventListener("click", async () => {
    if (summary.is_member) {
      const current = await getJson(`/api/rooms/${encodeURIComponent(summary.id)}?tab_id=${encodeURIComponent(tabId)}`);
      if (current) enterRoom(current);
    } else {
      document.getElementById("join-room-code").value = summary.id;
      openDialog("join-dialog");
    }
  });
  card.append(button);
  return card;
}

function enterRoom(nextRoom) {
  if (surface !== "room") {
    closeEntryDialogs();
    surface = "room";
  }
  room = nextRoom;
  roomId = nextRoom.id;
  sessionStorage.setItem("roulette_room_id", roomId);
  elements.landing_panel.classList.add("hidden");
  elements.lobby_panel.classList.add("hidden");
  elements.room_panel.classList.remove("hidden");
  renderRoom();
}

function renderRoom() {
  elements.room_code.textContent = room.id;
  elements.room_title.textContent = room.name;
  elements.room_phase.textContent = phaseName(room.phase);
  elements.room_password.textContent = room.password || "未设置";
  elements.member_count.textContent = `${room.members.length} / 6`;
  elements.members.replaceChildren(...room.members.map(memberCard));
  document.querySelectorAll(".owner-only").forEach((node) => node.classList.toggle("hidden", !room.is_owner));
  document.querySelectorAll(".waiting-only").forEach((node) => node.classList.toggle("hidden", room.phase !== "waiting"));
  const canStart = room.is_owner && room.phase === "waiting" && room.members.length >= 3;
  elements.start_room.disabled = !canStart || operationBusy;
  elements.add_bot.disabled = room.members.length >= 6 || operationBusy;
  elements.room_hint.textContent = room.phase === "waiting"
    ? room.members.length < 3 ? `还需要 ${3 - room.members.length} 名成员才能开始。` : room.is_owner ? "人数已满足，可以开始对局。" : "人数已满足，等待房主开始。"
    : room.phase === "playing" ? "离开进行中的对局后，你的席位会由机器人接管。" : "本局已经结束，房主仍可解散房间。";
  elements.waiting_room.classList.toggle("hidden", room.phase !== "waiting");
  elements.game_workspace.classList.toggle("hidden", !room.game);
  if (room.game) {
    renderGame();
  } else {
    renderEventDebugTo(elements.sidebar_event_debug_content, elements.sidebar_trace_count, true);
  }
  setConnection(`房间 ${room.id}`, true);
}

function memberCard(member) {
  const card = document.createElement("div");
  card.className = `member-card ${member.is_self ? "self" : ""}`;
  const gamePlayer = member.player_id == null ? null : room.game?.players.find((player) => player.id === member.player_id);
  const top = document.createElement("div");
  top.className = "member-main";
  const identity = document.createElement("div");
  identity.innerHTML = `<span class="member-avatar ${member.kind}">${member.kind === "bot" ? "B" : escapeHtml(member.name.slice(0, 1).toUpperCase())}</span><span><strong>${escapeHtml(member.name)}</strong><small>${member.kind === "bot" ? "随机机器人" : member.is_self ? "当前标签页" : "真人玩家"}</small></span>`;
  const badges = document.createElement("div");
  badges.className = "member-badges";
  if (member.is_owner) badges.append(makeBadge("房主", "owner"));
  if (gamePlayer) badges.append(makeBadge(gamePlayer.status === "alive" ? "存活" : "出局", gamePlayer.status));
  top.append(identity, badges);
  card.append(top);

  if (room.is_owner && room.phase === "waiting" && !member.is_self) {
    const actions = document.createElement("div");
    actions.className = "member-actions";
    const button = document.createElement("button");
    button.type = "button";
    if (member.kind === "bot") {
      button.textContent = "移除 BOT";
      button.addEventListener("click", () => roomMutation("bots/remove", { tab_id: tabId, member_id: member.id }));
    } else {
      button.textContent = "转让房主";
      button.addEventListener("click", () => roomMutation("owner", { tab_id: tabId, member_id: member.id }));
    }
    actions.append(button);
    card.append(actions);
  }
  return card;
}

function makeBadge(text, className) {
  const badge = document.createElement("span");
  badge.className = `member-badge ${className}`;
  badge.textContent = text;
  return badge;
}

async function roomMutation(path, body) {
  if (!roomId) return null;
  const result = await mutate(`/api/rooms/${encodeURIComponent(roomId)}/${path}`, body);
  if (result) enterRoom(result);
  return result;
}

function setMode(nextMode) {
  mode = nextMode;
  document.querySelectorAll("[data-mode]").forEach((button) => button.classList.toggle("active", button.dataset.mode === mode));
}

function sendDirection(direction) { sendCommand({ type: mode, direction }); }

async function sendCommand(command) {
  if (!canAct()) return;
  await roomMutation("command", { tab_id: tabId, expected_revision: room.game.revision, command });
}

function renderGame() {
  const game = room.game;
  elements.revision.textContent = `REV ${game.revision}`;
  if (elements.weather_badge) {
    elements.weather_badge.textContent = `天气：${weatherName(game.weather || "clear")}`;
  }
  if (elements.stalemate_badge) {
    const stalemateRounds = game.rounds_without_elimination || 0;
    if (stalemateRounds > 0) {
      elements.stalemate_badge.textContent = `🔥 僵局 ${stalemateRounds} 次未减员 (致死率提升)`;
      elements.stalemate_badge.classList.remove("hidden");
    } else {
      elements.stalemate_badge.classList.add("hidden");
    }
  }
  renderBoard(game);
  renderRecords(game);
  if (game.status.state === "finished") {
    const winner = game.players.find((player) => player.id === game.status.winner_id);
    elements.turn_title.textContent = winner ? `${winner.name} 获胜` : "无人幸存";
  } else {
    const active = activePlayer();
    if (active?.kind === "bot") {
      elements.turn_title.textContent = room.is_owner
        ? `${active.name} 行动中 · 点击下一步`
        : `${active.name} 行动中 · 等待房主推进`;
    } else {
      elements.turn_title.textContent = active?.id === game.human_player_id
        ? "轮到你行动 · 请选择意图"
        : `${active?.name || "其他玩家"} 行动中`;
    }
  }
  updateControls();
  renderEventDebugTo(elements.sidebar_event_debug_content, elements.sidebar_trace_count, true);
  if (elements.event_debug_modal?.open) {
    renderEventDebugTo(elements.event_debug_content, elements.debug_trace_count, false);
  }
}

function renderBoard(game) {
  elements.board.style.setProperty("--board-size", game.map_size);
  const playersByCell = new Map();
  game.players.filter((player) => player.status === "alive" && player.position).forEach((player) => playersByCell.set(`${player.position.x}:${player.position.y}`, player));
  elements.board.replaceChildren(...game.cells.map((cell) => {
    const node = document.createElement("div");
    node.className = `cell terrain-${cell.terrain} ${cell.terrain} ${cell.object ? `has-object object-${cell.object}` : ""}`;
    let tip = `(${cell.position.x}, ${cell.position.y}) · ${terrainName(cell.terrain)}`;
    if (cell.object) {
      tip += ` [${objectName(cell.object)}]`;
    }
    node.title = tip;

    if (cell.object) {
      const objEl = document.createElement("span");
      objEl.className = `map-object object-${cell.object}`;
      objEl.textContent = objectSymbols[cell.object] || "";
      node.append(objEl);
    } else if (terrainSymbols[cell.terrain]) {
      const terrEl = document.createElement("span");
      terrEl.className = "terrain-symbol";
      terrEl.textContent = terrainSymbols[cell.terrain];
      node.append(terrEl);
    }

    const player = playersByCell.get(`${cell.position.x}:${cell.position.y}`);
    if (player) {
      const token = document.createElement("span");
      token.className = `player-token ${player.kind} ${player.id === game.current_player_id ? "active" : ""} ${player.id === game.human_player_id ? "self" : ""}`;
      token.textContent = player.id === game.human_player_id ? "你" : player.kind === "bot" ? "B" : `P${player.id}`;
      token.title = player.name;
      node.append(token);
    }
    return node;
  }));
}

function groupRecordsIntoTurns(game) {
  const turns = [];
  let currentTurn = null;

  for (const record of game.records) {
    if (record.category === "turn") {
      if (currentTurn) {
        turns.push(currentTurn);
      }
      currentTurn = {
        type: "turn",
        round: record.round,
        player_id: record.player_id,
        sequence: record.sequence,
        action: null,
        items: [],
      };
    } else if (record.category === "notification") {
      if (!currentTurn) {
        turns.push({
          type: "notification",
          round: null,
          player_id: null,
          sequence: record.sequence,
          action: null,
          items: [record],
        });
      } else {
        currentTurn.items.push(record);
      }
    } else {
      if (!currentTurn) {
        currentTurn = {
          type: "turn",
          round: 1,
          player_id: record.actor_id || null,
          sequence: record.sequence,
          action: null,
          items: [],
        };
      }
      if (record.category === "action" && !currentTurn.action) {
        currentTurn.action = record;
        if (!currentTurn.player_id) {
          currentTurn.player_id = record.actor_id;
        }
      }
      currentTurn.items.push(record);
    }
  }

  if (currentTurn) {
    turns.push(currentTurn);
  }

  return turns;
}

function renderRecords(game) {
  const turns = groupRecordsIntoTurns(game);
  const reversedTurns = [...turns].reverse();

  elements.records.replaceChildren(...reversedTurns.map((turn) => {
    const card = document.createElement("li");
    card.className = `turn-frame-card ${turn.type === "notification" ? "notification-frame" : ""}`;

    const header = document.createElement("div");
    header.className = "turn-frame-header";

    const actorDiv = document.createElement("div");
    actorDiv.className = "turn-frame-actor";

    const metaDiv = document.createElement("div");
    metaDiv.className = "turn-frame-meta";

    if (turn.type === "notification") {
      const title = document.createElement("span");
      title.textContent = "📢 系统通知";
      actorDiv.append(title);
    } else {
      const player = game.players.find((p) => p.id === turn.player_id);
      const isBot = player?.kind === "bot";

      const roundSpan = document.createElement("span");
      roundSpan.className = "turn-frame-round";
      roundSpan.textContent = `第 ${turn.round} 回合`;

      const badge = document.createElement("span");
      badge.className = `turn-actor-badge ${isBot ? "bot" : "human"}`;
      badge.textContent = isBot ? "BOT" : "真人";

      const nameSpan = document.createElement("span");
      nameSpan.textContent = player?.name || `玩家 ${turn.player_id || "?"}`;

      actorDiv.append(roundSpan, badge, nameSpan);

      if (turn.action) {
        const actionChip = document.createElement("span");
        actionChip.className = "turn-action-chip";
        actionChip.textContent = commandText(turn.action.command);
        metaDiv.append(actionChip);
      }
    }

    const seqSpan = document.createElement("span");
    seqSpan.className = "record-sequence";
    seqSpan.textContent = `#${turn.sequence}`;
    metaDiv.append(seqSpan);

    header.append(actorDiv, metaDiv);
    card.append(header);

    const body = document.createElement("div");
    body.className = "turn-frame-body";

    if (turn.items.length === 0) {
      const emptyRow = document.createElement("div");
      emptyRow.className = "turn-record-empty";
      emptyRow.textContent = "等待执行行动...";
      body.append(emptyRow);
    } else {
      for (const record of turn.items) {
        const row = document.createElement("div");
        let extraClass = "";
        if (record.category === "event") {
          if (record.event.type === "player_eliminated") extraClass = " lethal";
          else if (record.event.type === "dramatic_event") extraClass = " dramatic";
        }
        row.className = `turn-record-row record ${record.category}${extraClass}`;

        const badge = document.createElement("span");
        badge.className = "record-badge";
        badge.textContent = ({ action: "行动", event: "事件", turn: "回合", notification: "通知" })[record.category];

        const content = document.createElement("span");
        content.className = "record-content";
        content.textContent = recordText(game, record);

        const rowSeq = document.createElement("span");
        rowSeq.className = "record-sequence";
        rowSeq.textContent = `#${record.sequence}`;

        row.append(badge, content, rowSeq);
        body.append(row);
      }
    }

    card.append(body);
    return card;
  }));
}

function recordText(game, record) {
  const name = (id) => game.players.find((player) => player.id === id)?.name || `玩家 ${id}`;
  switch (record.category) {
    case "action": return `${name(record.actor_id)} · ${commandText(record.command)}`;
    case "event": return eventText(record.event, name);
    case "turn": return `第 ${record.round} 回合 · 轮到 ${name(record.player_id)}`;
    case "notification": return notificationText(record.notification, name);
    default: return record.category;
  }
}

function eventText(event, name) {
  switch (event.type) {
    case "moved": return `位置变化 · ${name(event.actor_id)}：(${event.from.x}, ${event.from.y}) → (${event.to.x}, ${event.to.y})`;
    case "move_blocked": return `路径受阻 · ${name(event.actor_id)} 未能改变位置`;
    case "elbow_duel": return `${name(event.attacker_id)} 与 ${name(event.defender_id)} 发生肘击，${name(event.winner_id)} 胜出`;
    case "empty_chamber": return `枪膛状态 · ${name(event.actor_id)} 本次没有射出子弹`;
    case "shot_missed": return `弹道结果 · ${name(event.actor_id)} 的子弹未命中目标`;
    case "shield_consumed": return `${name(event.player_id)} 的护盾挡下致命伤害`;
    case "player_eliminated": return `${name(event.player_id)} 出局（${causeName(event.cause)}）`;
    case "terrain_changed": return `(${event.position.x}, ${event.position.y}) 的地形由 ${terrainName(event.from)} 变为 ${terrainName(event.to)}`;
    case "object_placed": return `(${event.position.x}, ${event.position.y}) 出现了 ${objectName(event.object)}`;
    case "object_destroyed": return `(${event.position.x}, ${event.position.y}) 处的 ${objectName(event.object)} 被击碎清除`;
    case "item_collected": return `${name(event.player_id)} 拾取了 ${objectName(event.item)}`;
    case "weather_changed": return `天气异变 · 环境由 ${weatherName(event.from)} 变为 ${weatherName(event.to)}`;
    case "dramatic_event": return `[波次 ${event.wave}] ${event.title} · ${event.narrative}`;
    default: return event.type;
  }
}

function commandText(command) {
  if (command.type === "move") return `向${directionName(command.direction)}移动`;
  if (command.type === "shoot") return `向${directionName(command.direction)}射击`;
  return command.type === "wait" ? "等待" : command.type === "suicide" ? "结束自己" : command.type;
}

function notificationText(notification, name) {
  if (notification.type === "match_started") return `对局开始 · 随机种子 ${notification.seed}`;
  if (notification.type === "match_finished") return notification.winner_id ? `${name(notification.winner_id)} 成为最后幸存者` : "对局结束 · 无人幸存";
  return notification.type;
}

function activePlayer() {
  const game = room?.game;
  if (!game) return null;
  return game.players.find((player) => player.id === game.current_player_id) || null;
}

function isBotTurn() {
  const game = room?.game;
  if (!game || game.status.state !== "running") return false;
  const active = activePlayer();
  return active?.kind === "bot";
}

function canAct() {
  const game = room?.game;
  return Boolean(game && !operationBusy && game.status.state === "running" && game.current_player_id === game.human_player_id);
}

function canStepBot() {
  return Boolean(room && isBotTurn() && room.is_owner && !operationBusy);
}

async function triggerNextStep() {
  if (!canStepBot()) return;
  await roomMutation("step", { tab_id: tabId, expected_revision: room.game.revision });
}

function updateControls() {
  const botTurn = isBotTurn();
  const humanAct = canAct();

  elements.step_panel.classList.toggle("hidden", !botTurn);
  document.querySelectorAll(".mode-switch, .direction-pad, .minor-actions").forEach((node) => {
    node.classList.toggle("hidden", botTurn);
  });

  if (botTurn) {
    const active = activePlayer();
    const botName = active?.name || "机器人";
    elements.next_step.disabled = !canStepBot();
    if (room.is_owner) {
      elements.step_hint.textContent = `轮到 ${botName} 行动，房主可点击“下一步”或按空格/回车推进`;
      elements.controls_shortcut.textContent = "空格 / 回车 / 点击推进机器人";
    } else {
      elements.step_hint.textContent = `轮到 ${botName} 行动，等待房主推进`;
      elements.controls_shortcut.textContent = "等待房主推进机器人回合";
    }
  } else {
    document.querySelectorAll(".controls button:not(.next-step-btn)").forEach((button) => {
      button.disabled = !humanAct;
    });
    elements.controls_shortcut.textContent = "方向键移动 · Shift + 方向键射击 · 空格等待";
  }
}

async function getJson(url, silent = false) {
  try {
    const response = await fetch(url);
    if (!response.ok) {
      const data = await safeJson(response);
      const error = new Error(data?.error || `请求失败（${response.status}）`);
      error.status = response.status;
      throw error;
    }
    return response.json();
  } catch (error) {
    setConnection("连接已中断", false);
    if (!silent) showToast(error.message);
    return null;
  }
}

async function mutate(url, body) {
  if (operationBusy) return null;
  operationBusy = true;
  document.body.classList.add("busy");
  updateControls();
  try {
    const response = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    const data = response.status === 204 ? {} : await safeJson(response);
    if (!response.ok) throw new Error(data?.error || `请求失败（${response.status}）`);
    return data;
  } catch (error) {
    showToast(error.message);
    return null;
  } finally {
    operationBusy = false;
    document.body.classList.remove("busy");
    updateControls();
  }
}

async function poll() {
  if (pollBusy || operationBusy) return;
  pollBusy = true;
  try {
    if (roomId) {
      const response = await fetch(`/api/rooms/${encodeURIComponent(roomId)}?tab_id=${encodeURIComponent(tabId)}`);
      if (response.ok) enterRoom(await response.json());
      else if (response.status === 403 || response.status === 404) await loadLobby();
      else setConnection("同步失败", false);
    } else {
      const response = await fetch(`/api/lobby?tab_id=${encodeURIComponent(tabId)}`);
      if (response.ok) renderLobby(await response.json());
      else setConnection("同步失败", false);
    }
  } catch (_) {
    setConnection("连接已中断", false);
  } finally {
    pollBusy = false;
  }
}

async function heartbeat() {
  try {
    await fetch("/api/session/heartbeat", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ tab_id: tabId }),
    });
  } catch (_) {
    setConnection("连接已中断", false);
  }
}

function returnToLobby(lobby) {
  surface = "lobby";
  closeDialogs();
  renderLobby(lobby);
  loadServerSettings();
}

function showLanding() {
  if (roomId) {
    showToast("请先离开当前房间");
    return;
  }
  surface = "landing";
  elements.room_panel.classList.add("hidden");
  elements.lobby_panel.classList.add("hidden");
  elements.landing_panel.classList.remove("hidden");
}

function showLobby() {
  if (roomId) return;
  surface = "lobby";
  elements.room_panel.classList.add("hidden");
  elements.landing_panel.classList.add("hidden");
  elements.lobby_panel.classList.remove("hidden");
  loadLobby();
}

function openDialog(id) {
  const dialog = document.getElementById(id);
  if (!dialog || dialog.open) return;
  closeDialogs();
  dialog.showModal();
  if (id === "settings-dialog") loadServerSettings();
  if (id === "event-debug-modal") renderEventDebugModal();
  if (id === "terrain-help-dialog") renderTerrainHelpModal();
}

function toggleEventDebugModal() {
  const modal = elements.event_debug_modal;
  if (!modal) return;
  if (modal.open) {
    modal.close();
  } else {
    openDialog("event-debug-modal");
  }
}

function renderEventDebugModal() {
  renderEventDebugTo(elements.event_debug_content, elements.debug_trace_count, false);
}

function renderTerrainHelpModal() {
  const container = elements.terrain_help_list;
  if (!container) return;

  const game = room?.game;
  const terrainCounts = new Map();
  const objectCounts = new Map();
  if (game?.cells) {
    for (const cell of game.cells) {
      terrainCounts.set(cell.terrain, (terrainCounts.get(cell.terrain) || 0) + 1);
      if (cell.object) {
        objectCounts.set(cell.object, (objectCounts.get(cell.object) || 0) + 1);
      }
    }
  }

  const terrains = [...(cachedTerrainRules || defaultTerrainRules)];
  const objects = [...(cachedObjectRules || defaultObjectRules)];

  const presentTerrainCount = [...terrainCounts.entries()].filter(([t, count]) => t !== "plain" && t !== "empty" && count > 0).length;
  const presentObjectCount = [...objectCounts.entries()].filter(([, count]) => count > 0).length;

  if (elements.terrain_present_count) {
    elements.terrain_present_count.textContent = game
      ? `当前地图含 ${presentTerrainCount} 种特殊地面，${presentObjectCount} 种上覆物体`
      : `图鉴含 4 种地面地形规范与 4 种地图物体规范`;
  }

  const nodes = [];

  // 1. SECTION: GROUND TERRAINS
  const sec1 = document.createElement("div");
  sec1.className = "guide-section-title";
  sec1.textContent = "▶ 基础地面地形 (GROUND TERRAINS)";
  nodes.push(sec1);

  for (const t of terrains) {
    const count = terrainCounts.get(t.terrain) || 0;
    const isPresent = count > 0;
    const card = document.createElement("article");
    card.className = `terrain-card ${isPresent ? "present" : "absent"} terrain-kind-${t.terrain}`;

    const header = document.createElement("div");
    header.className = "terrain-card-header";

    const titleGroup = document.createElement("div");
    titleGroup.className = "terrain-title-group";

    const symbolBox = document.createElement("span");
    symbolBox.className = `terrain-symbol-box ${t.terrain}`;
    symbolBox.textContent = t.symbol || "▫";

    const nameEl = document.createElement("h3");
    nameEl.className = "terrain-card-name";
    nameEl.textContent = t.name;

    const engId = document.createElement("code");
    engId.className = "terrain-card-id";
    engId.textContent = t.terrain;

    titleGroup.append(symbolBox, nameEl, engId);

    const badge = document.createElement("span");
    badge.className = `terrain-present-badge ${isPresent ? "active" : "inactive"}`;
    badge.textContent = isPresent ? `🟢 当前存在 (${count} 格)` : "⚪ 当前未包含";

    header.append(titleGroup, badge);

    const propsGrid = document.createElement("div");
    propsGrid.className = "terrain-props-grid";

    const typeCol = document.createElement("div");
    typeCol.className = "terrain-prop-item";
    typeCol.innerHTML = `<span class="prop-label">元素类别</span><strong class="prop-val layer-above_ground">基础地貌</strong>`;

    const bulletCol = document.createElement("div");
    bulletCol.className = "terrain-prop-item";
    bulletCol.innerHTML = `<span class="prop-label">阻挡子弹</span><strong class="prop-val">否 (子弹掠过)</strong>`;

    const transCol = document.createElement("div");
    transCol.className = "terrain-prop-item";
    transCol.innerHTML = `<span class="prop-label">持久底表</span><strong class="prop-val">恒定地面</strong>`;

    propsGrid.append(typeCol, bulletCol, transCol);

    const bulletRule = document.createElement("div");
    bulletRule.className = "terrain-bullet-rule";
    bulletRule.innerHTML = `<span class="bullet-rule-tag">地貌规则</span><span class="bullet-rule-desc">${terrainRuleText(t)}</span>`;

    const desc = document.createElement("p");
    desc.className = "terrain-card-desc";
    desc.textContent = t.description;

    card.append(header, propsGrid, bulletRule, desc);
    nodes.push(card);
  }

  // 2. SECTION: MAP OBJECTS
  const sec2 = document.createElement("div");
  sec2.className = "guide-section-title";
  sec2.textContent = "▶ 地图物体与道具 (MAP OBJECTS & ITEMS)";
  nodes.push(sec2);

  for (const o of objects) {
    const count = objectCounts.get(o.object) || 0;
    const isPresent = count > 0;
    const card = document.createElement("article");
    card.className = `terrain-card ${isPresent ? "present" : "absent"} object-kind-${o.object}`;

    const header = document.createElement("div");
    header.className = "terrain-card-header";

    const titleGroup = document.createElement("div");
    titleGroup.className = "terrain-title-group";

    const symbolBox = document.createElement("span");
    symbolBox.className = `terrain-symbol-box object-${o.object}`;
    symbolBox.textContent = o.symbol || "▫";

    const nameEl = document.createElement("h3");
    nameEl.className = "terrain-card-name";
    nameEl.textContent = o.name;

    const engId = document.createElement("code");
    engId.className = "terrain-card-id";
    engId.textContent = o.object;

    titleGroup.append(symbolBox, nameEl, engId);

    const badge = document.createElement("span");
    badge.className = `terrain-present-badge ${isPresent ? "active" : "inactive"}`;
    badge.textContent = isPresent ? `🟢 当前存在 (${count} 处)` : "⚪ 当前未包含";

    header.append(titleGroup, badge);

    const propsGrid = document.createElement("div");
    propsGrid.className = "terrain-props-grid";

    const catCol = document.createElement("div");
    catCol.className = "terrain-prop-item";
    catCol.innerHTML = `<span class="prop-label">物体类别</span><strong class="prop-val">${objectKindName(o.kind)}</strong>`;

    const hardCol = document.createElement("div");
    hardCol.className = "terrain-prop-item";
    const hardnessText = o.hardness != null ? `${o.hardness}` : "none (无)";
    hardCol.innerHTML = `<span class="prop-label">物理硬度</span><strong class="prop-val">${hardnessText}</strong>`;

    const blkCol = document.createElement("div");
    blkCol.className = "terrain-prop-item";
    blkCol.innerHTML = `<span class="prop-label">阻挡弹道</span><strong class="prop-val">${o.blocks_bullets ? "是" : "否"}</strong>`;

    const destCol = document.createElement("div");
    destCol.className = "terrain-prop-item";
    destCol.innerHTML = `<span class="prop-label">破坏/消除</span><strong class="prop-val" style="color:var(--gold)">移除(保留地面)</strong>`;

    propsGrid.append(catCol, hardCol, blkCol, destCol);

    const bulletRule = document.createElement("div");
    bulletRule.className = "terrain-bullet-rule";
    bulletRule.innerHTML = `<span class="bullet-rule-tag">交互判定</span><span class="bullet-rule-desc">${objectRuleText(o)}</span>`;

    const desc = document.createElement("p");
    desc.className = "terrain-card-desc";
    desc.textContent = o.description;

    card.append(header, propsGrid, bulletRule, desc);
    nodes.push(card);
  }

  container.replaceChildren(...nodes);
}

function terrainRuleText(t) {
  if (t.terrain === "water") {
    return "【深水地表】弹道掠过不受阻挡；角色涉入遭受深水阻滞，连续停留 2 回合溺亡。暴风雪天气下水面结冰。";
  }
  if (t.terrain === "ice") {
    return "【光滑冰面】弹道掠过不受阻挡；踏入可能滑行冲刺或失控；离开施力有 30% 概率薄冰震碎相变为水。热浪下融化为水。";
  }
  if (t.terrain === "high_ground") {
    return "【战术高台】弹道掠过不受阻挡；占据高地射击时有效射程额外增加 1 格。";
  }
  return "【常规地面】平整开阔，弹道与角色移动无任何额外限制。";
}

function objectRuleText(o) {
  if (o.object === "wall") {
    return "【硬度 2 掩体】阻挡移动与普通弹；普通子弹无法破坏；<strong>穿甲重弹（穿甲能力 >= 2）可击碎消除墙体并贯穿前行！消除后底层地面完好保留。</strong>";
  }
  if (o.object === "crate") {
    return "【硬度 1 掩体】阻挡移动与普通弹；普通子弹击碎后停下；<strong>穿甲重弹击碎后继续贯通穿透！消除后底层地面完好保留，可能掉落物资。</strong>";
  }
  if (o.object === "mine") {
    return "【陷阱道具】不阻挡弹道；角色踩入触发踩雷生命周期判定（引爆或哑雷）；<strong>触发后暗雷清除，底层地面完好保留。</strong>";
  }
  if (o.object === "shield" || o.object === "medkit") {
    return "【战术补给】不阻挡弹道；角色踩入拾取激活护盾，抵消一次致命伤害；<strong>拾取后道具清除，底层地面完好保留。</strong>";
  }
  return "地图物体交互判定。";
}

function renderEventDebugTo(container, countElement, isSidebar = false) {
  if (!container) return;
  const game = room?.game;
  const traces = game?.event_traces || [];
  if (countElement) {
    countElement.textContent = isSidebar ? `${traces.length} 条执行流水` : `共 ${traces.length} 条流水`;
  }

  if (traces.length === 0) {
    const emptyMsg = room?.phase === "waiting"
      ? "等待开局...<br />对局开始后，权威后端将通过 BFS 树解析事件并在此处实时展示。"
      : "暂无事件执行流水。<br />进行移动、射击或进入第 2 回合时，权威后端将通过 BFS 树解析事件并在此处实时展示。";
    container.innerHTML = `<div class="debug-empty">${emptyMsg}</div>`;
    return;
  }

  // Render traces in reverse chronological order (newest first)
  const traceCards = [...traces].reverse().map(buildTraceCard);
  container.replaceChildren(...traceCards);
}

function buildTraceCard(trace) {
  const card = document.createElement("div");
  card.className = "trace-card";

  const header = document.createElement("div");
  header.className = "trace-header";
  header.innerHTML = `
    <div class="trace-title">
      <span class="trace-id-badge">#${trace.id}</span>
      <span class="trace-root-trigger">${escapeHtml(trace.root_trigger_desc)}</span>
    </div>
    <div class="trace-meta-info">
      第 ${trace.round} 轮 · ${trace.stalemate_rounds > 0 ? `<strong style="color: #ff7865;">🔥 僵局第 ${trace.stalemate_rounds} 轮</strong> · ` : ""}Rev ${trace.revision} · ${trace.nodes.length} 个树节点
    </div>
  `;

  const nodesContainer = document.createElement("div");
  nodesContainer.className = "trace-nodes-container";

  trace.nodes.forEach((node) => {
    const nodeEl = document.createElement("div");
    nodeEl.className = `tree-node ${node.wave === 0 ? "root-wave" : "child-wave"}`;
    nodeEl.style.setProperty("--indent", node.wave);

    const top = document.createElement("div");
    top.className = "tree-node-top";
    top.innerHTML = `
      <div style="display: flex; gap: 6px; align-items: center; flex-wrap: wrap;">
        <span class="wave-tag wave-${node.wave}">Wave ${node.wave}</span>
        <span class="pool-tag">${escapeHtml(node.pool_name)}</span>
        <span style="font-size: 0.6rem; color: #7f888c;">(路径深度代价: ${node.path_depth})</span>
      </div>
      <div class="roll-info">
        掷骰值: <strong style="color: var(--gold);">${node.roll_value}</strong> / ${node.total_weight}
      </div>
    `;

    const triggerDesc = document.createElement("div");
    triggerDesc.className = "tree-node-trigger";
    triggerDesc.innerHTML = `<span>⚡</span> <span>${escapeHtml(node.trigger_desc)}</span>`;

    const outcomeDesc = document.createElement("div");
    outcomeDesc.className = "tree-node-outcome";
    outcomeDesc.innerHTML = `<strong>结算效果:</strong> ${escapeHtml(node.outcome_desc)}`;

    // Candidates list
    const candidatesWrap = document.createElement("div");
    candidatesWrap.className = "candidates-wrap";
    const cHeading = document.createElement("div");
    cHeading.className = "candidates-heading";
    cHeading.textContent = `候选事件池评估 (共 ${node.candidates.length} 项)`;
    candidatesWrap.appendChild(cHeading);

    node.candidates.forEach((cand) => {
      const cRow = document.createElement("div");
      cRow.className = `candidate-row ${cand.selected ? "selected" : ""}`;
      const pct = (cand.probability_permille / 10).toFixed(1);

      cRow.innerHTML = `
        <div class="candidate-row-top">
          <div class="candidate-row-title">
            <span class="candidate-marker">${cand.selected ? "▶" : "·"}</span>
            <span class="candidate-name" title="${escapeHtml(cand.title)} (${escapeHtml(cand.event_id)})">
              ${escapeHtml(cand.title)}
              ${cand.is_dampener ? '<span class="candidate-dampener-badge">[阻尼]</span>' : ""}
              ${cand.is_lethal ? '<span class="candidate-lethal-badge">[致死]</span>' : ""}
            </span>
          </div>
          <span class="candidate-pct">${pct}%</span>
        </div>
        <div class="candidate-bar-cell">
          <span class="candidate-weights">${cand.base_weight} → ${cand.effective_weight}</span>
          <div class="candidate-bar-track">
            <div class="candidate-bar-fill" style="width: ${pct}%;"></div>
          </div>
        </div>
      `;
      candidatesWrap.appendChild(cRow);
    });

    nodeEl.append(top, triggerDesc, outcomeDesc, candidatesWrap);
    nodesContainer.appendChild(nodeEl);
  });

  card.append(header, nodesContainer);
  return card;
}

function closeDialogs() {
  document.querySelectorAll("dialog[open]").forEach((dialog) => dialog.close());
}

function closeEntryDialogs() {
  ["create-dialog", "join-dialog", "solo-dialog"].forEach((id) => {
    const dialog = document.getElementById(id);
    if (dialog?.open) dialog.close();
  });
}

function restorePlayerNames() {
  const name = sessionStorage.getItem("roulette_player_name") || "玩家";
  document.getElementById("solo-player-name").value = name;
  document.getElementById("create-player-name").value = name;
  document.getElementById("join-player-name").value = name;
}

function rememberPlayerName(name) {
  sessionStorage.setItem("roulette_player_name", name);
  document.getElementById("solo-player-name").value = name;
  document.getElementById("create-player-name").value = name;
  document.getElementById("join-player-name").value = name;
}

function setConnection(text, connected) {
  elements.connection_status.textContent = text;
  elements.connection_dot.classList.toggle("connected", connected);
}

function showToast(message) {
  elements.toast.textContent = message;
  elements.toast.classList.remove("hidden");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => elements.toast.classList.add("hidden"), 4200);
}

function value(id) { return document.getElementById(id).value.trim(); }
function safeJson(response) { return response.json().catch(() => null); }
function escapeHtml(value) { return String(value).replace(/[&<>'"]/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", "'": "&#39;", '"': "&quot;" })[character]); }
function phaseName(phase) { return ({ waiting: "等待中", playing: "对局中", finished: "已结束" })[phase] || phase; }
function directionName(direction) { return ({ up: "上", down: "下", left: "左", right: "右" })[direction]; }
function terrainName(terrain) { return ({ plain: "平地", empty: "平地", water: "水域", ice: "冰面", high_ground: "高地" })[terrain] || terrain; }
function objectName(obj) { return ({ wall: "墙体", crate: "木箱", mine: "地雷", shield: "护盾", medkit: "护盾" })[obj] || obj; }
function objectKindName(kind) { return ({ cover: "掩体障碍", trap: "陷阱道具", pickup: "战术物资" })[kind] || kind; }
function weatherName(weather) { return ({ clear: "晴朗", blizzard: "暴雪", heatwave: "热浪", dense_fog: "浓雾" })[weather] || weather; }

async function startClient() {
  await identityReady;
  elements.tab_identity.textContent = `TAB · ${tabId.slice(0, 4).toUpperCase()}`;
  elements.settings_tab_identity.textContent = `TAB · ${tabId.slice(0, 4).toUpperCase()}`;
  restorePlayerNames();
  loadTerrainRules();
  await loadInitialState();
  setInterval(poll, 2000);
  setInterval(heartbeat, 5000);
}

startClient();
