// illuminate default policy — a conservative deny → ask → allow ruleset.
// Verbs evaluate in priority order: a matching `deny` beats `ask` beats `allow`.
// No match → the engine falls through to `ask` (never a silent allow).
//
// Scope variables available to every rule: tool, cmd, path, url, cwd, home,
// session_id. Helpers: cmd.matches("glob *"), cmd.regex("^re").

// ---- deny: never, regardless of anything else ----------------------------
deny if tool == "Bash" && cmd.contains("rm -rf") && !cwd.starts_with("/tmp");
deny if tool == "Bash" && cmd.regex("\\b(curl|wget)\\b.*\\|\\s*(sudo\\s+)?(ba)?sh\\b");
deny if tool == "Bash" && cmd.contains("git push --force") && cmd.contains("main");

// ---- ask: human-in-the-loop for outward / risky surfaces -----------------
ask if tool == "Bash" && cmd.regex("\\bgit\\s+push\\b");
ask if tool == "WebFetch";
ask if tool == "Bash" && cmd.regex("\\b(sudo|chmod\\s+\\+x|chown)\\b");

// ---- allow: the normal local dev loop ------------------------------------
allow if tool == "Read";
allow if tool == "Bash" && cmd.regex("^(cargo|npm|pnpm|yarn|go|python|pytest|node)\\b");
allow if tool == "Bash" && cmd.regex("^git\\s+(status|log|diff|add|commit|branch|stash|show)\\b");
allow if tool == "Bash" && cmd.regex("^(ls|cat|grep|rg|fd|head|tail|wc|echo|pwd|which)\\b");
