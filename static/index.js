"use strict";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------
const AXES = ["roll", "pitch", "yaw", "throttle"];
const AXIS_LABELS = { roll: "Roll · right stick ↔", pitch: "Pitch · right stick ↕", yaw: "Yaw · left stick ↔", throttle: "Throttle · left stick ↕" };
const DEFAULT_AXIS = { rate: 1, expo: 1, deadzone: 10 / 128, slew_us_per_s: 0 };
const AXIS_CONTROLS = [
    { key: "rate", label: "Rate", min: 0, max: 1, step: 0.01, fmt: (v) => `${Math.round(v * 100)}%` },
    { key: "expo", label: "Expo", min: 0, max: 1, step: 0.01, fmt: (v) => v.toFixed(2) },
    { key: "deadzone", label: "Deadzone", min: 0, max: 0.5, step: 0.01, fmt: (v) => `${Math.round(v * 100)}%` },
    { key: "slew_us_per_s", label: "Slew", min: 0, max: 5000, step: 50, fmt: (v) => (v === 0 ? "off" : `${v}`) },
];
const POWER_WINDOW_MS = 60_000;
/** Readings further apart than this are drawn as separate segments (no line across outages) */
const POWER_GAP_MS = 1_000;

const state = {
    telemetry: null, // latest ReceivedTelemetry.telemetry
    telemetryAt: 0, // performance.now() of the last telemetry event
    fcFrames: 0,
    fcAliveAt: 0,
    controller: null, // latest ControllerSnapshot
    controllerAt: 0,
    savedSettings: null,
    draftSettings: null,
    powerHistory: [], // { t: performance.now(), w }
    rebootPending: false,
    dirty: true,
};

const $ = (id) => document.getElementById(id);
const fmt = (v, digits = 1) => (v === null || v === undefined || Number.isNaN(v) ? "–" : Number(v).toFixed(digits));
const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));
const svgEl = (tag, attrs = {}) => {
    const el = document.createElementNS("http://www.w3.org/2000/svg", tag);
    Object.entries(attrs).forEach(([k, v]) => el.setAttribute(k, v));
    return el;
};

// ---------------------------------------------------------------------------
// Live data
// ---------------------------------------------------------------------------
function connectEvents() {
    const events = new EventSource("/telemetry/stream");

    events.addEventListener("telemetry", (e) => {
        const received = JSON.parse(e.data);
        if (!received) return;
        const now = performance.now();
        const t = received.telemetry;
        state.telemetry = t;
        state.telemetryAt = now;
        if (t.frames_received !== state.fcFrames) {
            state.fcFrames = t.frames_received;
            state.fcAliveAt = now;
        }
        if (t.battery) {
            state.powerHistory.push({ t: now, w: t.battery.power_w });
            trimPowerHistory(now);
        }
        state.dirty = true;
    });

    events.addEventListener("controller", (e) => {
        state.controller = JSON.parse(e.data);
        state.controllerAt = performance.now();
        state.dirty = true;
    });
    // EventSource reconnects on its own after errors
}

function setBadge(id, status, icon, text) {
    const badge = $(id);
    badge.dataset.status = status;
    const iconEl = badge.querySelector(".icon");
    if (iconEl) iconEl.textContent = icon;
    badge.querySelector(".text").textContent = text;
}

function renderBadges() {
    const now = performance.now();
    const age = state.telemetryAt ? now - state.telemetryAt : Infinity;
    if (age < 1000) setBadge("badge-link", "good", "✓", `Drone: ${Math.round(age)} ms`);
    else if (age < 5000) setBadge("badge-link", "warning", "!", `Drone: stale ${(age / 1000).toFixed(1)} s`);
    else setBadge("badge-link", "critical", "✕", "Drone: no data");

    const t = state.telemetry;
    if (!t || age >= 5000) setBadge("badge-fc", "neutral", "–", "FC: –");
    else if (now - state.fcAliveAt < 1000) setBadge("badge-fc", "good", "✓", `FC: OK (${t.crc_errors} CRC err)`);
    else setBadge("badge-fc", "critical", "✕", "FC: silent");

    const mode = t && t.flight_mode;
    if (!mode) setBadge("badge-armed", "neutral", "–", "Armed: –");
    else if (mode.armed) setBadge("badge-armed", "warning", "!", "ARMED");
    else setBadge("badge-armed", "good", "✓", "Disarmed");
    setBadge("badge-mode", "neutral", "", `Mode: ${mode ? mode.label || mode.name : "–"}`);
    renderRebootButton(t, age, now);

    const c = state.controller;
    const controllerAge = now - state.controllerAt;
    if (!c || controllerAge > 2000) setBadge("badge-gs", "neutral", "–", "Controller: –");
    else if (c.connected) setBadge("badge-gs", "good", "✓", `Controller: ${c.flight_mode} (${c.base_throttle})`);
    else setBadge("badge-gs", "critical", "✕", "Controller: not connected");
}

