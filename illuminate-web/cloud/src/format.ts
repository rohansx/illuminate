// Small pure formatting helpers.

/** Group digits with thin separators: 1240 → "1,240". */
export function num(n: number): string {
  return n.toLocaleString("en-US");
}

/** Collapse whitespace/newlines in an episode preview to one tidy line. */
export function cleanPreview(s: string): string {
  return s.replace(/\s+/g, " ").trim();
}

/** Title-case a path basename for display, leaving real names mostly intact. */
export function repoLabel(name: string): string {
  return name;
}

/** Short uppercase initials for an avatar, from a name or email. */
export function initials(nameOrEmail: string): string {
  const base = nameOrEmail.includes("@") ? nameOrEmail.split("@")[0] : nameOrEmail;
  const parts = base.split(/[\s._-]+/).filter(Boolean);
  if (parts.length === 0) return "?";
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return (parts[0][0] + parts[1][0]).toUpperCase();
}
