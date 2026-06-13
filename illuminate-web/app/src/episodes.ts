// Per-source episode browser: the drill-down behind a clicked Sources row.
// Fetches GET /api/episodes?source=<source> and renders clickable episode
// rows (cleaned preview text) that open the episode detail slide-over.
// Honest loading / error / empty states — no fabricated rows.

import type { EpisodeListItem } from "./types.ts";
import { div, el, text } from "./dom.ts";
import { cleanSnippet, num } from "./format.ts";
import { fetchEpisodes } from "./api.ts";

/** Callback invoked with an episode id when an episode row is activated. */
export type OpenEpisode = (id: string) => void;

function episodeRow(ep: EpisodeListItem, onOpen: OpenEpisode): HTMLElement {
  const preview = cleanSnippet(ep.preview);
  const body = div("body", [
    text("div", "name", preview || ep.id),
    div("meta", [text("span", "ago", ep.id)]),
  ]);
  const row = el("button", { class: "card-row clickable ep-row", type: "button" }, [body]);
  row.addEventListener("click", () => onOpen(ep.id));
  return row;
}

/**
 * Mount the episode list for `source` into `host`: a back control to the
 * source list, then loading -> rows | empty | error.
 */
export function mountEpisodes(
  host: HTMLElement,
  source: string,
  onOpen: OpenEpisode,
  onBack: () => void,
): void {
  const back = el("button", { class: "ep-back", type: "button" });
  back.textContent = "← all sources";
  back.addEventListener("click", () => onBack());

  const panelHost = div("ep-host", []);
  host.replaceChildren(div("stack", [back, panelHost]));

  panelHost.replaceChildren(
    div("state-panel", [div("state-spinner", []), text("p", "state-title", "loading episodes…")]),
  );

  void (async () => {
    try {
      const { episodes, total } = await fetchEpisodes(source);

      const head = el("div", { class: "ph" }, []);
      head.append(text("span", "label", "episodes"));
      head.append(text("span", "title", source));
      head.append(
        text(
          "span",
          "sub",
          episodes.length < total
            ? `${num(episodes.length)} of ${num(total)}`
            : `${num(total)} episode${total === 1 ? "" : "s"}`,
        ),
      );

      const body = div("pb tight", []);
      if (episodes.length === 0) {
        body.append(text("p", "empty", `no episodes recorded for ${source}`));
      } else {
        body.append(div("card-list", episodes.map((ep) => episodeRow(ep, onOpen))));
      }

      panelHost.replaceChildren(div("panel amber", [head, body]));
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      panelHost.replaceChildren(
        div("panel rust state-panel", [
          text("p", "state-title", "couldn’t load episodes"),
          text("p", "state-sub", `GET /api/episodes?source=${source}`),
          text("p", "state-detail", message),
        ]),
      );
    }
  })();
}
