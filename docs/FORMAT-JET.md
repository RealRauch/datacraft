# FORMAT-JET.md — Jet/ACE binary format facts (clean-room knowledge base)

> The authoritative record of every format fact DataCraft's decoders rely on.
> **Sources are public documentation only** — no MDB Tools source code, no
> decompiled Microsoft material (`docs/ARCHITECTURE.md` §12). Facts are
> verified against real files by the CI oracle (step 2.4, item H7).
> Sources: the community format reference at `jabakobob.net/mdb/` (pages
> "First Page", "Table Definition Pages", "Data Pages") and the public format
> documentation distributed with MDB Tools (`HACKING` — documentation file,
> not code; section "Text Data Type" for compression).

## File header (page 0)

| Field | Offset | Size | Value / meaning |
|---|---|---|---|
| Magic | 0x00 | 4 | `00 01 00 00` (u32 LE = 0x100) |
| Product string | 0x04 | 16 | NUL-terminated `"Standard Jet DB"` (`.mdb`) / `"Standard ACE DB"` (`.accdb`) |
| Version | 0x14 | 4 | u32 LE: `0x00000000` Jet3 (A97), `0x00000001` Jet4 (A2000–2003), `0x00000002` ACE 2007, `0x00000103` ACE 2010 |
| Encrypted region | 0x18 | 126 (Jet3) / 128 (Jet4) | RC4, key `0x6b39dac7` — contains collation, code page, creation date. **Not decoded by DataCraft yet** (password detection planned). |

- **Page size is not stored in the file.** It follows from the generation:
  Jet3 = 2048 bytes, Jet4/ACE = 4096 bytes.
- File size must be an exact multiple of the page size.

## Pages

- Pages are 0-indexed regions of `page_size` bytes; page N at offset `N * page_size`.
- The **first byte of every page except page 0** is the page type:
  `0x01` data, `0x02` table definition (TDEF), `0x03` intermediate index,
  `0x04` leaf index, `0x05` usage bitmaps, `0x08` unknown.
- `MSysObjects` (the catalog) has its TDEF always at **page 2** (catalog
  enumeration lands in step 2.4).

## Data pages (type 0x01)

| Field | Offset | Size | Meaning |
|---|---|---|---|
| page type | 0x00 | 1 | `0x01` |
| unknown | 0x01 | 1 | — |
| free space | 0x02 | 2 | u16 LE |
| owner (tdef page) | 0x04 | 4 | u32 LE; `"LVAL"` bytes for long-value pages |
| unknown | 0x08 | 4 | Jet4 only |
| record count | 0x0C | 2 | u16 LE |
| record offsets | 0x0E | 2 × count | u16 each; records stored back-to-front (first record ends at page end) |

Offset flags in the high bits: `0x8000` = deleted row (skip), `0x4000` =
overflow pointer (row lives elsewhere) — **DataCraft rejects overflow rows
with `Unsupported`** for now (rare; item to be scheduled before large-DB work).

## Rows (Jet4)

```
[num_cols u16][fixed data][var data][eod u16][var_table u16 × n][var_len u16][null mask]
```

- `num_cols` = columns stored in this row (≤ TDEF column count; schema
  evolution means rows may store fewer — rows carry a **prefix of the
  column-id space**, columns added later report NULL).
- Fixed columns first, in TDEF order; `offset_F` is relative to the first
  byte after `num_cols`.
- Variable columns stored first-to-last (non-null only); `eod` = offset after
  the last variable field; `var_table` holds start offsets **in reverse
  order** (entry 0 = last variable column); `var_len` = variable columns in
  this row. An empty variable slot (start equals the next start or `eod`) is
  NULL for long-value columns, empty string for text.
- Null mask: `(num_cols + 7) / 8` bytes, LSB first — but **bit k belongs to
  the column with `col_num` k, not to the k-th descriptor**. TDEF descriptors
  can be stored name-sorted (MSysObjects stores them alphabetically) while
  `col_num` follows creation order, so the two orders diverge.
  `0` = NULL for all types **except boolean**, whose value **is** the mask bit.
  Verified against both pinned writers of the oracle fixture (real Access
  rewrite + pristine jackcess file): the mask agrees with the variable-slot
  layout on every catalog row of both variants; zero contradictions.
- Boolean columns occupy no fixed-storage space (value lives in the mask).

## Column types (TDEF byte)

`0x01` Bool, `0x02` Byte, `0x03` Int16, `0x04` LongInt, `0x05` Currency
(scaled i64, 10⁻⁴), `0x06` Float32, `0x07` Float64, `0x08` DateTime (OLE
automation f64), `0x09` Binary, `0x0A` Text, `0x0B` OLE, `0x0C` Memo,
`0x0F` GUID, `0x10` Numeric (96-bit scaled decimal in 17 bytes), `0x12` Complex.

Column flags (bitmask byte): `0x01` fixed length, `0x02` nullable,
`0x04` autonumber, `0x10` replication/hidden, `0x40` auto GUID, `0x80` hyperlink.
Jet4 `misc_flags` bit `0x01` = **compressed unicode** text (see below).

## Text compression (Jet4)

