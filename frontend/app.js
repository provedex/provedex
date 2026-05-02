const API = '/api';
const eventsEl = document.getElementById('events');
const eventCountEl = document.getElementById('event-count');
const conversationEl = document.getElementById('conversation');
const verifyResultEl = document.getElementById('verify-result');
const sessionInfoEl = document.getElementById('session-info');
const micBtn = document.getElementById('mic-btn');
const micStatus = document.getElementById('mic-status');

let eventCount = 0;
const eventRowsBySeq = new Map();
let mediaRecorder = null;
let chunks = [];

async function init() {
  try {
    const r = await fetch(`${API}/healthz`);
    const j = await r.json();
    sessionInfoEl.textContent = `session ${j.session_id.slice(0, 8)}\npubkey ${j.pubkey.slice(0, 16)}...`;
  } catch (e) {
    sessionInfoEl.textContent = 'server unreachable';
  }
  subscribeEvents();
  wireButtons();
  wireMic();
}

function subscribeEvents() {
  const es = new EventSource(`${API}/events`);
  es.addEventListener('signed', (msg) => {
    try {
      const evt = JSON.parse(msg.data);
      addEventRow(evt);
    } catch (e) {
      console.error('parse error', e);
    }
  });
  es.onerror = () => {
    setTimeout(subscribeEvents, 2000);
    es.close();
  };
}

function addEventRow(evt) {
  const row = document.createElement('div');
  row.className = 'event-row fresh';
  row.dataset.seq = evt.seq;
  const type = evt.event && evt.event.type ? evt.event.type : 'Unknown';
  const shortHash = evt.self_hash ? evt.self_hash.slice(0, 12) : '';
  row.innerHTML = `<span class="text-slate-500">seq=${String(evt.seq).padStart(3, '0')}</span> ` +
    `<span class="event-type">${type}</span> ` +
    `<span class="event-hash">${shortHash}</span>`;
  eventsEl.appendChild(row);
  eventsEl.scrollTop = eventsEl.scrollHeight;
  eventRowsBySeq.set(evt.seq, row);
  eventCount += 1;
  eventCountEl.textContent = `${eventCount} events`;

  if (evt.event && evt.event.type === 'UtteranceCaptured' && evt.event.payload) {
    addBubble('user', evt.event.payload.transcript || '');
  }
  if (evt.event && evt.event.type === 'UtteranceSpoken' && evt.event.payload) {
    addBubble('agent', evt.event.payload.text || '');
  }
}

function addBubble(role, text) {
  if (!text) return;
  const wrap = document.createElement('div');
  wrap.className = 'flex flex-col';
  const bubble = document.createElement('span');
  bubble.className = `bubble ${role === 'user' ? 'bubble-user' : 'bubble-agent'}`;
  bubble.textContent = text;
  wrap.appendChild(bubble);
  conversationEl.appendChild(wrap);
  conversationEl.scrollTop = conversationEl.scrollHeight;
}

function wireButtons() {
  document.getElementById('verify-btn').addEventListener('click', async () => {
    verifyResultEl.textContent = 'verifying...';
    verifyResultEl.className = 'ml-auto text-sm font-mono text-slate-400';
    try {
      const r = await fetch(`${API}/verify`, { method: 'POST' });
      const j = await r.json();
      renderVerifyResult(j);
    } catch (e) {
      verifyResultEl.textContent = `error: ${e}`;
      verifyResultEl.className = 'ml-auto text-sm font-mono text-red-400';
    }
  });
  document.getElementById('tamper-btn').addEventListener('click', async () => {
    if (!confirm('Tamper test will mutate one event in the local ledger so the chain visibly breaks. Continue?')) return;
    try {
      const r = await fetch(`${API}/tamper-test`, { method: 'POST' });
      if (!r.ok) throw new Error(await r.text());
      const j = await r.json();
      verifyResultEl.textContent = `tampered seq=${j.tampered_seq}; click Verify`;
      verifyResultEl.className = 'ml-auto text-sm font-mono text-amber-400';
    } catch (e) {
      verifyResultEl.textContent = `tamper failed: ${e.message || e}`;
      verifyResultEl.className = 'ml-auto text-sm font-mono text-red-400';
    }
  });
  document.getElementById('export-btn').addEventListener('click', () => {
    fetch(`${API}/export`, { method: 'POST' })
      .then((r) => r.blob())
      .then((blob) => {
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'provedex-export.json';
        a.click();
        URL.revokeObjectURL(url);
      });
  });
}

function renderVerifyResult(report) {
  for (const row of eventRowsBySeq.values()) {
    row.classList.remove('broken');
  }
  if (report.status === 'valid') {
    verifyResultEl.textContent = `valid - ${report.event_count} events - root ${report.root_hash.slice(0, 16)}...`;
    verifyResultEl.className = 'ml-auto text-sm font-mono text-emerald-400';
  } else {
    verifyResultEl.textContent = `BROKEN at seq ${report.broken_at_seq} - ${report.broken_reason || ''}`;
    verifyResultEl.className = 'ml-auto text-sm font-mono text-red-400';
    const row = eventRowsBySeq.get(report.broken_at_seq);
    if (row) row.classList.add('broken');
  }
}

function wireMic() {
  if (!navigator.mediaDevices || !window.MediaRecorder) {
    micBtn.disabled = true;
    micStatus.textContent = 'mic unavailable';
    return;
  }
  micBtn.addEventListener('mousedown', startRecording);
  micBtn.addEventListener('touchstart', (e) => { e.preventDefault(); startRecording(); });
  micBtn.addEventListener('mouseup', stopRecording);
  micBtn.addEventListener('mouseleave', stopRecording);
  micBtn.addEventListener('touchend', stopRecording);
}

async function startRecording() {
  if (mediaRecorder && mediaRecorder.state === 'recording') return;
  try {
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    mediaRecorder = new MediaRecorder(stream);
    chunks = [];
    mediaRecorder.ondataavailable = (e) => { if (e.data.size > 0) chunks.push(e.data); };
    mediaRecorder.onstop = handleRecordingStopped;
    mediaRecorder.start();
    micStatus.textContent = 'recording';
    micBtn.classList.remove('bg-emerald-600');
    micBtn.classList.add('bg-red-600');
  } catch (e) {
    micStatus.textContent = `mic error: ${e.message}`;
  }
}

function stopRecording() {
  if (mediaRecorder && mediaRecorder.state === 'recording') {
    mediaRecorder.stop();
    micStatus.textContent = 'processing';
    micBtn.classList.remove('bg-red-600');
    micBtn.classList.add('bg-emerald-600');
  }
}

async function handleRecordingStopped() {
  const blob = new Blob(chunks, { type: chunks[0]?.type || 'audio/webm' });
  const fd = new FormData();
  fd.append('audio', blob, 'utterance.webm');
  try {
    const r = await fetch(`${API}/chat`, { method: 'POST', body: fd });
    if (!r.ok) {
      const t = await r.text();
      micStatus.textContent = `chat error: ${t}`;
      return;
    }
    const j = await r.json();
    if (j.response_audio_b64) {
      const audio = new Audio(`data:audio/wav;base64,${j.response_audio_b64}`);
      audio.play().catch(() => {});
    }
    micStatus.textContent = 'idle';
  } catch (e) {
    micStatus.textContent = `request failed: ${e.message}`;
  }
}

init();
