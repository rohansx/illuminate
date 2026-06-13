// Formatting + safe-parsing helpers. All API data flows through these before
// reaching the DOM, so nothing is ever fabricated and nothing is injected raw.

/** Thousands-separated integer, e.g. 6012568 -> "6,012,568". */
export function num(n: unknown): string {
  const v = typeof n === "number" && Number.isFinite(n) ? n : 0;
  return v.toLocaleString("en-US");
}

/** A percentage like 100 -> "100%", 87.5 -> "87.5%". */
export function pct(n: unknown): string {
  const v = typeof n === "number" && Number.isFinite(n) ? n : 0;
  // Drop a trailing ".0" so 100.0 reads as "100%".
  const s = Number.isInteger(v) ? String(v) : v.toFixed(1);
  return `${s}%`;
}

/**
 * Strip markdown syntax noise (code fences, inline backticks, heading hashes,
 * bold/italic asterisks, blockquote markers, table pipes) from a snippet or
 * preview so list rows read as plain prose. Conservative on purpose: it never
 * touches underscores or word characters, so identifiers survive intact.
 */
export function cleanSnippet(s: string): string {
  return (s ?? "")
    .replace(/```[^\n`]*/g, " ") // code-fence openers/closers
    .replace(/`([^`]*)`/g, "$1") // inline code
    .replace(/^#{1,6}\s+/gm, "") // heading markers
    .replace(/^>\s?/gm, "") // blockquote markers
    .replace(/\*\*([^*]+)\*\*/g, "$1") // bold
    .replace(/\*([^*]+)\*/g, "$1") // italics
    .replace(/^[\s|:-]+$/gm, " ") // table separator rows
    .replace(/\|/g, " ") // table pipes
    .replace(/\s+/g, " ")
    .trim();
}

/** Relative time from an RFC-3339 stamp, e.g. "3m ago", "just now". */
export function relativeTime(rfc3339: string): string {
  const t = Date.parse(rfc3339);
  if (Number.isNaN(t)) return "unknown";
  const diffMs = Date.now() - t;
  const sec = Math.round(diffMs / 1000);
  if (sec < 45) return "just now";
  const min = Math.round(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.round(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.round(hr / 24);
  if (day < 30) return `${day}d ago`;
  const mon = Math.round(day / 30);
  if (mon < 12) return `${mon}mo ago`;
  return `${Math.round(mon / 12)}y ago`;
}