- Plain strings are UCS-2LE.
- Compressed strings start with the marker `FF FE`, then begin in **compressed
  mode**: one byte per character (implicit high byte 0x00). A `0x00` byte
  switches to **uncompressed mode** (UCS-2LE pairs); a pair whose high byte is
  `0x00` yields the low byte as the character (equivalent to compressed mode,
  so the two readings coincide). Strings may end in either mode. Access only
  uses the marker when compression actually shortens the string; strings
  containing characters `0x##00` are stored plain.

## Memo / OLE (long values)

Variable-column content of a memo/OLE field is a 12-byte header
(4-byte form for inline):

| Flag (top bits of first u32) | Storage |
|---|---|
| `0x80000000` | data **inline**, right after the length dword (length = value & `0x3FFFFFFF`) |
| `0x40000000` | **single** LVAL record at the record pointer (next u32) |
| `0x00000000` | **chain** of LVAL records starting at the record pointer |

- Record pointer = `(page << 8) | row`.
- LVAL pages are data pages owned by `"LVAL"`; type-1 records hold raw data;
  type-2 records are `[next pointer u32][data]`, `next = 0` ends the chain.

## Table definition pages (type 0x02)

8-byte page header (`0x02`, `0x01`, free-space u16, **next TDEF page u32 at
offset 4**), then the TDEF block (multi-page TDEFs concatenate via the next
pointer). Jet4 block:

```
tdef_len u32 · unknown u32 · num_rows u32 · autonumber u32 ·
autonum_flag u8 (0x01) + 3 unknown · complex-autonumber u32 · unknown 8 ·
table_type u8 (0x4E user, 0x53 system) · max_cols u16 · num_var_cols u16 ·
num_cols u16 · num_idx u32 · num_real_idx u32 · used_pages u32 · free_pages u32
```

then `num_real_idx` × 12 bytes, then `num_cols` × 25-byte column descriptors
(`type u8, unknown u32, col_num u16, var_index u16, col_num u16, misc u16,
misc_ext u16, flags u8, misc_flags u8, unknown u32, offset_F u16, len u16`),
then per column a u16 **byte** length + UCS-2LE name (the length field counts
bytes, not characters — verified against real files), then the index blocks
(52 bytes per real index, 28 per logical index) and their UCS-2 names (byte
lengths as well), then per-variable column usage-map pointers terminated by
`0xFFFF` (real files sometimes end the block without the terminator — treat
running out of bytes as the end). Descriptor storage order need not match the
`col_num` order — MSysObjects stores its descriptors alphabetically; row null
masks follow `col_num` (see "Rows"). The **logical column order** of a table
is the `col_num` (creation) order, not the descriptor storage order —
verified via the CI mdb-export comparison (mdb-export lists MSysObjects
columns as Id, ParentId, Name, Type, DateCreate, DateUpdate, Owner, Flags,
Database, Connect, ForeignName, RmtInfoShort, RmtInfoLong, Lv, LvProp,
LvModule, LvExtra = col_num 0–16); decoded columns are therefore presented
sorted by `col_num`.

## Real-row tail semantics (RESOLVED 2026-11-01 — BACKLOG T2)

The documented tail order `[eod][var_table reverse][var_len][null mask]` and
the reverse var_table were correct all along; the real discrepancy was the
null-mask **bit index**. Bit k belongs to the column with `col_num` k, not to
the k-th TDEF descriptor — and MSysObjects stores its descriptors
alphabetically (Connect, Database, …, Type) while `col_num` follows creation
order (Id=0, ParentId=1, Name=2, Type=3, DateCreate=4, DateUpdate=5, Owner=6,
Flags=7, Database=8, …). Under the descriptor-position reading every catalog
row's mask looks wrong (`ff 00 00` for container rows) and `Name`/`Type`
decode as NULL — which had stalled catalog enumeration.

Evidence (hex study on both fixture variants, hashes per
`docs/FIXTURE-SOURCES.md` §3):

- Container rows (Tables/Databases/…, `Id` 1–3) end `… 0b 00 ff 00 00` =
  var_len 11 + mask bits 0–7 set — exactly the eight columns with values
  (Id, ParentId, Name, Type, DateCreate, DateUpdate, Owner, Flags = col ids
  0–7). The two-byte variable value after the name is `Owner` (Binary, col id 6).
- Table rows (MSysNavPaneObjectIDs etc.) read `ff 40 00` — bit 14 = LvProp,
  precisely the one variable column carrying a 64-byte payload.
- Variable-slot layout vs. mask: zero contradictions on all 28 Access-touched
  and all 22 pristine-jackcess catalog rows (fixed-column bits consistent
  given that all-zero bytes are legitimate non-null values, e.g. `Flags = 0`).

## Strategy notes (DataCraft decisions)

- **Data-page discovery:** scan pages for type `0x01` with owner == TDEF page
  (documented as sufficient for all but the largest databases). Usage-map
  decoding (types `0x00` inline / `0x01` indirection) is a later optimization,
  not needed for reads.
- **Catalog enumeration (MSysObjects)** lands in step 2.4 together with the
  oracle, which verifies these facts against real files on ubuntu/macOS CI.
- DataCraft reads Jet4/ACE; Jet3 is detected and rejected (BACKLOG L1).
