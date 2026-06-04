# Ages Beyond Companion

`AgesBeyondCompanion.exe` is built from `crates/companion` and is launched by
the Civ IV DLL over a Windows named pipe.

The DLL searches for the executable next to `CvGameCoreDLL.dll`, in
`..\Companion\AgesBeyondCompanion.exe`, and in `..\AgesBeyondCompanion.exe`.

The first provider is Ollama. The companion assumes Ollama is already running at
`http://localhost:11434` unless a different `--ollama-url` is supplied.

Example:

```cmd
AgesBeyondCompanion.exe --pipe \\.\pipe\AgesBeyond-12345 --model llama3.1
```