const BATTERY_STATE_STATUS = { OK: "good", WARNING: "warning", CRITICAL: "critical", NOT_PRESENT: "warning", INIT: "neutral" };

function renderPower(t) {
    const b = t && t.battery;
    const bs = t && t.battery_state;
    $("power-w").textContent = b ? fmt(b.power_w, 0) : "–";
    // MSP gives 0.01 V resolution, CRSF only 0.1 V
    $("voltage").textContent = bs ? fmt(bs.voltage_v, 2) : b ? fmt(b.voltage_v, 2) : "–";
    $("cell-voltage").textContent = bs && bs.cell_voltage_v ? fmt(bs.cell_voltage_v, 2) : "–";
    $("cell-count").textContent = bs && bs.cell_count ? `(${bs.cell_count}S)` : "";

    const stateEl = $("battery-state");
    if (bs) {
        const status = BATTERY_STATE_STATUS[bs.state] || "neutral";
        stateEl.className = "chip";
        stateEl.dataset.status = status;
        const icon = status === "good" ? "✓" : status === "neutral" ? "–" : status === "critical" ? "✕" : "!";
        stateEl.textContent = `${icon} Betaflight battery state: ${bs.state}`;
    } else {
        stateEl.className = "muted";
        delete stateEl.dataset.status;
        stateEl.textContent = "Battery state: – (MSP)";
    }
    $("current").textContent = b ? fmt(b.current_a, 1) : "–";
    $("used-mah").textContent = b ? b.used_mah : "–";
    $("battery-pct").textContent = b ? b.remaining_pct : "–";

    const meter = $("battery-meter");
    const pct = b ? b.remaining_pct : 0;
    meter.querySelector(".fill").style.width = `${pct}%`;
    const bfStatus = bs && BATTERY_STATE_STATUS[bs.state];
    meter.dataset.status = bfStatus === "critical" || pct < 20 ? "critical" : bfStatus === "warning" || pct < 40 ? "warning" : "good";

    renderSparkline();
}

/** Drops readings that have scrolled out of the window. Runs on every redraw, not only when a
 * new reading arrives, so the history empties out instead of sliding off-screen when data stops. */
function trimPowerHistory(now) {
    while (state.powerHistory.length && now - state.powerHistory[0].t > POWER_WINDOW_MS) {
        state.powerHistory.shift();
    }
}

function renderSparkline() {
    const svg = $("power-spark");
    svg.replaceChildren();
    const now = performance.now();
    trimPowerHistory(now);

    const pts = state.powerHistory;
    const newest = pts[pts.length - 1];
    const staleS = newest ? (now - newest.t) / 1000 : null;
    $("power-spark-label").textContent = !newest
        ? "Power, last 60 s · no data"
        : staleS > 2 ? `Power, last 60 s · no data for ${staleS.toFixed(0)} s` : "Power, last 60 s";
    if (!pts.length) return;

    const maxW = Math.max(10, ...pts.map((p) => p.w)) * 1.1;
    const x = (p) => 300 - ((now - p.t) / POWER_WINDOW_MS) * 300;
    const y = (p) => 58 - (p.w / maxW) * 56;
    svg.append(svgEl("line", { x1: 0, x2: 300, y1: 58, y2: 58, class: "baseline", stroke: "var(--grid)", "stroke-width": 1 }));
    svg.append(svgEl("path", {
        // start a new segment after a gap so an outage isn't drawn as a straight line
        d: pts.map((p, i) => `${i && p.t - pts[i - 1].t <= POWER_GAP_MS ? "L" : "M"}${x(p).toFixed(1)},${y(p).toFixed(1)}`).join(""),
        fill: "none", stroke: "var(--accent)", "stroke-width": 2, "vector-effect": "non-scaling-stroke",
        "stroke-linejoin": "round", "stroke-linecap": "round",
    }));
}

function setupSparklineHover() {
    const svg = $("power-spark");
    const tip = $("power-tip");
    svg.addEventListener("pointermove", (e) => {
        const pts = state.powerHistory;
        if (!pts.length) return;
        const rect = svg.getBoundingClientRect();
        const frac = clamp((e.clientX - rect.left) / rect.width, 0, 1);
        const target = performance.now() - (1 - frac) * POWER_WINDOW_MS;
        const nearest = pts.reduce((a, b) => (Math.abs(b.t - target) < Math.abs(a.t - target) ? b : a));
        const ago = (performance.now() - nearest.t) / 1000;
        tip.textContent = `${fmt(nearest.w, 0)} W · ${ago < 1 ? "now" : `${ago.toFixed(0)} s ago`}`;
        tip.style.left = `${frac * rect.width}px`;
        tip.style.top = `${svg.offsetTop}px`;
        tip.hidden = false;
    });
    svg.addEventListener("pointerleave", () => (tip.hidden = true));
}

