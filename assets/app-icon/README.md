# DataCraft app icon

A **database cylinder whose front is a datasheet**, in the Crafting Apps office-icon
grammar. WordCraft uses a document and a pen, GridCraft a ledger page, DeckCraft a
presentation board — DataCraft uses the database stack and the datasheet grid. No
mascot, no letter mark.

The icon is original, hand-written SVG. It is not a placeholder for a creature portrait:
this project deliberately follows the **office-symbol** branch of the Crafting Apps
(WordCraft, GridCraft, DeckCraft), not the engraved-creature branch. See
[`docs/brand/DATACRAFT-ICON.md`](../../docs/brand/DATACRAFT-ICON.md).

## Palette (exactly three colours)

| Colour | Hex | Used for |
|---|---|---|
| Ink | `#0B0B0C` | contour, grid, selected cell |
| Paper | `#EFE9DC` | the cylinder and the datasheet |
| DataCraft teal (app colour) | `#0F8B7E` | the full-bleed field and the datasheet header |

The teal is distinct from the other office-family colours (WordCraft `#3B5BDB`,
GridCraft `#1F9D55`, DeckCraft `#F26B1D`, CADCraft `#14A3C7`) and from PdfCraft
`#B70228`.

## Geometry

- `viewBox="0 0 512 512"`, rounded-square tile `rx=112`, clipped, full bleed.
- The figure sits in a `rotate(-6 256 256)` group, like WordCraft and GridCraft.
- Contours `stroke-width="14"` with round joins and caps; grid `8`; panel border `12`.
- Cylinder body `x 118…394, y 120…392`, top disk `rx 138 / ry 40`; datasheet panel
  `x 148…364, y 190…348`, teal header, four record rows, one ink selected cell.
- macOS render: the tile on Apple's 824/1024 grid, transparent margin.
- Windows and Linux: the full-bleed tile edge to edge.

## Provenance

Original geometry, hand-written for this repository. The three-colour palette, the
paper-and-ink object, the −6° rotation and the tile geometry follow the public
Crafting Apps office icons (WordCraft, GridCraft, DeckCraft). No ArtCraft icon, wordmark
or vendor database logo was copied, traced or recoloured. Licence: same as the repository
(MIT OR Apache-2.0).

## Files

| File | Use |
|---|---|
| `datacraft.svg` | canonical artwork; source for every render |
| `datacraft-1024.png` | full tile, 1024 px |
| `datacraft-512.png` … `datacraft-16.png` | full tile, native renders |
| `datacraft-macos-512.png` | tile on the 824/1024 Apple grid, transparent margin |
| `datacraft.ico` | Windows icon, 16–256 px |

The Linux `hicolor/` tree, `datacraft-small.svg` and `datacraft.icns` arrive with the
app's packaging step; the master is plain SVG, so the suite's `packaging/icons.sh`
(`resvg` + `cargo xtask ico`) can produce them unchanged.

## Regenerating the renders

The current renders were made with a headless browser at native sizes:

```text
msedge --headless=new --screenshot=datacraft-1024.png --window-size=1024,1024 \
       --default-background-color=00000000 file:///…/datacraft.svg
```

Then pack the `.ico` with `png2ico`, Pillow or the suite's `cargo xtask ico`. Keep the
three colours intact and the tile edge clean.
