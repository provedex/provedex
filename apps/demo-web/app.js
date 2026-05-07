const API = '/api';
const eventsEl = document.getElementById('events');
const eventCountEl = document.getElementById('event-count');
const conversationEl = document.getElementById('conversation');
const verifyResultEl = document.getElementById('verify-result');
const sessionIdEl = document.getElementById('session-id');
const pubkeyEl = document.getElementById('pubkey');
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
    sessionIdEl.textContent = j.session_id.slice(0, 8);
    pubkeyEl.textContent = j.pubkey.slice(0, 16);
  } catch (e) {
    sessionIdEl.textContent = 'offline';
    pubkeyEl.textContent = '----';
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
    es.close();
    setTimeout(subscribeEvents, 2000);
  };
}

function addEventRow(evt) {
  if (eventRowsBySeq.has(evt.seq)) return;
  const row = document.createElement('div');
  row.className = 'evt-row';
  row.dataset.seq = evt.seq;
  const type = evt.event && evt.event.type ? evt.event.type : 'Unknown';
  const shortHash = evt.self_hash ? evt.self_hash.slice(0, 16) : '';
  const ts = formatTs(evt.timestamp_nanos);
  row.innerHTML =
    `<span class="col-seq">${String(evt.seq).padStart(3, '0')}</span>` +
    `<span class="col-type">${type}</span>` +
    `<span class="col-hash">${shortHash}</span>` +
    `<span class="col-ts">${ts}</span>`;
  eventsEl.appendChild(row);
  eventsEl.scrollTop = eventsEl.scrollHeight;
  eventRowsBySeq.set(evt.seq, row);
  eventCount += 1;
  eventCountEl.textContent = `${eventCount} ${eventCount === 1 ? 'event' : 'events'}`;

  if (evt.event && evt.event.type === 'UtteranceCaptured' && evt.event.payload) {
    addMessage('user', evt.event.payload.transcript || '');
  }
  if (evt.event && evt.event.type === 'UtteranceSpoken' && evt.event.payload) {
    addMessage('agent', evt.event.payload.text || '');
  }
}

function formatTs(nanos) {
  if (!nanos) return '';
  const ms = Number(BigInt(nanos) / 1000000n);
  const d = new Date(ms);
  return d.toISOString().replace('T', ' ').replace('Z', '');
}

function addMessage(role, text) {
  if (!text) return;
  const empty = document.getElementById('conv-empty');
  if (empty) empty.remove();
  const row = document.createElement('div');
  row.className = 'msg';
  const r = document.createElement('span');
  r.className = `msg-role ${role}`;
  r.textContent = role === 'user' ? 'user' : 'agent';
  const t = document.createElement('span');
  t.className = 'msg-text';
  t.textContent = text;
  row.appendChild(r);
  row.appendChild(t);
  conversationEl.appendChild(row);
  conversationEl.scrollTop = conversationEl.scrollHeight;
}

function wireButtons() {
  document.getElementById('verify-btn').addEventListener('click', async () => {
    setVerifyResult('verifying', 'warn');
    try {
      const r = await fetch(`${API}/verify`, { method: 'POST' });
      const j = await r.json();
      renderVerifyResult(j);
    } catch (e) {
      setVerifyResult(`error: ${e.message || e}`, 'fail');
    }
  });
  document.getElementById('tamper-btn').addEventListener('click', async () => {
    if (!confirm('tamper-test will mutate one event in the local ledger so the chain visibly breaks. continue?')) return;
    try {
      const r = await fetch(`${API}/tamper-test`, { method: 'POST' });
      if (!r.ok) throw new Error(await r.text());
      const j = await r.json();
      setVerifyResult(`tampered seq ${j.tampered_seq}; click verify`, 'warn');
    } catch (e) {
      setVerifyResult(`tamper failed: ${e.message || e}`, 'fail');
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

function setVerifyResult(text, state) {
  verifyResultEl.textContent = text;
  verifyResultEl.className = `foot-right mono ${state || ''}`;
}

function renderVerifyResult(report) {
  for (const row of eventRowsBySeq.values()) {
    row.classList.remove('broken');
  }
  if (report.status === 'valid') {
    setVerifyResult(
      `chain valid - root ${report.root_hash.slice(0, 16)}`,
      'pass',
    );
  } else {
    const seq = String(report.broken_at_seq).padStart(3, '0');
    setVerifyResult(`chain broken at seq ${seq}`, 'fail');
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
  micBtn.addEventListener('touchstart', (e) => {
    e.preventDefault();
    startRecording();
  });
  micBtn.addEventListener('mouseup', stopRecording);
  micBtn.addEventListener('mouseleave', stopRecording);
  micBtn.addEventListener('touchend', stopRecording);

  // Push-to-talk: hold spacebar. Skip auto-repeat and any text input focus.
  document.addEventListener('keydown', (e) => {
    if (e.code !== 'Space' || e.repeat) return;
    const tag = (document.activeElement && document.activeElement.tagName) || '';
    if (tag === 'INPUT' || tag === 'TEXTAREA') return;
    e.preventDefault();
    startRecording();
  });
  document.addEventListener('keyup', (e) => {
    if (e.code !== 'Space') return;
    e.preventDefault();
    stopRecording();
  });
}

async function startRecording() {
  if (mediaRecorder && mediaRecorder.state === 'recording') return;
  try {
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    mediaRecorder = new MediaRecorder(stream);
    chunks = [];
    mediaRecorder.ondataavailable = (e) => {
      if (e.data.size > 0) chunks.push(e.data);
    };
    mediaRecorder.onstop = handleRecordingStopped;
    mediaRecorder.start();
    micStatus.textContent = 'recording';
    micBtn.classList.add('recording');
  } catch (e) {
    micStatus.textContent = `mic error: ${e.message}`;
  }
}

function stopRecording() {
  if (mediaRecorder && mediaRecorder.state === 'recording') {
    mediaRecorder.stop();
    micStatus.textContent = 'processing';
    micBtn.classList.remove('recording');
  }
}

async function handleRecordingStopped() {
  const blob = new Blob(chunks, { type: chunks[0]?.type || 'audio/webm' });
  const fd = new FormData();
  fd.append('audio', blob, 'utterance.webm');
  try {
    const r = await fetch(`${API}/chat`, { method: 'POST', body: fd });
    if (!r.ok) {
      micStatus.textContent = `chat error: ${await r.text()}`;
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
