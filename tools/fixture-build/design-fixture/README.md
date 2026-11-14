# design-fixture — six-kind form-design fixture (78.3, M88)

Builds `fixtures/formDesignsV2016.accdb`, the ground truth for the M88
import mapping (`docs/M88-IMPORT.md`). The fixture contains:

- `All Controls` — Label, TextBox, CheckBox, ComboBox, CommandButton and
  SubForm with explicit twips rects, captions and control sources.
- `Defaults` — one control of each kind created with default geometry, to
  pin how Access represents omitted `Left`/`Top`/`Width`/`Height`.
- `Category Details` — the empty subform source.

Prerequisites: Windows with Microsoft Access (the committed fixture was
built with `16.0.20527.20112`). The tool is never part of CI, never opens
the committed fixture and writes only inside the scratch directory.

```text
powershell -NoProfile -ExecutionPolicy Bypass -File generate-form-designs.ps1 \
  [-OutDir <scratch>] [-Install]
```

`-Install` copies the result to `fixtures/formDesignsV2016.accdb` and
refuses to overwrite an existing file. The script prints the sha256; record
it in `fixtures/manifest` and the provenance in `docs/FIXTURE-SOURCES.md`
("Form-design fixture"). `export/` holds UTF-8 conversions of the
SaveAsText exports (Access writes UTF-16LE at build time); they are
evidence, not build inputs.

The committed fixture is never reopened in Access after it is pinned.
Regeneration runs the generator into a new scratch directory and compares
the object inventory before replacing the pin.
