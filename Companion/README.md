# Ages Beyond Companion

`mod.exe` is built from `crates/companion` and is launched by the Rust bridge
`CvGameCoreDLL`.

The packaged mod installs the companion as `mod.exe` at the mod root. The DLL
looks for it at `..\mod.exe` relative to `Assets\CvGameCoreDLL.dll`.

The first LLM provider is Ollama. The companion assumes Ollama is already running at
`http://localhost:11434` unless a different `--ollama-url` is supplied.

The DLL stores the canonical structured event ledger in the save game. The
companion writes generated prose as a Markdown projection to
`..\Chronicle\AgesBeyondChronicle.md` and rewrites the current director memory
snapshot to `..\Chronicle\AgesBeyondMemory.json`.

The companion owns event listener behavior: it classifies incoming DLL events,
filters internal engine events such as barbarian setup diplomacy, applies
audience/fog-of-war gating, calls Ollama for chronicle-worthy events, and skips
duplicate Markdown projections by saved event id.

DLL events include names, type keys, era/chapter metadata, importance,
audience/visibility metadata, and quest-policy hints where available. The
save-game ledger keeps these structured facts; the Markdown chronicle is a
chaptered projection of that ledger.

For contract version 3 events, Rust ignores events that are not known to the
active player before calling Ollama unless the DLL marks the event as
`rumor_possible` with a plausible `rumor_channel`. Rumors are projected as
sanitized `rumor` events with vague hearsay, no coordinates, no hidden city
names, and no hidden source facts stored in director memory. Coordinates can be
used as private prompt grounding for known events, but final player-facing text
is sanitized so raw map coordinates are not shown. If a global announcement is
known but its location is hidden, location facts are redacted before prompt
construction.

Ollama is prompted to return a compact experimental narrative packet:

```text
Chronicle: One short historical line.
Council: Optional roleplay advice.
Quest Hook: Optional unresolved narrative prompt.
World Arc: Optional mention of the current historical arc.
```

Only the first `Chronicle:` line is written to the in-game notification TSV.
The full packet is written to `AgesBeyondChronicle.md`. Deterministic
`Memory:` projections may also be appended to the Markdown chronicle when a
civilization gains new remembered history, but those memory lines are not
written to the in-game notification TSV. Deterministic `Quest:` projections may
also be appended to a `Living Quests` chapter when accepted events create or
complete Rust-owned Living Quests. Quest creation and completion are written to
`AgesBeyondQuestNotifications.tsv`, which Python polls separately from ordinary
chronicle notifications so quest messages can appear in game without changing
normal chronicle popup behavior. The companion also rewrites
`AgesBeyondQuestLog.md` as an inspectable active/completed quest log generated
from the same persisted director snapshot, plus `AgesBeyondQuestJournal.tsv`
as a compact active/completed summary for Python to show in game when the
journal changes. Completed active-player quests can also write
`AgesBeyondQuestRewards.tsv` commands; Python currently supports idempotent
`gold` rewards, with stance choices able to adjust completion reward text and
amount. New active-player quests can write `AgesBeyondQuestDecisions.tsv`
prompts; Python shows them as one-shot stance popups, stores the selected
stance in save-game script data, and appends
`AgesBeyondQuestDecisionResponses.tsv` for Rust to ingest into Living Quest
memory. If Ollama fails, the companion emits
deterministic fallback text in the same format.

The companion only supports the Rust bridge connection path. It connects to the
DLL bridge using the default bridge pipe names. The bridge DLL auto-enables and
launches it when the packaged `mod.exe` is present.

The companion also keeps an in-memory director state. Accepted game events feed
relationship memories, civilization memories, named conflicts,
per-civilization arcs, Living Quests, era transition memories, and a current
world arc. Civilization memory records each civilization's own remembered
events, such as founded cities, lost or razed cities, discoveries, wonders,
faiths, golden ages, great people, wars, and peace settlements. Living Quests
are persistent narrative prompts created from major events, including
restoration claims after city loss or razing, legitimacy prompts after
conquest, wonder legacy prompts, faith prompts, breakthrough prompts, war aims,
and peace settlement prompts. Quests complete from later structured milestones
such as city restoration, peace, conquest, wonders, projects, discoveries,
golden ages, great people, and victories. Each quest carries objective text,
numeric progress, reward text, and consequence text. Completed active-player
quests can now apply supported reward commands through Python, and new
active-player quests can ask the player to choose a remembered stance. When war
starts, Rust asks Ollama to name the conflict and keeps that name active for
later chronicle and diplomacy prompts. When peace is signed, Rust asks for a
treaty or peace name and records the closed conflict as relationship memory.
For major civilization-specific
events, Rust asks Ollama to name that civilization's current arc from its own
settlements, wars, discoveries, wonders, faiths, conquests, and golden ages.
When a tech discovery crosses a civilization into a new era, Rust emits an
internal `era_transition` event, projects it into the chronicle and in-game
notifications, and remembers the transition for later diplomacy. When a major
world trigger appears, Rust asks Ollama to name the global arc from the actual
civilizations, places, faiths, wonders, and conflicts in the game, then stores
the result in memory. Rust only applies structural cleanup to generated names:
one line, bounded length, non-empty, and no raw coordinate leaks. Diplomacy
generation receives the relevant relationship memory between the active player
and leader, active/recent conflict names, both civilizations' arcs, both
civilizations' memories, active Living Quests, era memories, and the current
world arc. Chronicle generation receives the current arc, conflict context,
civilization memory, active Living Quests, civilization arc context, era memory,
and recent world-event summaries as continuity context.

`AgesBeyondMemory.json` is intended for debugging, design iteration, and
companion restart persistence. On startup, Rust restores director memory from
this file when it exists and matches the supported format version. If the file
is missing, invalid, or from an unsupported future format, Rust starts with
clean memory and logs the reason. The file contains recent world events, the
current world arc, civilization memories, civilization arcs, relationship
memories, active conflicts, active or completed Living Quests, and recent
closed conflicts as currently held by the Rust companion.

Example:

```cmd
mod.exe --chronicle ..\Chronicle\AgesBeyondChronicle.md --model llama3.1
```
