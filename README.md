# Ages Beyond

Ages Beyond is a Civilization IV: Beyond the Sword mod layered on the separate
Rust-bridge `CvGameCoreDLL` project. The goal is to make the game feel like it
has continued beyond its original edges: richer chronicles, dynamic story
hooks, and LLM-assisted flavor that reacts to real game events.

The current experiment keeps the original game rules intact. The bridge DLL
captures structured callbacks, launches the Rust companion, and Ollama
generates narrative projections that appear in-game and in a saved chronicle.

## What is working now

- DLL startup launches `mod.exe` from the mod root.
- The Rust bridge DLL sends structured callbacks over the bridge callback pipe.
- The Rust companion filters internal/noisy events before narrative generation.
- The Rust companion queues accepted bridge events as save-backed LLM jobs, so
  callback handling can continue while Ollama works.
- Ollama generates short in-world narrative text from game facts.
- Python projects notification text into the Civ IV UI.
- Rust maintains save-backed diplomacy memories, civilization memories, named
  conflicts, treaty names, per-civilization arcs, era transition memories, and
  an LLM-named current world arc director from accepted game events.
- Rust creates persistent Living Quest prompts from major accepted events, such
  as razed cities, captured cities, wonders, faiths, discoveries, war aims, and
  peace settlements.
- A Markdown chronicle is written to `Chronicle/AgesBeyondChronicle.md`.
- The current Rust director memory snapshot, seen event ids, pending LLM event
  jobs, pending quest decisions, and pending/applied quest rewards are stored in
  the Civ save through the bridge `mod_state` blob.
- `Chronicle/AgesBeyondMemory.json` is written as a debug projection of the
  save-backed director state.
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
grant applied by Rust through the bridge.

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

`CvDiplomacy.py` keeps the normal XML diplomacy line as the fallback. Generated
diplomacy now belongs on the Rust bridge path; the companion no longer supports
the old Ages Beyond companion pipe protocol.

The companion connects with `civ4::BridgeClient::connect_default_with_handshake()`,
then adapts high-signal bridge callbacks into a save-backed event-job queue. A
companion worker handles LLM/director/projection work asynchronously, and the
main bridge task remains responsible for generic bridge commands, reward
application, callback replies, and `mod_state` persistence.

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

These systems enrich later LLM prompts so diplomacy lines and chronicle entries
can refer to what has actually happened in the current game. Hidden/internal
events are filtered before they can update this memory. Rust does not reject
generated names for style; it only keeps them one-line, bounded, non-empty, and
free of raw coordinate leaks. If name generation fails, Rust stores a plain
fallback title derived from the triggering event.

The companion loads its authoritative `AgesBeyondSaveState` from the bridge
`mod_state` blob stored in the Civ save. That blob contains the director
snapshot, seen event ids, pending LLM event jobs, pending quest decisions,
chosen quest decisions, pending quest rewards, and applied reward ids.
`Chronicle/AgesBeyondMemory.json` is rewritten after accepted game events for
debugging and design inspection, but it is not loaded as canonical state.

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

Quest rewards are stored in the save blob and applied by Rust through the
bridge. The current supported command is `gold`; Living Quest stance choices
can adjust the final reward text and amount when a quest completes.

Quest decision prompts are written to `Chronicle/AgesBeyondQuestDecisions.tsv`.
Python shows supported active-player prompts as one-shot popups and appends
`Chronicle/AgesBeyondQuestDecisionResponses.tsv`; Rust ingests that response
file, stores the stance on the Living Quest and in the save blob, rewrites the
memory/log/journal projections, and includes the chosen stance in later event
and diplomacy context.

## Repository layout

- `crates/protocol/` - Rust protocol types shared by the companion.
- `crates/companion/` - Rust bridge client, director, and Ollama integration.
- `Assets/Python/` - Python notification projection loaded by the mod.
- `Companion/` - companion documentation.
- `Mod/` - packaged mod-side files.

The DLL/bridge source lives in the separate `CvGameCoreDLL` repository. Ages
Beyond consumes its built `CvGameCoreDLL.dll` and the Rust `civ4` crate from the
`feature/civ4-bridge` branch.

## Running locally

The game launches the companion automatically from the mod install. The package
places the companion at `Ages Beyond\mod.exe`, and the bridge DLL auto-enables
when that executable is present. Ollama is assumed to already be running at:

```text
http://localhost:11434
```

The companion can also be run manually for debugging after the bridge has
created its default pipes:

```cmd
mod.exe --chronicle ..\Chronicle\AgesBeyondChronicle.md --model llama3.1
```

## Build and package

Build the bridge DLL in the separate `CvGameCoreDLL` checkout, then build the
Rust companion, package the mod, and copy that external bridge DLL into
`Assets\CvGameCoreDLL.dll`.

Local validation is usually split into:

```sh
cargo fmt --all
cargo test --workspace
```

`scripts/package_mod.ps1` defaults to copying the DLL from
`..\CvGameCoreDLL\artifacts\CvGameCoreDLL.dll`; pass `-DllPath` if your bridge
checkout or build artifact lives elsewhere.

## Current design rule

Keep engine changes in the separate bridge repository. The DLL should expose
structured, safe facts and generic game-state commands. Ages Beyond Rust should
own listener behavior, async LLM job scheduling, LLM prompting, narrative
policy, fallbacks, filtering, and most future experiment logic.