function renderAttitude(t) {
    const a = t && t.attitude;
    $("pitch").textContent = a ? fmt(a.pitch_deg) : "–";
    $("roll").textContent = a ? fmt(a.roll_deg) : "–";
    $("yaw").textContent = a ? fmt(a.yaw_deg, 0) : "–";
    drawHorizon(a ? a.pitch_deg : 0, a ? a.roll_deg : 0, !!a);
}

function drawHorizon(pitch, roll, hasData) {
    const canvas = $("horizon");
    const ctx = canvas.getContext("2d");
    const css = getComputedStyle(document.documentElement);
    const color = (name) => css.getPropertyValue(name).trim();
    const size = canvas.width;
    const r = size / 2;
    const pxPerDeg = r / 45;

    ctx.clearRect(0, 0, size, size);
    ctx.save();
    ctx.beginPath();
    ctx.arc(r, r, r - 2, 0, Math.PI * 2);
    ctx.clip();

    ctx.translate(r, r);
    ctx.rotate((-roll * Math.PI) / 180);
    ctx.translate(0, pitch * pxPerDeg);

    ctx.fillStyle = hasData ? color("--sky") : color("--surface-2");
    ctx.fillRect(-size, -size * 2, size * 2, size * 2);
    ctx.fillStyle = hasData ? color("--ground") : color("--border");
    ctx.fillRect(-size, 0, size * 2, size * 2);

    // horizon + pitch ladder
    ctx.strokeStyle = "#ffffff";
    ctx.fillStyle = "#ffffff";
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(-size, 0);
    ctx.lineTo(size, 0);
    ctx.stroke();
    ctx.lineWidth = 1;
    ctx.font = "10px system-ui";
    ctx.textBaseline = "middle";
    for (let deg = -60; deg <= 60; deg += 10) {
        if (deg === 0) continue;
        const y = -deg * pxPerDeg;
        const half = deg % 20 === 0 ? 30 : 16;
        ctx.beginPath();
        ctx.moveTo(-half, y);
        ctx.lineTo(half, y);
        ctx.stroke();
        if (deg % 20 === 0) ctx.fillText(String(Math.abs(deg)), half + 4, y);
    }
    ctx.restore();

    // fixed aircraft symbol
    ctx.strokeStyle = "#f5c400";
    ctx.lineWidth = 3;
    ctx.lineCap = "round";
    ctx.beginPath();
    ctx.moveTo(r - 50, r);
    ctx.lineTo(r - 14, r);
    ctx.lineTo(r - 6, r + 8);
    ctx.moveTo(r + 50, r);
    ctx.lineTo(r + 14, r);
    ctx.lineTo(r + 6, r + 8);
    ctx.stroke();
    ctx.beginPath();
    ctx.arc(r, r, 2.5, 0, Math.PI * 2);
    ctx.fillStyle = "#f5c400";
    ctx.fill();

    // bezel
    ctx.strokeStyle = color("--border");
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.arc(r, r, r - 2, 0, Math.PI * 2);
    ctx.stroke();
}

function renderBars(containerId, rows, emptyText) {
    const container = $(containerId);
    if (!rows.length) {
        container.innerHTML = `<p class="muted">${emptyText}</p>`;
        return;
    }
    if (container.children.length !== rows.length || !container.querySelector(".bar-row")) {
        container.replaceChildren(...rows.map(() => {
            const row = document.createElement("div");
            row.className = "bar-row";
            row.innerHTML = `<span class="name"></span><div class="track"><div class="fill"></div></div><span class="val"></span>`;
            return row;
        }));
    }
    rows.forEach((r, i) => {
        const row = container.children[i];
        row.querySelector(".name").textContent = r.name;
        row.querySelector(".fill").style.width = `${clamp(r.pct, 0, 100)}%`;
        row.querySelector(".val").textContent = r.value;
    });
}

function renderFlightMode(t) {
    const mode = t && t.flight_mode;
    $("mode-label").textContent = mode ? `${mode.label || mode.name}${mode.armed ? "" : " · disarmed"}` : "–";
    $("mode-code").textContent = mode ? mode.name : "–";

    const s = t && t.fc_status;
    const list = $("active-modes");
    if (!s) {
        list.innerHTML = `<li class="muted">No mode data (MSP)</li>`;
    } else if (!s.active_modes.length) {
        list.innerHTML = `<li class="muted">None active (or mode names not loaded yet)</li>`;
    } else {
        list.replaceChildren(...s.active_modes.map((name) => {
            const li = document.createElement("li");
            li.className = "chip";
            li.textContent = name;
            return li;
        }));
    }

    const c = state.controller;
    $("gs-mode").textContent = c ? `${c.flight_mode} · base throttle ${c.base_throttle} µs` : "–";
}

