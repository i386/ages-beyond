# Ages Beyond

Ages Beyond is a Civilization IV: Beyond the Sword mod built on the MMod
`CvGameCoreDLL`. The goal is to make the game feel like it has continued
beyond its original edges: richer chronicles, dynamic story hooks, and
LLM-assisted flavor that reacts to real game events.

The current experiment keeps the original game rules intact. The DLL captures
structured events, a Rust companion listens over a Windows named pipe, and
Ollama generates narrative projections that appear in-game and in a saved
chronicle.

## What is working now

- DLL startup launches `AgesBeyondCompanion.exe`.
- The DLL sends structured game events over a named pipe.
- The Rust companion filters internal/noisy events before narrative generation.
- Ollama generates short in-world narrative text from game facts.
- Python projects notification text into the Civ IV UI.
- Diplomacy text can request generated replacements through the existing named
  pipe and fall back to vanilla XML text when no cached line is ready.
- Rust maintains in-memory diplomacy memories, civilization memories, named
  conflicts, treaty names, per-civilization arcs, era transition memories, and
  an LLM-named current world arc director from accepted game events.
- Rust creates persistent Living Quest prompts from major accepted events, such
  as razed cities, captured cities, wonders, faiths, discoveries, war aims, and
  peace settlements.
- A Markdown chronicle is written to `Chronicle/AgesBeyondChronicle.md`.
- A current Rust director memory snapshot is written to
  `Chronicle/AgesBeyondMemory.json`.
- Chronicle source events are stored in save-game state by event id.
- Fog-of-war audience facts gate whether an event can be narrated or reduced
  to a vague rumor.

## LLM narrative packet

The companion now asks Ollama for a small labeled packet:

```text
Chronicle: A short historical line for the event.
Council: Optional roleplay advice for the active player.
Quest Hook: Optional unresolved narrative prompt.
World Arc: Optional mention of the current age, conflict, or legacy.
```

Only the first `Chronicle:` line is projected as an in-game notification. The
full packet is written to the Markdown chronicle so experiments can be richer
without flooding the game UI.

The companion may also append deterministic `Memory:` entries to the Markdown
chronicle when a civilization gains a new remembered event. These entries are
not projected into the in-game notification TSV.

The companion may append deterministic `Quest:` entries to a `Living Quests`
chapter when accepted events create or complete a Rust-owned Living Quest.
Quest creation and completion are also written to a separate quest notification
TSV so Python can show them in game without mixing them into ordinary chronicle
notifications. These are structured narrative objectives with progress,
reward, and consequence metadata. Completed active-player quests can emit a
reward command; the first implemented mechanical reward is an idempotent gold
grant applied by Python.

If Ollama fails or times out, the Rust companion produces deterministic
fallback text in the same packet format.

## Fog of war

Coordinates may be sent to the LLM as private grounding context when the event
is already known to the active player. Player-facing text must not print raw map
coordinates.

For contract version 3 events, the DLL includes audience fields such as:

- `known_to_active_player`
- `location_known_to_active_player`
- `plot_visibility`
- `involves_active_player`
- `involves_active_team`
- `is_global_announcement`

The companion ignores hidden events before calling Ollama unless the DLL marks
the event as `rumor_possible` with a plausible `rumor_channel`, such as
travellers, displaced witnesses, or merchant reports. Rumors are projected as
new sanitized `rumor` events; they do not feed hidden source facts into
diplomacy memory, named conflicts, civilization arcs, era memory, or world
arcs. Known global events can still be narrated if their location is hidden,
but city and plot details are redacted from the prompt.

## Dynamic diplomacy text

`CvDiplomacy.py` keeps the normal XML diplomacy line as the fallback. When a
leader comment is shown, Python asks the DLL for an Ages Beyond replacement.
The DLL sends a `diplomacy_text` request over the existing companion named pipe.

Rust keeps generated diplomacy lines in memory. On a cache hit, the generated
line is returned quickly and replaces the XML line. On a cache miss, Rust starts
background Ollama generation and immediately returns an empty response, so the
diplomacy screen uses the vanilla XML line for that encounter.

The cache key includes the comment type, the two players, a coarse turn bucket,
and the leader attitude. This keeps lines fresh enough without blocking the UI
or writing extra diplomacy cache files.

The companion also has a bridge-client runtime for the newer Rust
`CvGameCoreDLL` bridge. If `AgesBeyondCompanion.exe` starts without `--pipe`, it
connects with `civ4::BridgeClient::connect_from_env_with_handshake()` and
adapts high-signal bridge callbacks into the same Rust director pipeline. Use
`--engine pipe` to force the legacy companion pipe or `--engine bridge` to force
the new bridge.

## Diplomacy memory, quests, named conflicts, and arcs

The Rust companion observes accepted, player-legal game events and maintains
six in-memory director systems:

- **Diplomacy memory** records relationship facts such as wars, peace treaties,
  captured cities, and razed cities.
