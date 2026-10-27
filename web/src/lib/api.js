// Thin fetch wrapper around the JSON API.

export class ApiError extends Error {
  constructor(message, status) {
    super(message);
    this.status = status;
  }
}

export async function api(method, path, body) {
  const opts = { method, credentials: "same-origin", headers: {} };
  if (body !== undefined) {
    opts.headers["Content-Type"] = "application/json";
    opts.body = JSON.stringify(body);
  }
  let res;
  try {
    res = await fetch(path, opts);
  } catch {
    throw new ApiError("Can't reach the server", 0);
  }
  let data = null;
  try {
    data = await res.json();
  } catch {
    /* empty body */
  }
  if (!res.ok) throw new ApiError(data?.error || `Request failed (${res.status})`, res.status);
  return data;
}

export const get = (path) => api("GET", path);
export const post = (path, body = {}) => api("POST", path, body);
export const patch = (path, body) => api("PATCH", path, body);
export const del = (path) => api("DELETE", path);

/** Uploads a File/Blob; resolves to { id, name, mime, size, url }. */
export async function upload(file) {
  const res = await fetch("/api/files", {
    method: "POST",
    credentials: "same-origin",
    headers: { "Content-Type": file.type || "application/octet-stream", "X-Filename": encodeURIComponent(file.name || "file") },
    body: file,
  });
  const data = await res.json().catch(() => null);
  if (!res.ok) throw new ApiError(data?.error || (res.status === 413 ? "That file is too large (25 MB max)" : "Upload failed"), res.status);
  return data;
}

export const qs = (params) =>
  Object.entries(params)
    .filter(([, v]) => v !== undefined && v !== null && v !== "")
    .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(v)}`)
    .join("&");
