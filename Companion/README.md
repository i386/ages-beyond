# Ages Beyond Companion

`AgesBeyondCompanion.exe` is built from `crates/companion` and is launched by
the Civ IV DLL over a Windows named pipe.

The DLL searches for the executable next to `CvGameCoreDLL.dll`, in
`..\Companion\AgesBeyondCompanion.exe`, and in `..\AgesBeyondCompanion.exe`.

The first LLM provider is Ollama. The companion assumes Ollama is already running at
`http://localhost:11434` unless a different `--ollama-url` is supplied.

The DLL stores the canonical structured event ledger in the save game. The
companion writes generated prose as a Markdown projection to
`..\Chronicle\AgesBeyondChronicle.md`.

The companion owns event listener behavior: it classifies incoming DLL events,
filters internal engine events such as barbarian setup diplomacy, applies
audience/fog-of-war gating, calls Ollama for chronicle-worthy events, and skips
duplicate Markdown projections by saved event id.

DLL events include names, type keys, era/chapter metadata, importance,
audience/visibility metadata, and quest-policy hints where available. The
save-game ledger keeps these structured facts; the Markdown chronicle is a
chaptered projection of that ledger.

For contract version 3 events, Rust ignores events that are not known to the
active player before calling Ollama. Coordinates can be used as private prompt
grounding for known events, but final player-facing text is sanitized so raw
map coordinates are not shown. If a global announcement is known but its
location is hidden, location facts are redacted before prompt construction.

Ollama is prompted to return a compact experimental narrative packet:

```text
Chronicle: One short historical line.
Council: Optional roleplay advice.
Quest Hook: Optional unresolved narrative prompt.
World Arc: Optional mention of the current historical arc.
```

Only the first `Chronicle:` line is written to the in-game notification TSV.
The full packet is written to `AgesBeyondChronicle.md`. If Ollama fails, the
companion emits deterministic fallback text in the same format.

The same named pipe also supports cache-first generated diplomacy text. The DLL
sends `diplomacy_text` requests when Python shows a leader comment. Rust returns
an in-memory cached line if one is ready; otherwise it starts background Ollama
generation and returns an empty string so the diplomacy screen keeps its vanilla
XML fallback.

The companion also keeps an in-memory director state. Accepted game events feed
relationship memories, named conflicts, per-civilization arcs, and a current
world arc. When war starts, Rust asks Ollama to name the conflict and keeps
that name active for later chronicle and diplomacy prompts. When peace is
signed, Rust asks for a treaty or peace name and records the closed conflict as
relationship memory. For major civilization-specific events, Rust asks Ollama
to name that civilization's current arc from its own settlements, wars,
discoveries, wonders, faiths, conquests, and golden ages. When a major world
trigger appears, Rust asks Ollama to name the global arc from the actual
civilizations, places, faiths, wonders, and conflicts in the game, then stores
the result in memory. Rust only applies structural cleanup to generated names:
one line, bounded length, non-empty, and no raw coordinate leaks. Diplomacy
generation receives the relevant relationship memory between the active player
and leader, active/recent conflict names, both civilizations' arcs, and the
current world arc. Chronicle generation receives the current arc, conflict
context, civilization arc context, and recent world-event summaries as
continuity context.

Example:

```cmd
AgesBeyondCompanion.exe --pipe \\.\pipe\AgesBeyond-12345 --chronicle ..\Chronicle\AgesBeyondChronicle.md --model llama3.1
```