/** Same rules as `Telemetry::check_reboot_allowed` on the drone, which has the final say */
function rebootBlockedReason(t, telemetryAge, now) {
    if (!t || telemetryAge > 1000) return "no telemetry from the drone";
    if (now - state.fcAliveAt > 1000) return "no recent data from the FC";
    if (!t.flight_mode) return "FC arm state unknown";
    if (t.flight_mode.armed) return "FC is ARMED";
    if (t.rc_sent && t.rc_sent.aux1 >= 1500) return "arm switch (AUX1) is not in the disarm position";
    return null;
}

function renderRebootButton(t, telemetryAge, now) {
    const button = $("reboot-fc");
    const reason = rebootBlockedReason(t, telemetryAge, now);
    button.disabled = !!reason || state.rebootPending;
    button.title = reason ? `Unavailable: ${reason}` : "Reboot the flight controller";

    const r = t && t.fc_reboot;
    const status = $("reboot-status");
    if (state.rebootPending) status.textContent = "Requesting…";
    else if (r && Date.now() - r.at_ms < 60_000) status.textContent = r.message;
    else status.textContent = reason ? `Unavailable: ${reason}` : "";
}

async function rebootFc() {
    if (!confirm("Reboot the flight controller?\n\nThe drone will only do it if the FC reports disarmed. Telemetry drops out for a few seconds.")) return;
    state.rebootPending = true;
    renderBadges();
    try {
        const res = await fetch("/reboot-fc", { method: "POST" });
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
    } catch (e) {
        $("reboot-status").textContent = `Request failed: ${e.message}`;
    } finally {
        // the drone reports the outcome in telemetry (fc_reboot)
        setTimeout(() => { state.rebootPending = false; renderBadges(); }, 500);
    }
}

function renderMotorsAndRc(t) {
    const motors = (t && t.motors) || [];
    renderBars("motors", motors.map((m, i) => ({
        name: `M${i + 1}`,
        pct: m ? (m - 1000) / 10 : 0,
        value: m ? `${Math.round((m - 1000) / 10)}%` : "off",
    })), "No motor data (MSP)");

    const rc = t && t.rc_sent;
    const channels = ["roll", "pitch", "yaw", "thr", "aux1", "aux2", "aux3", "aux4"];
    renderBars("rc-sent", rc ? channels.map((ch) => ({
        name: ch,
        pct: (rc[ch] - 1000) / 10,
        value: rc[ch],
    })) : [], "–");
}

function renderPosition(t) {
    const g = t && t.gps;
    const alt = t && t.altitude;
    $("baro-alt").textContent = alt ? fmt(alt.baro_altitude_m) : "–";
    $("vspeed").textContent = alt ? fmt(alt.vertical_speed_ms) : "–";
    $("gspeed").textContent = g ? fmt(g.ground_speed_ms) : "–";
    $("heading").textContent = g ? fmt(g.heading_deg, 0) : "–";
    $("gps-alt").textContent = g ? g.altitude_m : "–";
    $("sats").textContent = g ? g.satellites : "–";
    $("latlon").textContent = g ? `${g.latitude.toFixed(6)}, ${g.longitude.toFixed(6)}` : "–";
}

function renderHealth(t) {
    const s = t && t.fc_status;
    $("cpu").textContent = s ? s.cpu_load_pct : "–";
    $("cycle").textContent = s ? s.cycle_time_us : "–";
    $("cpu-temp").textContent = s && s.cpu_temp_c ? s.cpu_temp_c : "–";
    $("crc").textContent = t ? t.crc_errors : "–";
    $("sensors").textContent = s ? s.sensors.join(", ") || "none" : "–";

    const meter = $("cpu-meter");
    const load = s ? s.cpu_load_pct : 0;
    meter.querySelector(".fill").style.width = `${clamp(load, 0, 100)}%`;
    meter.dataset.status = load > 85 ? "critical" : load > 65 ? "warning" : "good";

    const list = $("arming");
    if (!s) {
        list.innerHTML = `<li class="muted">No status (MSP)</li>`;
    } else if (!s.arming_disable_flags.length && !s.reboot_required) {
        list.innerHTML = `<li data-status="good">✓ Ready to arm</li>`;
    } else {
        const items = s.arming_disable_flags.map((f) => `✕ ${f}`);
        if (s.reboot_required) items.push("✕ REBOOT_REQUIRED");
        list.innerHTML = items.map((txt) => `<li data-status="critical">${txt}</li>`).join("");
    }

    const imu = t && t.imu;
    ["x", "y", "z"].forEach((axis, i) => {
        $(`g${axis}`).textContent = imu ? imu.gyro_dps[i] : "–";
        $(`a${axis}`).textContent = imu ? imu.acc_raw[i] : "–";
    });
}

