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

const terrainSymbols = { empty: "", wall: "▤", crate: "▦", water: "≈", ice: "❄", high_ground: "△", mine: "◆", medkit: "✚" };
const defaultTerrainRules = [
  {
    terrain: "wall",
    name: "墙体",
    symbol: "▤",
    layer: "above_ground",
    hardness: 2,
    transform_on_destroy: "empty",
    description: "坚硬砖石掩体，阻挡角色移动与普通弹道。普通子弹无法击穿；高温穿甲弹可直接击碎并贯穿继续射击。撞击时有极高概率致死。"
  },
  {
    terrain: "crate",
    name: "木箱",
    symbol: "▦",
    layer: "above_ground",
    hardness: 1,
    transform_on_destroy: "empty",
    description: "轻质木制掩体，阻挡角色移动。普通子弹可击碎破坏后停下；穿甲弹击碎后可继续贯穿前行。"
  },
  {
    terrain: "water",
    name: "水域",
    symbol: "≈",
    layer: "above_and_below",
    hardness: null,
    transform_on_destroy: null,
    description: "低洼深水区域，移动涉入时会遭受深水阻滞。若连续停留两回合将溺水淘汰。弹道直接掠过不受阻挡。暴风雪天气下相变为冰面。"
  },
  {
    terrain: "ice",
    name: "冰面",
    symbol: "❄",
    layer: "above_ground",
    hardness: null,
    transform_on_destroy: null,
    description: "极度光滑的低温冰面，踏入极易触发滑行冲刺或失控打滑。离开冰面蹬地施力有 30% 概率震碎薄冰使其相变为深水。热浪下融化为水。"
  },
  {
    terrain: "high_ground",
    name: "高地",
    symbol: "△",
    layer: "special",
    hardness: null,
    transform_on_destroy: null,
    description: "开阔的战术制高点，居高临下视野极佳。占据高地射击时有效射程额外增加 1 格。弹道可掠过高地。"
  },
  {
    terrain: "mine",
    name: "地雷",
    symbol: "◆",
    layer: "underground",
    hardness: null,
    transform_on_destroy: null,
    description: "埋伏于地表之下的烈性暗雷。子弹从上方空域飞过无法引爆或破坏；角色踏入进入生命周期检测，触发 85% 引爆或 10% 哑雷。"
  },
  {
    terrain: "medkit",
    name: "护盾",
    symbol: "✚",
    layer: "special",
    hardness: null,
    transform_on_destroy: null,
    description: "散落的单兵便携充能护盾补给。角色踏入时拾取激活护盾，可完全抵消一次致命伤害；拾取后变为空地。"
  },
  {
    terrain: "empty",
    name: "空地",
    symbol: "",
    layer: "above_ground",
    hardness: null,
    transform_on_destroy: null,
    description: "平整坚实的常规地面，无移动阻碍与特殊物理效果。"
  }
];
let cachedTerrainRules = null;

