// illuminate minimalist policy — the "write less" layer.
//
// Stacks on top of the conservative default ruleset. It encodes ponytail's
// decision ladder ("the best code is the code you never write") as `ask` rules
// that pause an agent the moment it reaches for a NEW dependency, so a human
// can apply the ladder before a package lands in the tree:
//
//   1. does this need to exist at all?
//   2. can the standard library do it?
//   3. can a native platform feature do it?
//   4. can a dependency you ALREADY have do it?
//   5. is it one line?
//   6. otherwise: the minimum that works.
//
// Carve-out: this never touches validation, security, error handling, or
// accessibility — those are not the code to cut. It only gates dependency
// growth. Since the engine groups rules by verb (deny -> ask -> allow), these
// `ask`s win over the default's broad `allow` on package managers; syncing from
// an existing manifest (`npm install`, `pip install -r`, `cargo build`) adds
// nothing and falls through to the default allow.
//
// Patterns use `[^-\s]` (first arg is not a flag) — NOT lookahead, which the
// regex engine doesn't support. The `=> "…"` suffix is the reason the agent
// sees when the rule fires.

// ---- ask: pause before adding a new dependency ---------------------------
ask if tool == "Bash" && cmd.regex("\\bnpm\\s+(i|install|add)\\s+[^-\\s]") => "new npm dependency — does it need to exist? stdlib / a native feature / a dep you already have first; prefer the minimum that works (validation/security/a11y excepted)";
ask if tool == "Bash" && cmd.regex("\\b(yarn|pnpm)\\s+add\\s+[^-\\s]") => "new JS dependency — apply the ladder: needed? stdlib? native? existing dep? one line? (validation/security/a11y excepted)";
ask if tool == "Bash" && cmd.regex("\\bcargo\\s+add\\s+\\S") => "new crate — does it need to exist? std / an existing dep first; prefer the minimum that works (validation/security/a11y excepted)";
ask if tool == "Bash" && cmd.regex("\\bpip\\s+install\\s+[^-\\s]") => "new Python package — needed? stdlib / an installed package first; prefer the minimum that works (validation/security/a11y excepted)";
ask if tool == "Bash" && cmd.regex("\\bgo\\s+get\\s+[^-\\s]") => "new Go module — does it need to exist? stdlib / an existing dep first; prefer the minimum that works (validation/security/a11y excepted)";
ask if tool == "Bash" && cmd.regex("\\bpoetry\\s+add\\s+[^-\\s]") => "new Python dependency — apply the ladder before it lands in pyproject (validation/security/a11y excepted)";

// ---- ask: scaffolding generators pull in whole trees ---------------------
ask if tool == "Bash" && cmd.regex("\\bnpx\\s+create-") => "project scaffolder — this pulls a whole dependency tree; confirm you want all of it before generating";
ask if tool == "Bash" && cmd.regex("\\bnpm\\s+init\\s+[^-\\s]") => "project scaffolder — `npm init <generator>` pulls a whole dependency tree; confirm before generating";