// ---------------------------------------------------------------------------
// Sensitivity tuning
// ---------------------------------------------------------------------------
/** Same curve as `AxisSettings::apply` in src/dualsense_controller/settings.rs */
function applyCurve(axis, stick) {
    const s = clamp(stick, -1, 1);
    const magnitude = Math.abs(s);
    if (magnitude <= axis.deadzone) return 0;
    const x = (magnitude - axis.deadzone) / (1 - axis.deadzone);
    return Math.sign(s) * ((1 - axis.expo) * x + axis.expo * x ** 3) * axis.rate;
}

function baseThrottle() {
    return state.controller ? state.controller.base_throttle : 1000;
}

/** Stick in [-1, 1] -> µs, matching `DualsenseController::to_rc_controls` (without slew) */
function stickToUs(axisName, axisSettings, stick) {
    const v = applyCurve(axisSettings, stick);
    if (axisName !== "throttle") return 1500 + v * 500;
    const base = baseThrottle();
    return clamp(base + v * (2000 - base), 885, 2000);
}

const CURVE = { w: 260, h: 150, padL: 34, padR: 8, padT: 8, padB: 18 };
const yRange = (axisName) => (axisName === "throttle" ? [885, 2000] : [1000, 2000]);
const curveX = (stick) => CURVE.padL + ((stick + 1) / 2) * (CURVE.w - CURVE.padL - CURVE.padR);
const curveY = (axisName, us) => {
    const [lo, hi] = yRange(axisName);
    return CURVE.h - CURVE.padB - ((us - lo) / (hi - lo)) * (CURVE.h - CURVE.padT - CURVE.padB);
};

function buildAxisCards() {
    const container = $("axes");
    container.replaceChildren(...AXES.map((axisName) => {
        const card = document.createElement("div");
        card.className = "axis";
        card.innerHTML = `<h3><span>${AXIS_LABELS[axisName]}</span><span class="live" id="live-${axisName}">–</span></h3>`;

        const svg = svgEl("svg", { viewBox: `0 0 ${CURVE.w} ${CURVE.h}`, class: "curve", id: `curve-${axisName}`, role: "img", "aria-label": `${axisName} response curve` });
        card.append(svg);
        const tip = document.createElement("div");
        tip.className = "tooltip";
        tip.hidden = true;
        card.append(tip);
        setupCurveHover(axisName, svg, tip);

        AXIS_CONTROLS.forEach((c) => {
            const row = document.createElement("div");
            row.className = "control";
            const id = `${axisName}-${c.key}`;
            row.innerHTML = `<label for="${id}">${c.label}</label><input type="range" id="${id}" min="${c.min}" max="${c.max}" step="${c.step}"><output id="${id}-out"></output>`;
            row.querySelector("input").addEventListener("input", (e) => {
                state.draftSettings[axisName][c.key] = parseFloat(e.target.value);
                renderSettings();
            });
            card.append(row);
        });
        return card;
    }));
}

function setupCurveHover(axisName, svg, tip) {
    svg.addEventListener("pointermove", (e) => {
        if (!state.draftSettings) return;
        const rect = svg.getBoundingClientRect();
        const vbX = ((e.clientX - rect.left) / rect.width) * CURVE.w;
        const stick = clamp(((vbX - CURVE.padL) / (CURVE.w - CURVE.padL - CURVE.padR)) * 2 - 1, -1, 1);
        const us = stickToUs(axisName, state.draftSettings[axisName], stick);
        tip.textContent = `stick ${Math.round(stick * 100)}% → ${Math.round(us)} µs`;
        tip.style.left = `${e.clientX - svg.parentElement.getBoundingClientRect().left}px`;
        tip.style.top = `${svg.offsetTop}px`;
        tip.hidden = false;
        let line = svg.querySelector(".hover-line");
        if (!line) {
            line = svgEl("line", { class: "hover-line", y1: CURVE.padT, y2: CURVE.h - CURVE.padB });
            svg.append(line);
        }
        line.setAttribute("x1", curveX(stick));
        line.setAttribute("x2", curveX(stick));
    });
    svg.addEventListener("pointerleave", () => {
        tip.hidden = true;
        svg.querySelector(".hover-line")?.remove();
    });
}

