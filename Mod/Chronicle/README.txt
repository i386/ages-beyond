Generated chronicle entries are written here at runtime.

AgesBeyondChronicle.md contains the full LLM narrative packet for each accepted
game event. In-game notifications show only the first Chronicle line so the UI
stays readable.

The chronicle can also contain Markdown-only Memory entries when Rust records
new civilization memories. These entries are not shown as in-game notifications.

The chronicle can also contain a Markdown-only Living Quests chapter when Rust
creates or completes persistent narrative quest prompts. These entries are not
shown through the ordinary chronicle notification file.

AgesBeyondQuestNotifications.tsv contains quest creation and completion
notifications for Python to show in game separately from normal chronicle
messages.

AgesBeyondQuestLog.md contains an active/completed Living Quest log rewritten
by the Rust companion after accepted game events.

AgesBeyondQuestJournal.tsv contains a compact Living Quest summary for Python
to show bounded in-game quest journal updates.

Living Quest rewards are applied by Rust through the bridge and persisted in
the Civ save. This directory contains projections and UI handoff files, not
mechanical reward command state.

AgesBeyondQuestDecisions.tsv contains Living Quest stance prompts. Python shows
active-player prompts as one-shot popups.

AgesBeyondQuestDecisionResponses.tsv contains the chosen Living Quest stances
that Python writes back for Rust to ingest into quest memory and save state.

AgesBeyondMemory.json contains the current Rust director memory snapshot for
debugging and design iteration. Rust restores canonical companion state from
the Civ save through the bridge mod_state blob, not from this projection.

The Rust companion filters hidden/internal events before generation. Known
events can use private map context for grounding, but player-facing text should
not expose raw map coordinates.