async function loadTerrainRules() {
  try {
    const res = await fetch("/api/rules/terrains");
    if (res.ok) {
      cachedTerrainRules = await res.json();
    }
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
  elements.founder_badge.textContent = settings.founder_authenticated ? "已认证" : "未认证";
  elements.founder_badge.classList.toggle("verified", settings.founder_authenticated);
  elements.lan_toggle.disabled = !settings.founder_authenticated;
  elements.lan_toggle.checked = settings.lan_exposed;
  elements.lan_description.textContent = settings.lan_exposed ? "局域网设备当前可以访问" : "当前仅本机可访问";
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
      elements.stalemate_badge.textContent = `🔥 僵局第 ${stalemateRounds} 轮 (致死率提升)`;
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
        ? "轮到你行动"
        : `${active?.name || "玩家"} 行动中`;
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
    node.className = `cell ${cell.terrain}`;
    node.title = `(${cell.position.x}, ${cell.position.y}) · ${terrainName(cell.terrain)}`;
    node.textContent = terrainSymbols[cell.terrain] || "";
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

function renderRecords(game) {
  const records = [...game.records].reverse();
  elements.records.replaceChildren(...records.map((record) => {
    const item = document.createElement("li");
    item.className = `record ${record.category}`;
    const badge = document.createElement("span");
    badge.className = "record-badge";
    badge.textContent = ({ action: "行动", event: "事件", turn: "回合", notification: "通知" })[record.category];
    const content = document.createElement("span");
    content.className = "record-content";
    content.textContent = recordText(game, record);
    const sequence = document.createElement("span");
    sequence.className = "record-sequence";
    sequence.textContent = `#${record.sequence}`;
    item.append(badge, content, sequence);
    return item;
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
    case "item_collected": return `${name(event.player_id)} 获得护盾`;
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
  if (game?.cells) {
    for (const cell of game.cells) {
      terrainCounts.set(cell.terrain, (terrainCounts.get(cell.terrain) || 0) + 1);
    }
  }

  const list = [...(cachedTerrainRules || defaultTerrainRules)].sort((a, b) => {
    const countA = terrainCounts.get(a.terrain) || 0;
    const countB = terrainCounts.get(b.terrain) || 0;
    if (countA > 0 && countB === 0) return -1;
    if (countA === 0 && countB > 0) return 1;
    if (countA !== countB) return countB - countA;
    return 0;
  });

  const presentSpecialCount = [...terrainCounts.entries()].filter(([t, count]) => t !== "empty" && count > 0).length;
  if (elements.terrain_present_count) {
    elements.terrain_present_count.textContent = game
      ? `当前地图含 ${presentSpecialCount} 种特殊地形 (共 ${list.length} 种)`
      : `全图鉴共 ${list.length} 种特殊地形规范`;
  }

  container.replaceChildren(...list.map((t) => {
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
    badge.textContent = isPresent ? `🟢 当前存在 (${count} 处)` : "⚪ 当前地图未包含";

    header.append(titleGroup, badge);

    const propsGrid = document.createElement("div");
    propsGrid.className = "terrain-props-grid";

    const posCol = document.createElement("div");
    posCol.className = "terrain-prop-item";
    posCol.innerHTML = `<span class="prop-label">位置层级</span><strong class="prop-val layer-${t.layer}">${layerName(t.layer)}</strong>`;

    const hardCol = document.createElement("div");
    hardCol.className = "terrain-prop-item";
    const hardnessText = t.hardness != null ? `${t.hardness}` : "none (无)";
    hardCol.innerHTML = `<span class="prop-label">物理硬度</span><strong class="prop-val">${hardnessText}</strong>`;

    const transCol = document.createElement("div");
    transCol.className = "terrain-prop-item";
    const transText = t.transform_on_destroy ? terrainName(t.transform_on_destroy) : "none (无)";
    transCol.innerHTML = `<span class="prop-label">被摧毁转换</span><strong class="prop-val">${transText}</strong>`;

    propsGrid.append(posCol, hardCol, transCol);

    const bulletRule = document.createElement("div");
    bulletRule.className = "terrain-bullet-rule";
    bulletRule.innerHTML = `<span class="bullet-rule-tag">弹药判定</span><span class="bullet-rule-desc">${bulletRuleText(t)}</span>`;

    const desc = document.createElement("p");
    desc.className = "terrain-card-desc";
    desc.textContent = t.description;

    card.append(header, propsGrid, bulletRule, desc);
    return card;
  }));
}

function bulletRuleText(t) {
  if (t.terrain === "wall") {
    return "【硬度 2 掩体】普通子弹无法破坏并被阻挡；<strong>穿甲重弹（穿甲能力 >= 2）可击碎为平地并贯穿继续向前！</strong>";
  }
  if (t.terrain === "crate") {
    return "【硬度 1 掩体】普通子弹击碎后停下；<strong>穿甲重弹击碎为平地后继续贯穿向前！</strong>";
  }
  if (t.terrain === "mine") {
    return "【地下暗藏】<strong>埋藏于地表之下（合法性保护）</strong>，子弹飞掠无法引爆或击碎；角色踏入触发踩雷判定。";
  }
  if (t.terrain === "water" || t.terrain === "ice") {
    return "【表面/凹地】无物理硬度，普通弹与穿甲弹直接在上方呼啸掠过，不阻挡弹道。";
  }
  if (t.terrain === "high_ground") {
    return "【战术高台】无物理硬度，子弹掠过不阻挡；占据高地射击时有效射程额外增加 1 格。";
  }
  if (t.terrain === "medkit") {
    return "【战术补给】特殊补给箱，子弹掠过不阻挡；角色进入拾取激活护盾。";
  }
  return "常规平地地面，无物理硬度，弹道正常穿越。";
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
function causeName(cause) { return ({ shot: "射击", elbow_duel: "肘击", mine: "地雷", drowned: "溺水", suicide: "主动结束", collision: "猛烈撞墙" })[cause] || cause; }
function terrainName(terrain) { return ({ empty: "空地", wall: "墙体", crate: "木箱", water: "水域", ice: "冰面", high_ground: "高地", mine: "地雷", medkit: "护盾" })[terrain] || terrain; }
function layerName(layer) { return ({ above_ground: "地上", underground: "地下", above_and_below: "地上且地下", special: "特殊" })[layer] || layer; }
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
