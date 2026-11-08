# DataCraft app icon — symbol system

> **Status:** final icon in [`assets/app-icon/`](../../assets/app-icon/), hand-built SVG.
> Direction: **office symbol, not a creature portrait** — following the current
> WordCraft, GridCraft and DeckCraft icons. (Upstream those are still placeholders
> awaiting creature portraits; this project deliberately adopts the office-symbol
> branch as its final choice.) No image generation is required.

---

## 1. Why an office symbol

The Crafting Apps split into two visual families:

| Family | Examples | Icon language |
|---|---|---|
| Creative tools | PhotoCraft, VectorCraft, FilmCraft, LightCraft, PdfCraft, EffectCraft, DesignCraft, FigCraft, SoundCraft | engraved creature portrait on a full-bleed colour |
| Office tools | **WordCraft, GridCraft, DeckCraft** | the literal office object on a full-bleed colour |

DataCraft is the Access-style database app, so it belongs to the office family and gets
the literal object: a **database cylinder whose front is a datasheet**. WordCraft uses a
document and a pen, GridCraft a ledger page, DeckCraft a presentation board — DataCraft
uses the database stack and the datasheet grid. No mascot, no animal, no letter mark.

---

## 2. The icon grammar

Taken from the hand-written WordCraft / GridCraft / DeckCraft SVGs
(`github.com/storytold/{wordcraft,gridcraft,deckcraft}`, `assets/app-icon/`):

| Rule | Value |
|---|---|
| Canvas | `viewBox="0 0 512 512"` |
| Tile | rounded square, `rx=112`, clipped, **full bleed** |
| Colours | **exactly three**: Ink `#0B0B0C`, Paper `#EFE9DC`, DataCraft teal `#0F8B7E` |
| Figure | Paper fill, Ink contour, **thin grid lines**; the app colour appears *inside* the figure (GridCraft's header cells, DeckCraft's bars) |
| Contour | `stroke-width="14"`, `stroke-linejoin="round"`, `stroke-linecap="round"` |
| Inner detail | grid/line work `stroke-width="8"`; panel border `12` |
| Placement | the object is rotated **−6°** around the tile centre and fills the tile |
| Frame | **none** — no border, roundel or badge |
| Text | **none** — no letters, numbers, wordmark or logo |
| macOS | separate render on Apple's 824/1024 grid (transparent margin) |
| Windows/Linux | full-bleed tile edge to edge |

---

## 3. Construction

The figure is a database cylinder with a datasheet wrapped onto its front:

```
rotate(-6 256 256)

cylinder body      x 118…394, y 120…392, bottom arc ry 44, stroke 14
top disk           cx 256, cy 120, rx 138, ry 40, stroke 14
inner rim          rx 104, ry 26, stroke 8

datasheet panel    x 148…364, y 190…348, border 12
  header row       y 190…220, full width, filled DataCraft teal
  grid             columns x 220 / 292; rows y 220 / 252 / 284 / 316; stroke 8
  selected cell    x 220…292, y 252…284, filled ink
```

The panel leaves a paper margin on all four sides of the cylinder (30 units at the
sides, 30 above, 40 below), so the cylinder still reads as a solid object rather than a
label. The header is the app colour, exactly like GridCraft's header row; the selected
cell is ink, exactly like a highlighted datasheet cell.

---

## 4. Palette

| Colour | Hex | Use |
|---|---|---|
| Ink | `#0B0B0C` | contour, grid, selected cell |
| Paper | `#EFE9DC` | cylinder, datasheet |
| DataCraft teal | `#0F8B7E` | tile field and datasheet header |

The teal is distinct from every other Crafting App colour in the office family
(WordCraft `#3B5BDB`, GridCraft `#1F9D55`, DeckCraft `#F26B1D`, CADCraft `#14A3C7`) and
from PdfCraft `#B70228`. It reads as data/storage without drifting into CADCraft cyan or
GridCraft green.

---

## 5. Files

| File | Use |
|---|---|
| `datacraft.svg` | canonical artwork; source for every render |
| `datacraft-1024.png` | full tile, 1024 px |
| `datacraft-512.png` … `datacraft-16.png` | full tile, native renders |
| `datacraft-macos-512.png` | tile on the 824/1024 Apple grid, transparent margin |
| `datacraft.ico` | Windows icon, 16–256 px |

The Linux `hicolor/` tree, `datacraft-small.svg` and `datacraft.icns` are produced when
the app gets its bundle/packaging step (none exists yet in Phase 1). The master is plain
SVG, so any of the suite's `packaging/icons.sh` pipelines can generate them.

---

## 6. Acceptance checklist

- [x] Exactly three colours: `#0B0B0C`, `#EFE9DC`, `#0F8B7E`.
- [x] Full-bleed teal field; no frame, border, roundel or badge.
- [x] Tile geometry `viewBox="0 0 512 512"`, `rx=112`.
- [x] Object rotated −6°, fills the tile.
- [x] No text, letters, numbers, logo or watermark.
- [x] Database cylinder + datasheet reads at 32 px.
- [x] Header in the app colour; selected cell in ink.
- [x] macOS render on the 824/1024 grid; Windows/Linux full bleed.
- [x] Original geometry — no vendor database logo and no third-party icon copied.

---

## 7. Legal note

The Crafting Apps' names, wordmarks and icons are their authors'
trademarks/artwork and are **not** open source. This icon borrows the
*visual grammar* of the office apps (three colours, paper-and-ink symbol,
tile geometry) for an original DataCraft drawing. It contains no
third-party mark, no vendor logo and no copied artwork.