function drawCurve(axisName) {
    const svg = $(`curve-${axisName}`);
    const axis = state.draftSettings[axisName];
    const hover = svg.querySelector(".hover-line");
    svg.replaceChildren();
    const [lo, hi] = yRange(axisName);

    // grid + ticks
    [lo, 1500, hi].forEach((us) => {
        svg.append(svgEl("line", { class: "gridline", x1: CURVE.padL, x2: CURVE.w - CURVE.padR, y1: curveY(axisName, us), y2: curveY(axisName, us) }));
        const label = svgEl("text", { x: CURVE.padL - 4, y: curveY(axisName, us) + 3, "text-anchor": "end" });
        label.textContent = us;
        svg.append(label);
    });
    [-1, 0, 1].forEach((s) => {
        const anchor = s < 0 ? "start" : s > 0 ? "end" : "middle";
        const label = svgEl("text", { x: curveX(s), y: CURVE.h - 4, "text-anchor": anchor });
        label.textContent = `${s * 100}%`;
        svg.append(label);
    });
    svg.append(svgEl("line", { class: "axisline", x1: curveX(0), x2: curveX(0), y1: CURVE.padT, y2: CURVE.h - CURVE.padB }));

    // linear full-rate reference, then the tuned curve
    const path = (fn) => Array.from({ length: 101 }, (_, i) => {
        const stick = -1 + (i / 100) * 2;
        return `${i ? "L" : "M"}${curveX(stick).toFixed(1)},${curveY(axisName, fn(stick)).toFixed(1)}`;
    }).join("");
    svg.append(svgEl("path", { class: "reference", d: path((s) => stickToUs(axisName, { rate: 1, expo: 0, deadzone: 0 }, s)) }));
    svg.append(svgEl("path", { class: "line", d: path((s) => stickToUs(axisName, axis, s)) }));

    // live stick position
    const c = state.controller;
    if (c) {
        const stick = c.sticks[axisName];
        svg.append(svgEl("circle", { class: "dot", r: 5, cx: curveX(stick), cy: curveY(axisName, stickToUs(axisName, axis, stick)) }));
    }
    if (hover) svg.append(hover);
}

function renderLiveAxes() {
    const c = state.controller;
    AXES.forEach((axisName) => {
        const el = $(`live-${axisName}`);
        if (!c || !c.rc) {
            el.textContent = "–";
            return;
        }
        const sent = c.rc[axisName === "throttle" ? "thr" : axisName];
        el.textContent = `${Math.round(c.sticks[axisName] * 100)}% → ${sent} µs`;
    });
}

function renderSettings() {
    if (!state.draftSettings) return;
    AXES.forEach((axisName) => {
        AXIS_CONTROLS.forEach((c) => {
            const id = `${axisName}-${c.key}`;
            const value = state.draftSettings[axisName][c.key];
            const input = $(id);
            if (document.activeElement !== input) input.value = value;
            $(`${id}-out`).textContent = c.fmt(value);
        });
        drawCurve(axisName);
    });
    const dirty = JSON.stringify(state.draftSettings) !== JSON.stringify(state.savedSettings);
    $("settings-save").disabled = !dirty;
    $("settings-revert").disabled = !dirty;
    if (dirty) $("settings-status").textContent = "Unsaved changes";
}

async function loadSettings() {
    try {
        const res = await fetch("/controller-settings");
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        state.savedSettings = await res.json();
        state.draftSettings = structuredClone(state.savedSettings);
        $("settings-status").textContent = "";
        renderSettings();
    } catch (e) {
        $("axes").innerHTML = `<p class="muted">Controller settings unavailable (${e.message}). Is the control server built with the dualsense feature?</p>`;
        document.querySelectorAll(".sensitivity .actions button").forEach((b) => (b.disabled = true));
    }
}

async function saveSettings() {
    $("settings-status").textContent = "Saving…";
    try {
        const res = await fetch("/controller-settings", {
            method: "PUT",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(state.draftSettings),
        });
        const body = await res.json();
        if (!res.ok) throw new Error(body.error || `HTTP ${res.status}`);
        state.savedSettings = body;
        state.draftSettings = structuredClone(body);
        renderSettings();
        $("settings-status").textContent = "Saved";
    } catch (e) {
        $("settings-status").textContent = `Save failed: ${e.message}`;
    }
}

function setupSettingsButtons() {
    $("settings-save").addEventListener("click", saveSettings);
    $("settings-revert").addEventListener("click", () => {
        state.draftSettings = structuredClone(state.savedSettings);
        $("settings-status").textContent = "";
        renderSettings();
    });
    $("settings-defaults").addEventListener("click", () => {
        AXES.forEach((axisName) => (state.draftSettings[axisName] = { ...DEFAULT_AXIS }));
        renderSettings();
    });
}

// ---------------------------------------------------------------------------
// Manual RC sliders (sends straight to /set-rc)
// ---------------------------------------------------------------------------
const defaultValues = { roll: 1500, pitch: 1500, yaw: 1500, thr: 885, aux1: 1000, aux2: 1000, aux3: 1000, aux4: 1000 };
const sliderAmount = 100;
const keyboardMaps = {
    arrowup: { name: "pitch", type: "incremental", increaseBy: sliderAmount },
    arrowdown: { name: "pitch", type: "incremental", increaseBy: -sliderAmount },
    arrowleft: { name: "roll", type: "incremental", increaseBy: -sliderAmount },
    arrowright: { name: "roll", type: "incremental", increaseBy: sliderAmount },
    w: { name: "thr", type: "incremental", increaseBy: sliderAmount },
    s: { name: "thr", type: "incremental", increaseBy: -sliderAmount },
    a: { name: "yaw", type: "incremental", increaseBy: -sliderAmount },
    d: { name: "yaw", type: "incremental", increaseBy: sliderAmount },
    1: { name: "aux1", type: "modes", modes: [1000, 1700, 1950] },
    2: { name: "aux2", type: "modes", modes: [1000, 1400, 1900] },
    3: { name: "aux3", type: "modes", modes: [1000, 1700] },
    4: { name: "aux4", type: "modes", modes: [1000, 1700] },
};

