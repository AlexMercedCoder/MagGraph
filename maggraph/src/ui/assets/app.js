const API = "/api";

let selectedId = null;

const nodeList = document.getElementById("node-list");
const edgeList = document.getElementById("edge-list");
const emptyState = document.getElementById("empty-state");
const nodeDetail = document.getElementById("node-detail");
const statusEl = document.getElementById("status");

function setStatus(message, kind = "") {
  statusEl.textContent = message;
  statusEl.className = "status" + (kind ? " " + kind : "");
}

async function fetchJson(path, options = {}) {
  const res = await fetch(API + path, {
    headers: { "Content-Type": "application/json", ...options.headers },
    ...options,
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error || res.statusText);
  }
  if (res.status === 204) return null;
  return res.json();
}

async function loadNodes() {
  const nodes = await fetchJson("/nodes");
  nodeList.innerHTML = "";
  for (const node of nodes) {
    const li = document.createElement("li");
    const btn = document.createElement("button");
    btn.type = "button";
    btn.textContent = node.id;
    btn.dataset.id = node.id;
    if (node.id === selectedId) btn.classList.add("active");
    btn.addEventListener("click", () => selectNode(node.id));
    li.appendChild(btn);
    nodeList.appendChild(li);
  }
}

async function loadEdges() {
  const data = await fetchJson("/edges");
  edgeList.innerHTML = "";
  for (const edge of data.edges) {
    const li = document.createElement("li");
    li.textContent = `${edge.from} → ${edge.to}`;
    edgeList.appendChild(li);
  }
  for (const u of data.unresolved) {
    const li = document.createElement("li");
    li.textContent = `${u.from} ⇢ ?${u.target}`;
    li.style.color = "#c69026";
    edgeList.appendChild(li);
  }
}

async function selectNode(id) {
  selectedId = id;
  document.querySelectorAll(".node-list button").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.id === id);
  });

  const node = await fetchJson(`/nodes/${encodeURIComponent(id)}`);
  emptyState.classList.add("hidden");
  nodeDetail.classList.remove("hidden");

  document.getElementById("detail-id").textContent = node.id;
  document.getElementById("detail-type").textContent = node.type;
  document.getElementById("detail-path").textContent = node.relative_path || "—";
  document.getElementById("detail-source").textContent = node.source || "—";
  document.getElementById("detail-links").textContent =
    (node.links && node.links.length) ? node.links.join(", ") : "—";
  document.getElementById("detail-body").value = node.body || "";
  setStatus("");
}

async function saveNode() {
  if (!selectedId) return;
  const body = document.getElementById("detail-body").value;
  try {
    await fetchJson(`/nodes/${encodeURIComponent(selectedId)}`, {
      method: "PATCH",
      body: JSON.stringify({ body }),
    });
    setStatus("Saved.", "ok");
    await loadEdges();
  } catch (e) {
    setStatus(e.message, "err");
  }
}

async function deleteNode() {
  if (!selectedId) return;
  if (!confirm(`Delete node "${selectedId}"?`)) return;
  try {
    await fetchJson(`/nodes/${encodeURIComponent(selectedId)}`, {
      method: "DELETE",
    });
    selectedId = null;
    nodeDetail.classList.add("hidden");
    emptyState.classList.remove("hidden");
    setStatus("Deleted.", "ok");
    await loadNodes();
    await loadEdges();
  } catch (e) {
    setStatus(e.message, "err");
  }
}

document.getElementById("refresh-nodes").addEventListener("click", async () => {
  await loadNodes();
  await loadEdges();
});
document.getElementById("save-node").addEventListener("click", saveNode);
document.getElementById("delete-node").addEventListener("click", deleteNode);

loadNodes().catch((e) => setStatus(e.message, "err"));
loadEdges().catch((e) => setStatus(e.message, "err"));
