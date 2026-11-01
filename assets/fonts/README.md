# Bundled fonts

Two openly licensed font families are embedded in the desktop UI via
`include_bytes!` (step 20.2, item U2). The files are unmodified: no
subsetting, no renaming of the internal font names, no re-encoding.

| File | Family | Version | Source | License |
| --- | --- | --- | --- | --- |
| `Inter-Regular.ttf` | Inter Regular | 4.1 | <https://github.com/rsms/inter/releases/tag/v4.1> | SIL OFL 1.1 — [`LICENSE-Inter.txt`](LICENSE-Inter.txt) |
| `Inter-Medium.ttf` | Inter Medium | 4.1 | <https://github.com/rsms/inter/releases/tag/v4.1> | SIL OFL 1.1 — [`LICENSE-Inter.txt`](LICENSE-Inter.txt) |
| `Inter-SemiBold.ttf` | Inter SemiBold | 4.1 | <https://github.com/rsms/inter/releases/tag/v4.1> | SIL OFL 1.1 — [`LICENSE-Inter.txt`](LICENSE-Inter.txt) |
| `JetBrainsMono-Regular.ttf` | JetBrains Mono Regular | 2.304 | <https://github.com/JetBrains/JetBrainsMono/releases/tag/v2.304> | SIL OFL 1.1 — [`LICENSE-JetBrainsMono.txt`](LICENSE-JetBrainsMono.txt) |

Use: Inter is the proportional UI face (egui's default faces stay behind it
as fallback), the Medium and SemiBold faces back the named families
`medium`/`semibold`, and JetBrains Mono renders the SQL editor.