function setupManualRc() {
    const slidersDiv = $("sliders");
    const data = { ...defaultValues };
    const sendData = () => fetch("/set-rc", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(data) });

    Object.entries(defaultValues).forEach(([name, defaultValue]) => {
        const container = document.createElement("div");
        container.className = "slider-container";
        container.innerHTML = `<label for="${name}">${name}</label><input type="range" id="${name}" min="885" max="2000" value="${defaultValue}"><div class="value">${defaultValue}</div>`;
        const input = container.querySelector("input");
        input.addEventListener("input", () => {
            container.querySelector(".value").textContent = input.value;
            data[name] = parseInt(input.value, 10);
            sendData();
        });
        slidersDiv.append(container);
    });

    window.addEventListener("keydown", (event) => {
        // Only while the manual panel is open, and never while an input has focus
        // (range sliders use the arrow keys too)
        if (!document.querySelector("details.manual").open) return;
        if (event.target instanceof HTMLInputElement) return;
        const keyboardMap = keyboardMaps[event.key.toLowerCase()];
        if (!keyboardMap) return;
        const input = $(keyboardMap.name);
        if (!input) return;
        event.preventDefault();

        if (keyboardMap.type === "incremental") {
            const step = keyboardMap.increaseBy / (event.shiftKey ? Math.abs(keyboardMap.increaseBy) : 1);
            input.value = clamp(parseInt(input.value, 10) + step, 885, 2000);
        } else {
            const { modes } = keyboardMap;
            const currentIndex = modes.indexOf(parseInt(input.value, 10));
            input.value = currentIndex === -1 ? modes[0] : modes[(currentIndex + 1) % modes.length];
        }
        input.dispatchEvent(new Event("input"));
    });
}

// ---------------------------------------------------------------------------
// Camera: H.264 from the drone, decoded in the browser with WebCodecs
// ---------------------------------------------------------------------------
const CAMERA_RECONNECT_MS = 2000;
/** If the decoder falls this far behind, skip ahead to the next keyframe */
const CAMERA_MAX_DECODE_QUEUE = 8;
const CAMERA_FRAME_US = 40_000;
const CAMERA_ROTATE_KEY = "camera-rotated";

const camera = {
    decoder: null,
    codec: null,
    needKey: true,
    pending: [], // non-picture NAL units (SPS/PPS/SEI) waiting for the next frame
    timestamp: 0,
    framesThisSecond: 0,
    lastFrameAt: 0,
    size: "",
};

function concatBytes(parts) {
    const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
    let offset = 0;
    for (const p of parts) {
        out.set(p, offset);
        offset += p.length;
    }
    return out;
}

/** Offset of the NAL header byte (after a 00 00 01 or 00 00 00 01 start code) */
const nalHeaderOffset = (nal) => (nal[2] === 1 ? 3 : 4);
const nalType = (nal) => nal[nalHeaderOffset(nal)] & 0x1f;

/** e.g. avc1.640028, from the SPS profile / constraint / level bytes */
function codecFromSps(sps) {
    const h = nalHeaderOffset(sps);
    const hex = (b) => b.toString(16).padStart(2, "0");
    return `avc1.${hex(sps[h + 1])}${hex(sps[h + 2])}${hex(sps[h + 3])}`;
}

function setCameraStatus(text, overlayText) {
    $("camera-status").textContent = text;
    const overlay = $("camera-overlay");
    overlay.hidden = !overlayText;
    if (overlayText) overlay.textContent = overlayText;
}

function createCameraDecoder(codec) {
    if (camera.decoder && camera.decoder.state !== "closed") camera.decoder.close();
    const canvas = $("camera");
    const ctx = canvas.getContext("2d");
    camera.decoder = new VideoDecoder({
        output: (frame) => {
            if (canvas.width !== frame.displayWidth || canvas.height !== frame.displayHeight) {
                canvas.width = frame.displayWidth;
                canvas.height = frame.displayHeight;
                canvas.parentElement.style.aspectRatio = `${frame.displayWidth} / ${frame.displayHeight}`;
            }
            ctx.drawImage(frame, 0, 0);
            frame.close();
            camera.framesThisSecond += 1;
            camera.lastFrameAt = performance.now();
            camera.size = `${canvas.width}×${canvas.height}`;
        },
        error: (e) => {
            console.warn("Camera decoder error", e);
            camera.codec = null; // recreated at the next SPS
            camera.needKey = true;
        },
    });
    camera.decoder.configure({ codec, optimizeForLatency: true });
    camera.codec = codec;
    camera.needKey = true;
}

