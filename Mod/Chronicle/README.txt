Generated chronicle entries are written here at runtime.

AgesBeyondChronicle.md contains the full LLM narrative packet for each accepted
game event. In-game notifications show only the first Chronicle line so the UI
stays readable.

The Rust companion filters hidden/internal events before generation. Known
events can use private map context for grounding, but player-facing text should
not expose raw map coordinates.
