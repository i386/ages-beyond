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
- Rust maintains in-memory diplomacy memories, named conflicts, treaty names,
  and an LLM-named current world arc director from accepted game events.
- A Markdown chronicle is written to `Chronicle/AgesBeyondChronicle.md`.
- Chronicle source events are stored in save-game state by event id.
- Fog-of-war audience facts gate whether an event can be narrated.

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

The companion ignores hidden events before calling Ollama. Known global events
can still be narrated if their location is hidden, but city and plot details are
redacted from the prompt.

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

## Diplomacy memory, named conflicts, and world arcs

The Rust companion observes accepted, player-legal game events and maintains
three in-memory director systems:

- **Diplomacy memory** records relationship facts such as wars, peace treaties,
  captured cities, and razed cities.
- **Named conflicts** ask Ollama to name wars when they begin, keep the name
  active during the conflict, and ask for a treaty or peace name when the war
  ends.
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
