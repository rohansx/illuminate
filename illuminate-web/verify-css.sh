#!/usr/bin/env bash
# Acceptance test for A1 — illuminate-web CSS theme + dashboard stylesheet.
# Asserts both stylesheets exist non-empty, the JS stub exists, and EVERY
# bespoke class token used in index.html + dashboard.html has a matching
# selector in the CSS. Mirrors the work-item verify command exactly.
set -uo pipefail
cd "$(dirname "$0")" || exit 2

fail() { echo "FAIL: $1"; exit 1; }

test -s illuminate-v4.css        || fail "illuminate-v4.css missing or empty"
test -s illuminate-dashboard.css || fail "illuminate-dashboard.css missing or empty"
test -f illuminate-v4.js         || fail "illuminate-v4.js missing"

# No inline <style> blocks allowed in the HTML (external stylesheets only).
if grep -qiE '<style[ >]' index.html dashboard.html; then
  fail "inline <style> block found in HTML — external stylesheets only"
fi

comm -23 \
  <(grep -ohE 'class="[^"]+"' index.html dashboard.html \
      | sed 's/class="//;s/"//' | tr ' ' '\n' | sort -u) \
  <(grep -ohE '\.[a-zA-Z][a-zA-Z0-9_-]*' illuminate-v4.css illuminate-dashboard.css \
      | sed 's/\.//' | sort -u) \
  | tee /tmp/missing-css-classes.txt | head

if test -s /tmp/missing-css-classes.txt; then
  echo "----"
  echo "FAIL: $(wc -l < /tmp/missing-css-classes.txt) class(es) have no CSS selector (see above / /tmp/missing-css-classes.txt)"
  exit 1
fi

echo "PASS: all bespoke classes covered; stylesheets + js stub present; no inline <style>"
