# Ages Beyond Companion

`AgesBeyondCompanion.exe` is built from `crates/companion` and is launched by
the Civ IV DLL over a Windows named pipe.

The DLL searches for the executable next to `CvGameCoreDLL.dll`, in
`..\Companion\AgesBeyondCompanion.exe`, and in `..\AgesBeyondCompanion.exe`.

The first provider is Ollama. The companion assumes Ollama is already running at
`http://localhost:11434` unless a different `--ollama-url` is supplied.

The DLL stores the canonical structured event ledger in the save game. The
companion writes generated prose as a Markdown projection to
`..\Chronicle\AgesBeyondChronicle.md`.

The companion owns event listener behavior: it classifies incoming DLL events,
filters internal engine events such as barbarian setup diplomacy, calls Ollama
for chronicle-worthy events, and skips duplicate Markdown projections by saved
event id.

Example:

```cmd
AgesBeyondCompanion.exe --pipe \\.\pipe\AgesBeyond-12345 --chronicle ..\Chronicle\AgesBeyondChronicle.md --model llama3.1
```