- **Civilization memory** records each civilization's own remembered events,
  such as founded cities, razed or lost cities, wonders, discoveries, faiths,
  golden ages, great people, wars, and peace settlements.
- **Living Quests** create persistent narrative prompts from major events,
  including restoration claims after city loss or razing, legitimacy prompts
  after conquest, wonder legacy prompts, faith prompts, breakthrough prompts,
  war aims, and peace settlement prompts. Quests complete from later structured
  milestones such as city restoration, peace, conquest, wonders, projects,
  discoveries, golden ages, great people, and victories. Each quest carries
  objective text, numeric progress, reward text, and consequence text for later
  UI and mechanical hooks.
- **Named conflicts** ask Ollama to name wars when they begin, keep the name
  active during the conflict, and ask for a treaty or peace name when the war
  ends.
- **Civilization arcs** ask Ollama to name each civilization's current story
  from its own events, such as settlements, wars, discoveries, wonders, faiths,
  conquests, and golden ages.
- **Era transition narrator** detects when a civilization crosses into a new
  tech era, emits an internal chronicle event, and remembers the transition for
  later diplomacy.
- **World arc director** tracks recent world events and asks Ollama to name the
  current historical arc from the civilizations, places, faiths, wonders, and
  conflicts actually present in the game.

These systems do not directly change game mechanics yet. They enrich later LLM
prompts so diplomacy lines and chronicle entries can refer to what has actually
happened in the current game. Hidden/internal events are filtered before they
can update this memory. Rust does not reject generated names for style; it only
keeps them one-line, bounded, non-empty, and free of raw coordinate leaks. If
name generation fails, Rust stores a plain fallback title derived from the
triggering event.

For debugging, inspection, and companion restart persistence, the companion
loads `Chronicle/AgesBeyondMemory.json` on startup and rewrites it after
accepted game events. The file contains recent world events, the current world
arc, civilization memories, civilization arcs, relationship memories, active
conflicts, living quests, and recent closed conflicts. If the file is missing,
invalid, or from an unsupported future format, Rust starts with clean memory and
logs the reason.

The companion also rewrites `Chronicle/AgesBeyondQuestLog.md` after accepted
game events. This Markdown log is a readable projection of the same persisted
Living Quest state, split into active and completed quests for quick inspection
outside the game.

For in-game journal awareness, the companion rewrites
`Chronicle/AgesBeyondQuestJournal.tsv` with a compact active/completed summary.
The Python UI poller watches that file and shows a bounded `Quest Journal:`
message only when the summary changes.

Quest notifications are written separately to
`Chronicle/AgesBeyondQuestNotifications.tsv`. The Python UI poller watches this
file independently from `AgesBeyondNotifications.tsv` and shows quest messages
with a `Quest:` label.

Quest reward commands are written separately to
`Chronicle/AgesBeyondQuestRewards.tsv`. Python applies supported active-player
commands once per save by recording reward ids in `CyGame` script data. The
current supported command is `gold`; Living Quest stance choices can adjust the
final reward text and amount when a quest completes.

Quest decision prompts are written to `Chronicle/AgesBeyondQuestDecisions.tsv`.
Python shows supported active-player prompts as one-shot popups and records the
chosen stance in `CyGame` script data. Python also appends
`Chronicle/AgesBeyondQuestDecisionResponses.tsv`; Rust ingests that response
file, stores the stance on the Living Quest, rewrites the memory/log/journal
projections, and includes the chosen stance in later event and diplomacy
context.

## Repository layout

- `CvGameCoreDLL/` - C++ DLL source and Ages Beyond event hooks.
- `crates/protocol/` - Rust protocol types shared by the companion.
- `crates/companion/` - Rust named-pipe listener and Ollama integration.
- `Assets/Python/` - Python notification projection loaded by the mod.
- `Companion/` - packaged companion documentation.
- `Mod/` - packaged mod-side files.

## Running locally

The game launches the companion automatically from the mod install. Ollama is
assumed to already be running at:

```text
http://localhost:11434
```

The companion can also be run manually for debugging:

```cmd
AgesBeyondCompanion.exe --pipe \\.\pipe\AgesBeyond-12345 --chronicle ..\Chronicle\AgesBeyondChronicle.md --model llama3.1
```

## Build and package

The GitHub Actions workflow builds the legacy 32-bit `CvGameCoreDLL.dll`, builds
the Rust companion, packages the mod, and uploads a downloadable zip artifact.

Local validation is usually split into:

```sh
cargo fmt --all
cargo test --workspace
```

The DLL itself requires the Visual C++ 7.1 toolchain; see
`CvGameCoreDLL/README.md` for details.

## Current design rule

Keep C++ changes small and engine-facing. The DLL should expose structured,
safe facts. Rust should own listener behavior, LLM prompting, narrative policy,
fallbacks, filtering, and most future experiment logic.