/** Assumes one slice per frame, which is what the Pi's hardware encoder produces */
function handleCameraNal(nal) {
    const type = nalType(nal);
    if (type === 7) {
        const codec = codecFromSps(nal);
        if (codec !== camera.codec || !camera.decoder || camera.decoder.state === "closed") {
            createCameraDecoder(codec);
        }
    }
    if (type !== 1 && type !== 5) {
        camera.pending.push(nal);
        if (camera.pending.length > 32) camera.pending = [];
        return;
    }

    const isKey = type === 5;
    const parts = camera.pending;
    camera.pending = [];
    if (!camera.decoder || camera.decoder.state !== "configured") return;
    if (camera.decoder.decodeQueueSize > CAMERA_MAX_DECODE_QUEUE) camera.needKey = true;
    if (camera.needKey && !isKey) return;
    camera.needKey = false;

    camera.decoder.decode(new EncodedVideoChunk({
        type: isKey ? "key" : "delta",
        timestamp: camera.timestamp,
        data: concatBytes([...parts, nal]),
    }));
    camera.timestamp += CAMERA_FRAME_US;
}

/** Reads `[u32 BE length][NAL]` records from /camera/stream forever, reconnecting on errors */
async function runCamera() {
    if (!("VideoDecoder" in window)) {
        setCameraStatus("Unsupported", "This browser can't decode the H.264 camera stream. Use Chrome, Edge or Safari 16.4+.");
        return;
    }
    for (;;) {
        try {
            const res = await fetch("/camera/stream", { cache: "no-store" });
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            const reader = res.body.getReader();
            let buf = new Uint8Array(0);
            camera.needKey = true;
            camera.pending = [];
            for (;;) {
                const { value, done } = await reader.read();
                if (done) break;
                buf = buf.length ? concatBytes([buf, value]) : value;
                let offset = 0;
                while (buf.length - offset >= 4) {
                    const len = new DataView(buf.buffer, buf.byteOffset + offset, 4).getUint32(0);
                    if (buf.length - offset - 4 < len) break;
                    handleCameraNal(buf.subarray(offset + 4, offset + 4 + len));
                    offset += 4 + len;
                }
                buf = buf.slice(offset);
            }
        } catch (e) {
            console.warn("Camera stream", e);
        }
        await new Promise((resolve) => setTimeout(resolve, CAMERA_RECONNECT_MS));
    }
}

async function renderCameraStatus() {
    const fps = camera.framesThisSecond;
    camera.framesThisSecond = 0;
    if (!("VideoDecoder" in window)) return;
    if (performance.now() - camera.lastFrameAt < 1500) {
        setCameraStatus(`Live · ${fps} fps · ${camera.size}`, null);
        return;
    }
    try {
        const status = await fetch("/camera/status", { cache: "no-store" }).then((r) => r.json());
        if (status.connected) setCameraStatus("Waiting for keyframe…", "Connected, waiting for the next keyframe…");
        else setCameraStatus("Offline", `Can't reach the drone's camera at ${status.addr}. Is live_camera.service running on the Pi?`);
    } catch {
        setCameraStatus("Offline", "Control server unreachable");
    }
}

function setupCameraRotate() {
    const canvas = $("camera");
    try {
        canvas.classList.toggle("rotated", localStorage.getItem(CAMERA_ROTATE_KEY) === "1");
    } catch { /* storage unavailable */ }
    $("camera-rotate").addEventListener("click", () => {
        const rotated = canvas.classList.toggle("rotated");
        try {
            localStorage.setItem(CAMERA_ROTATE_KEY, rotated ? "1" : "0");
        } catch { /* storage unavailable */ }
    });
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------
function frame() {
    if (state.dirty) {
        state.dirty = false;
        const t = state.telemetry;
        renderPower(t);
        renderAttitude(t);
        renderFlightMode(t);
        renderMotorsAndRc(t);
        renderPosition(t);
        renderHealth(t);
        renderLiveAxes();
        if (state.draftSettings) AXES.forEach(drawCurve);
    }
    requestAnimationFrame(frame);
}

window.addEventListener("load", () => {
    buildAxisCards();
    setupSettingsButtons();
    $("reboot-fc").addEventListener("click", rebootFc);
    setupCameraRotate();
    runCamera();
    renderCameraStatus();
    setInterval(renderCameraStatus, 1000);
    setupSparklineHover();
    setupManualRc();
    loadSettings();
    connectEvents();
    renderBadges();
    setInterval(() => {
        renderBadges();
        // keep the time axis moving (and old points expiring) even when no events arrive
        renderSparkline();
    }, 250);
    requestAnimationFrame(frame);
});
