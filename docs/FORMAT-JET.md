# FORMAT-JET.md — Jet/ACE binary format facts (clean-room knowledge base)

> The authoritative record of every format fact DataCraft's decoders rely on.
> Compatibility claims are bounded by the README **Compatibility matrix**;
> this file records facts, evidence and known remainders, never a blanket
> support statement. **Sources are public documentation only** — no MDB Tools
> source code, no decompiled Microsoft material (`docs/ARCHITECTURE.md` §12).
> Facts are verified against real files by the CI oracle (step 2.4, item H7).
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
| Encrypted region | 0x18 | 126 (Jet3) / 128 (Jet4/ACE) | RC4, key `C7 DA 39 6B` (LE `0x6b39dac7`) — contains code page, database key, password field, creation date. Decoded/validated since M5 (see "Encryption and passwords"). |

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
`0x11` is not covered by HACKING's (incomplete) type table; both fixture
variants use it for `MSysAccessObjects.Data` (a long-value column). DataCraft
answers a precise `Unsupported`; mdb-tools itself warns "unsupported data
type 17" on the same column (BACKLOG L2).

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

## Domain mapping (2026-11-01 — BACKLOG H12/H16 decisions)

- **Plain GUID columns (Jet type `0x0F`)** map to a dedicated
  `FieldKind::Guid` (accepts only 16-byte GUID values). AutoNumbers with
  GUIDs stay `AutoNumber(ReplicationId)`. Rationale: mirrors
  `StorageDataType::Guid` 1:1; the Access UI surfaces them under
  "Number > Replication ID", but the domain kind is cleaner for validation.
- **System-table filter:** user-table enumeration excludes names with the
  `MSys` prefix. Observed on the fixture (both writers): system-table
  `Flags` values vary (`0x80000000`, `0x2`, `0xa`) — no single flag bit is
  writer-stable; the reserved name prefix is.
- **Text length semantics:** the TDEF `len` field counts bytes for
  uncompressed (UTF-16) text — the Access character budget is `len / 2`;
  compressed-text columns are byte-per-character (`misc_flags` bit `0x01`).
- **Jet `Numeric` (decimal) columns** carry only the scale in the `misc`
  byte; precision is modeled as the 96-bit maximum (28).
- **Multi-page TDEFs:** Access chains table definitions via the
  next-TDEF-page pointer at header offset 4; the decoder concatenates the
  post-header payloads and keeps `tdef_len` (self-inclusive) as the parse
  bound. Chain cycles and non-TDEF chain members are precise `Corrupt`
  errors. (Found via the Access-rewritten fixture variant — Table2.)
- **Writes** run through the clean-room writer (BACKLOG H9, Phase 10):
  create tables, insert, update and delete rows on Jet4/ACE with
  usage-map-consistent allocation — see "Usage maps (writer)" and
  "Row update and delete (writer)" below. ACE write-through is verified
  against the pinned 2007/2010 fixtures (Phase 25: byte-preserving
  contracts in `ace_writer.rs`, port/wire/CLI write-through in
  `ace_port_writes.rs` / `server_e2e_ace.rs`; office-encrypted ACCDB
  answers `Unsupported` at the reader and writer boundary).

## Usage maps (writer — verified 2026-11-01, step 10.1)

Black-box verified against the Access-written fixture by decoding the TDEF
`used_pages`/`free_pages` dwords of every table and cross-checking the bits
against the data-page scan (`owner == tdef`): **zero contradictions on all
13 real tables** (jet_usage_map.rs `fixture_usage_maps_agree_with_the_page_scan`).

- The TDEF dwords pack **`[map type in the HIGH byte][u24 LE record pointer
  = (page << 8) | row]`** — e.g. MSysObjects: `used = 0x00000600` (page 6,
  row 0), `free = 0x00000601` (page 6, row 1). HACKING's byte-wise reading
  ("byte 0 = map type") describes Jet3 byte order; in Jet4 files the type
  sits in the highest byte of the dword.
- **All maps in the file are type `0x00`**: the map lives as a **row in a
  data page** — there is not a single type-`0x05` usage page in the file
  (HACKING's "indirection to 0x05 pages" describes the type-1 form, which
  Access did not use here; DataCraft answers a precise `Unsupported`).
- A type-0 map row is **`[0x00][start_page u32][bitmap]`**, LSB first, bit
  set = page **allocated to the table**. Rows grow with the table's window:
  observed 64- and 96-byte bitmaps (512/768-page windows) and a non-zero
  `start_page` (752, for a table whose pages began beyond the first window).
- The **global map is page 1, row 0** with the opposite polarity: bit set =
  page **free or absent**; Access pre-marks headroom beyond EOF (fixture:
  bits 765–767 of a 765-page file in a 96-byte/768-page window). When a new
  page is appended its bit is cleared there.
- Usage-map rows live in data pages **owned by page `0`** (page 1 itself is
  owned by the phony TDEF `1`). The `free_space` field of usage pages
  follows the data-page formula `lowest_row_start − (14 + 2·row_count)` —
  verified exactly on pages 1, 6, 17, 25.
- Rewriting a grown map row in place keeps every `(page, row)` reference
  valid: rows keep their slot indices; only offsets and `free_space` change.
- **Freed TDEF pages** are re-filled with the pattern `ff 00 ff ff …` and
  keep their page-type byte `0x02` (fixture pages 322, 707, 708 — same
  signature; 708 additionally serves as page 30's chain tail). A real TDEF
  block is recognized by its fixed header: `table_type` ∈ {`0x4E`, `0x53`}
  at block offset 32, `num_cols ≤ max_cols`, `tdef_len ≥ 55`. The writer
  gates allocation on this plausibility check.
- Writer allocation policy (step 10.1): **append-only** at EOF — interior
  free-page reclaim is not needed for the write milestone (the fixture
  carries 27 zeroed pages that the global map marks allocated; they are
  preserved untouched).

## Catalog insert recipe (writer — verified 2026-11-01, step 10.2)

Decoded from real table rows of both fixture writers (jackcess `Table1`,
Access `Table2`–`Table4`) — identical shape:

- **Id** = the table's TDEF page number (both writers agree).
- **ParentId** = `0x0F000001` (the "Tables" container row's Id; container
  rows themselves use the magic Ids `0x0F000001`–`3`).
- **Type** = 1 (table), **Flags** = 0 (user tables), **Owner** = the 2-byte
  binary `E5 A3` (user-created; system rows carry `E4 A1`).
- **DateCreate/DateUpdate** = OLE automation date, identical values.
- **LvProp** = NULL is accepted on catalog rows (system rows carry NULL);
  real writers store an LVAL-referenced property pack (single LVAL record,
  935–2194 bytes, `MR2\0` header per HACKING). DataCraft writes NULL and
  documents the divergence until the LVAL writer lands.
- A new table's **usage page** (data page, owner 0) carries two rows:
  used map (row 0) and free map (row 1), both initially empty.
- **Free-map polarity**: bit set = page has room for another row. Observed
  bracket on the fixture: a page with 75 free bytes is unmarked, pages with
  966+ bytes are marked. The writer marks conservatively at ≥ 512 bytes.
- **Data-page offset table order**: sorted by **descending offset** — the
  physically lowest row is the offset table's LAST entry; appends extend
  the table and place the row between the offset table and the lowest row.

## Writer round-trip facts (verified 2026-11-01, step 10.3)

Verified by `decode(encode(decode(x))) == decode(x)` over **155 rows across
12 tables** of the Access-written fixture (`jet_writer_roundtrip.rs`):

- `var_len` counts **all** variable columns of the schema; NULL variable
  columns occupy an empty slot (the null mask distinguishes NULL from
  empty-string). Writing fewer slots breaks the var_index addressing.
- **Scalar types can be stored in variable slots** — the fixture's nav-pane
  tables store `LongInt` (and other scalars) with the FIXED flag cleared.
  The encoder emits identical bytes in both slot kinds.
- Fixed `Binary` columns store exactly their descriptor length (padded
  values round-trip byte-exact).
- Writer divergences from real writers (documented, functionally
  transparent to readers): memo/OLE values are written in the **inline**
  long-value form instead of LVAL pages; new catalog rows carry **NULL
  LvProp** instead of a property pack; text is stored uncompressed unless
  the column demands the compressed form.
- **Manual gate item (cannot run in CI):** "Access opens the written file
  without repair prompts" requires a real Access check on the golden files
  — executable proxies in CI: our decoder + `mdb-export` both read the
  written files (the oracle test runs on ubuntu/macOS CI).

## Port write-through (2026-11-01, step 11.1)

`Jet4Port` exposes the writer behind `StoragePort`: `insert_row` and
`create_table` run a read-modify-write on a private copy of the file bytes,
persist the result to the same path and swap in a fresh reader — a failed
operation leaves both file and in-memory state untouched. The domain -> Jet
column mapping (with its lossiness) is documented in `src/port_write.rs`:

- text `max_len` becomes a byte budget of `2 * max_len` (UTF-16, variable
  slot); memo/OLE use the inline long-value form; numeric widths, decimal
  scale (17-byte `Numeric`), date, currency, GUID and boolean map 1:1.
- **AutoNumber (LongInteger):** the TDEF head `+12` holds the table's next
  value; the column carries the `0x04` AUTONUMBER flag and is written as a
  fixed 4-byte `LongInt`. `Null` on insert means engine-assigned: the writer
  fills the counter value and advances `+12` (persisted across reopen); a
  user-supplied value is rejected. GUID AutoNumbers (`ReplicationId`) stay a
  precise `Unsupported` (step 24.1, M8).
- **Table drop (M7, step 24.2):** the `MSysObjects` row is deleted (the
  `0x8000` slot flag, `num_rows` decremented), every page from the table's
  used map plus its TDEF chain returns to the global free map, the usage
  page is freed when no other table references it, and all freed pages are
  overwritten with the `0xFF` pattern Access leaves behind. Failures leave
  the file untouched.
- **Design-time alter (M9, step 24.3):** the TDEF block is re-encoded in
  place (the change must fit the page). Appending a field adds a `Null` slot
  to every row; dropping a field removes the value and renumbers the
  remaining column ids densely (rows store a prefix of the column-id
  space); renaming and toggling `required` touch only the TDEF. Every change
  runs on a scratch writer first, so a rejected change leaves the file
  untouched. Storage-shape changes (type/size), required additions with
  existing rows and non-TDEF properties answer a precise `Unsupported`.
- **`Unsupported` today:** complex-value columns (attachments, multi-value,
  version history — read-only since M4).
- `update_row`/`delete_row` address rows by their zero-based live-row
  index — the same order `read_rows` returns (next section).

Verified end-to-end on the pinned fixture: an inserted row (text, byte,
integer, long, double, date, currency, boolean) reads back through our
decoder after reopen **and** through `mdb-export` (mdbtools 1.0.1).

## Row update and delete (writer — 2026-11-01, step 11.10, BACKLOG H40)

Row indices address the **live-row order** (slots without the deleted
flag, in page/slot order) — the order the reader returns.

- **Delete** sets bit `0x8000` in the row's offset-table entry and
  decrements the TDEF `num_rows`; the bytes stay in place. This is the
  Access convention: space is reclaimed by a later compaction, never by
  the delete, so the page's free-space field and the usage/free maps stay
  valid unchanged. The reader already skips `0x8000` slots (deleted rows
  and overflow targets).
- **Update** takes the in-place path when the encoded row keeps its byte
  length (fixed-width rows and same-length variable values): only the
  slot's bytes change, offsets and free space stay valid.
- A different length **repacks the page** back-to-front with every slot's
  `0x8000`/`0x4000` flags preserved and recomputes the free-space field;
  the table's free-map bit is refreshed afterwards (>= 512 bytes free).
- **`Unsupported` (precise):** rows behind an overflow pointer (`0x4000`
  slot) and rows whose memo/OLE value lives in an **external LVAL chain**
  — rewriting them would orphan pages; and updates whose row does not fit
  the page (overflow into a new page is a later milestone).
- AutoNumber columns are written as-is on update; the command bus
  guarantees the value is unchanged (`Null` keeps the current value).

Verified on private fixture copies: update/delete persist across reopen,
memo/OLE repack round-trips, usage maps decode after the operations, and
`mdb-export` reads the written file where the oracle is installed.

## Jet 3 (Access 97) read path (2026-11-01 — step 10.6, BACKLOG L1)

Source: MDB Tools HACKING (public documentation), black-box verified by the
synthetic-fixture contract suite (`tests/jet3.rs`) and the pinned pristine
Jet3 fixtures `fixtures/testV1997.mdb` / `fixtures/test2V1997.mdb`
(`tests/jet3_remainders.rs`; provenance in `docs/FIXTURE-SOURCES.md`). The
oracle comparison applies where mdb-tools is installed.

- **2048-byte pages**; the header version dword is `0x00000000`.
- **Data pages**: 10-byte header — `[0x01][0x01][free u16][tdef u32]
  [count u16 @8]`, offset table from byte 10. Jet4's extra unknown dword
  is absent; owner and the back-to-front layout (incl. the `0x8000`/
  `0x4000` offset flags) are shared with Jet4.
- **TDEF**: page header carries the word `'VC'` where Jet4 stores free
  space. The block is a **35-byte header** (`tdef_len, num_rows,
  autonumber, table_type @12, max_cols @13, num_var @15, num_cols @17,
  num_idx @19, num_real_idx @23, used_pages @27, free_pages @31`), then
  8 bytes per real index, then **18-byte column descriptors** (`type,
  col_num u16, offset_V u16, col_num u16, sort_order u16, misc u16,
  unknown u16, flags u8, offset_F u16, col_len u16`), then **1-byte-length
  codepage names**. `misc` holds the code page for text columns
  (`0x4E4` = CP1252) and precision/scale otherwise.
- **Chained TDEFs:** the page header's next-page dword links continuation
  pages; their post-header payloads concatenate and the **first** page's
  `tdef_len` is the total block length (continuation pages carry arbitrary
  bytes in that field). Verified on `testV1997.mdb` (page 34 → 44, 89
  columns) and `test2V1997.mdb` (page 29 → 56, 74 columns).
- **Rows**: `[num_cols u8][fixed][var][eod u8][var_table u8 × n,
  reversed][jump_table][var_len u8][null mask]` — all counters/offsets are
  one byte. The null mask is identical to Jet4 (bit k = column id k). The
  fixed area starts at offset **1** (not 2).
- **Jump table:** rows whose data crosses 256-byte boundaries interleave a
  jump table between the var table and `var_len`. Its entries name the
  first variable column of each 256-byte segment (a `0xFF` entry is the
  documented dummy); the entry count follows the row size (`row_len / 256`,
  closure-checked). An offset reconstructs as `raw + 256 × (entries ≤
  var_index)`; the eod uses `entries ≤ var_len`. Verified on
  `test2V1997.mdb` page 50 (290-byte row, dummy entry, eod 255) and the
  synthetic two-segment contract in `tests/jet3.rs` (200 + 100 bytes, eod
  301).
- **Overflow rows:** the `0x4000` pointer is followed with the Jet3 10-byte
  data-page header (the catalog carries such rows); LVAL pages decode with
  the same header.
- **Text**: no unicode; CP1252 (writer-machine dependent) is decoded for
  names and cell text. No compressed form exists.
- **Writer**: unchanged — the writer milestone targets Jet4/ACE only.

## ACE / ACCDB read path (2026-11-01, step 14.2, BACKLOG H39)

Verified against the three pinned pristine fixtures (`fixtures/testV2007.accdb`,
`testV2010.accdb`, `attachmentsV2016.accdb`; hashes and provenance in
`fixtures/manifest` / `docs/FIXTURE-SOURCES.md`).

- **Header:** product string `Standard ACE DB\x00` at 0x04; version dword at
  0x14 is `0x00000002` (ACE 2007/2016 writer format) or `0x00000103` (ACE
  2010). Pages are 4096 bytes; the file size is an exact multiple. The
  product string and version must agree — a `Standard ACE DB` file with a Jet
  version dword (or the reverse) is a precise `Unsupported`, not a guess.
- **Catalog/TDEF/TDAT:** the ACE files decode through the **unchanged Jet4
  4096-byte path**: MSysObjects catalog at TDEF page 2, TDEF blocks, row
  layout, unicode compression, null-mask bit = `col_num`. Observed on all
  three fixtures (user tables `Table1`–`Table4`, plus complex subtables).
- **Complex-value subtables:** ACE stores them as tables named
  `f_<32 hex>_Data` / `f_<32 hex>_Field1` (attachment/multivalue backing
  tables). Since step 16.1 (M4) they are grouped into their owning column and
  hidden from the user catalog; mdb-tools hides them as system tables, so the
  oracle skips them (see "Complex values" below).
- **Oracle cross-check:** `mdb-export` (mdbtools 1.0.1) reads all three
  fixtures. Column names/order and row counts match our decoder on every
  non-`f_` table, and `Table1`'s values match exactly (text, byte/integer/
  long, double, OLE date, currency scaled 10⁻⁴, boolean). The CI oracle job
  runs `tests/oracle.rs` over `testV2000.mdb` plus all three `.accdb` files.
- **Client surface:** `DatabaseFormat::Ace` reaches UI/CLI/MCP through the
  snapshot; the status line renders `(ACE)`.

## Complex values — attachments, multi-value, version history (2026-11-01, step 16.1, item M4)

Verified against `fixtures/complexDataV2007.accdb` (jackcess V2007) and
`fixtures/attachmentsV2016.accdb` (mdb-reader V2016).

- **Mapping:** `MSysComplexColumns` carries one row per complex column:
  `ColumnName`, `ConceptualTableID` (owning table's TDEF page), `FlatTableID`
  (backing table's TDEF page), `ComplexTypeObjectID` (object id of the complex
  type, e.g. `MSysComplexType_Attachment`), `ComplexID`. The complex column's
  TDEF descriptor `misc` field carries the same `ComplexID` (observed:
  `Table1.Attachments` misc=2 ↔ `ComplexID` 2).
- **Cell value:** the complex column is a fixed 4-byte column (flags `0x07`);
  its value is a **row-scoped complex id** shared by all complex columns of
  that row. The backing table's link column is named `_<name>` (e.g. `_Data`,
  `_Field1`, `_multi-value-data`) and carries the same id per sub-record.
  Grouping the flat rows by that id yields the ordered sub-records.
- **Sub-record shape:** the flat table's user columns in flat-table order,
  excluding the `_<name>` link column and the flat table's own AutoNumber
  identity column (e.g. `Table1_Field1`, `Table1_multi-value-data`).
- **Field-kind mapping:** `MSysComplexType_Attachment` →
  `Binary(Attachment)`; a flat table with exactly one user value column →
  `MultiValue(element kind)` (Text/Number/DateTime/Currency/Boolean/Guid,
  derived from the value column); everything else (e.g.
  `MSysComplexTypeVH_*` version history) → `Complex`. A complex column without
  a `MSysComplexColumns` mapping keeps the binary attachment fallback.
- **Catalog:** backing tables are hidden from `tables()` and answer
  `TableNotFound` when addressed directly; the navigation pane lists only the
  owning table. `MSys*` system tables stay excluded as before.
- **Datasheet:** complex cells render as the joined scalar of their
  sub-records (`FileName` for attachments, `Value` for multi-value, the first
  non-binary field otherwise), separated by `", "`.
- **QBE:** criteria on a `MultiValue` column coerce to the element family and
  use **EXISTS semantics** — a row matches when at least one element
  satisfies the predicate (for `<>`: at least one element differs). Criteria
  on attachment/version-history columns stay a precise `Unsupported`.
- **SQL shadow engine:** complex cells materialize as their display text
  (documented lossiness; the in-memory SQLite database has no complex type).
- **Writes:** complex columns are read-only (a precise `Unsupported` on
  insert/update/create); the writer does not build flat tables yet.
- **Oracle:** mdb-tools hides the `f_` tables; `tests/oracle.rs` skips them and
  the complex columns, while the parent table's scalar columns still compare.

## Encryption and passwords (2026-11-01, step 16.2, item M5)

Verified against `fixtures/passwordV2000.mdb` / `passwordV2000-none.mdb`
(Jet4 password `"password"`), `fixtures/passwordV2010.accdb` and
`fixtures/officeAgile42.accdb` (mdb-reader test data), plus generated
page-encrypted copies of the pinned fixtures.

- **Header region:** bytes 0x18..0x96 (Jet3, 126) / 0x18..0x98 (Jet4/ACE,
  128) are RC4-encrypted with the fixed key `C7 DA 39 6B` **in every file**,
  including unprotected ones. The reader decrypts it on open; the writer
  re-encrypts it on every persist (RC4 is symmetric).
- **Database key:** the decrypted dword at 0x3e. Non-zero = every page after
  page 0 is RC4-encrypted with the 4-byte key `db_key XOR page_number`
  (LE). Page 0's body outside the header region stays plaintext.
- **Jet3 password:** 20 bytes at 0x42 as **plain CP1252 text** inside the
  encrypted header region, NUL/space padded. Empty = no password. (The
  documented fixed XOR key `86 FB EC 37 …` is the differential-recovery
  technique — XOR a known-unprotected file with the protected one — not an
  in-file mask.)
- **Jet4 password:** 40 bytes at 0x42 = 20 UTF-16LE characters XOR a 4-byte
  mask. The mask is the integer part of the OLE creation-date double at
  0x72 as 4 LE bytes (`44568.67…` → `18 AE 00 00`). Empty plaintext = no
  password (the stored field is then the repeated mask).
- **ACE 2007+:** Office encryption (RC4 CryptoAPI or AES) replaces the page
  cipher; a non-zero database key, a non-empty password field and/or the
  descriptor length at 0x299 mark it. DataCraft answers a precise
  `Unsupported` and does not attempt a decrypt.
- **Client path:** `StoragePort::open(path, password)`; `OpenDatabase` carries
  an optional `password` (serde-default, so old JSON stays valid). Missing →
  `PasswordRequired`, wrong → `WrongPassword` (`Event::PasswordRequired` /
  `Event::PasswordRejected`); the UI raises its prompt, the CLI accepts
  `-p`/`--password`, MCP agents pass the same command field.
- **Writes:** the port's read-modify-write keeps encrypted files encrypted —
  the writer re-encrypts the header region and every page with the same
  database key; newly appended pages use their page number.

## SQLite backend (2026-11-01, step 13.1, BACKLOG H35)

SQLite is the second source family behind the same `StoragePort`; it is not
a Jet format, so its mapping lives here next to the Jet facts to keep the
domain-mapping decisions in one place.

**Detection.** A file whose first 16 bytes are `SQLite format 3\0` is served
by `SqlitePort`; Jet/ACE headers (`01 00 00 00` at offset 0) by `Jet4Port`.
`DetectingPort` sniffs the magic once at `open` and delegates every port
operation to the chosen backend — every client (GUI, MCP, CLI, E2E) speaks
the same command vocabulary. Anything else fails with a precise
`OpenFailed` naming the path and the expected headers.

**Catalog.** `sqlite_master` (`type = 'table'`, internal `sqlite_*` names
filtered) in `rowid` order — creation order, deterministic per file.
`PRAGMA table_info` supplies name, declared type, `NOT NULL` and the
primary-key position; `PRAGMA foreign_key_list` supplies relationships
(a missing `to` column resolves to the referenced table's primary key).

**Declared type → Access field kind.** Marker types are matched by name
first, then by SQLite's affinity rules:

| SQLite declared type | Domain kind | Notes |
|---|---|---|
| `INTEGER PRIMARY KEY AUTOINCREMENT` | `AutoNumber(LongInteger)` | requires the table DDL to contain `AUTOINCREMENT`; always required |
| `INTEGER`, `BIGINT`, `INT*` | `Number(LongInteger)` | 64-bit values widen into the domain integer |
| `TEXT`, `CHAR`, `CLOB`, `VARCHAR(n)` | `Text { max_len }` | `n` clamped to 1–255; unbounded text maps to the Access maximum 255 |
| `BLOB` or no declared type | `Binary(OleObject)` | raw bytes |
| `REAL`, `FLOAT`, `DOUBLE` | `Number(Double)` | |
| `NUMERIC`, `DECIMAL`, other | `Number(Decimal { 28, 0 })` | SQLite has no decimal storage class; values are read as integers or rounded reals (documented lossiness) |
| `BOOLEAN`, `BOOL` | `Boolean { tri_state: nullable }` | SQLite stores 0/1 as integers; readers map non-zero to Yes |
| `DATE`, `DATETIME`, `TIMESTAMP` | `DateTime` | DataCraft treats the stored number as an OLE Automation day value; foreign text dates surface as text |
| `CURRENCY`, `MONEY` | `Currency` | scaled integer (10⁻⁴), same as Jet |
| `GUID`, `UNIQUEIDENTIFIER` | `Guid` | 16-byte BLOB or canonical 8-4-4-4-12 text |

**Value mapping.** `NULL` → `Null`; integers → `Integer` (or `Boolean`,
`Currency`, `DateTime`, scaled `Decimal` per the column kind); reals →
`Float` (or `DateTime`/scaled `Decimal`); text → `Text` (GUID-shaped text
under a GUID column parses to `Guid`); blobs → `Binary` (16-byte blobs
under a GUID column become `Guid`). Row windows use `LIMIT/OFFSET` over
the table's natural rowid order; a window starting beyond the count is
`OutOfRange`, one starting at the count is empty.

**Writes (step 13.2).** `create_table` emits `CREATE TABLE` with the
inverse of the affinity table: text → `VARCHAR(n)`/`TEXT`, integral numbers →
`INTEGER`, single/double → `REAL`, decimal → `NUMERIC`, date/time →
`DATETIME`, currency → `CURRENCY`, boolean → `BOOLEAN`, binary → `BLOB`,
GUID → `GUID`, `AutoNumber(LongInteger)` → `INTEGER PRIMARY KEY
AUTOINCREMENT` (exactly one per table; `AutoNumber(ReplicationId)` is a
precise `Unsupported`). Required fields become `NOT NULL`. `insert_row`
builds a parameterized `INSERT`; `NULL` on an AutoNumber column omits the
column so SQLite assigns the value, any other value is rejected. Stored
values keep their domain family on read-back; the documented lossiness is
SQLite's own (no width/scale enforcement for `INTEGER`/`NUMERIC`). A file
whose permissions mark it read-only still opens for reading (both platforms:
the metadata check runs before the connection is opened, because Windows
defers the failure to the first write) and answers a precise `Unsupported`
on every write.

## Column type 0x11 — decision record (2026-11-01, step 10.7, BACKLOG L2)

**Decision:** `0x11` maps to `JetType::LongBinary` — a **fixed-storage long
binary** (`StorageDataType::Binary`, domain `Binary(OleObject)`).

**Evidence (fixture hex study):** MSysAccessObjects (TDEF page 16) defines
`Data` as type `0x11`, flags `0x03` (fixed+nullable), `len = 3992` — and the
decoded rows carry the **OLE Compound File signature `D0 CF 11 E0 A1 B1 1A E1`**
inline: each row embeds an Access version-history OLE document. The type is
neither memo/OLE (no LVAL indirection — the payload sits in the fixed area)
nor the HACKING type table's members.

**Consequences:** the table decodes on our side (5 rows on the fixture);
the mdb-tools oracle **cannot** cross-check it ("unsupported data type 17" —
impossible by construction), so `tests/oracle.rs` skips that one table with
this documented reason. HACKING's type table remains incomplete (as already
noted for this byte); no format-doc update is needed beyond this record.

## Memo/OLE values (step 15.3 evidence, `fixtures/queryTestV2000.mdb`)

A row's memo/OLE slot holds a 4-byte header, then either a pointer or
inline data:

| Bits `0xC0000000` | Form |
|---|---|
| `0x80000000` | Inline; `len = header & 0x3FFFFFFF` |
| `0x40000000` | Single LVAL record; next dword = `page << 8 | record` |
| `0x00000000` | LVAL chain; next dword = first `[next][data]` record |

**Inline memos** written by Access 2000+ prepend an **8-byte zero header
that the length does not count** (slot total = `4 + 8 + len`, verified on
nine `MSysQueries` expression values); DataCraft's own writer omits it
(slot total = `4 + len`). The slot length decides which form is present.

**Text decoding** (Jet4): text with the `FF FE` header starts compressed
(one byte per character) and a `0x00` byte switches to UCS-2 pairs for
the remainder — headers announce compression (memos always carry it);
values without the header follow the column's compressed-unicode flag
(flag: single-byte + switch, no flag: UCS-2 throughout).

## MSysQueries — native saved queries (step 15.3, item M3)

`MSysObjects` rows with `Type = 5` name the queries; `Flags & 0xF0` is the
flavor: `0x00` select, `0x10` crosstab, `0x20` delete, `0x30` update,
`0x40` append, `0x50` make-table, `0x60` data-definition, `0x70`
pass-through, `0x80` union.

The definition lives in `MSysQueries` (one row per attribute, grouped by
`ObjectId`, ordered by `Order`):

| `Attribute` | Meaning | Columns |
|---:|---|---|
| 0 | start | — |
| 1 | select-type bits | `Flag` (`0x01` star, `0x02` distinct, `0x08` distinct row, `0x10` top, `0x20` percent) |
| 2 | parameter | `Name1`, `LvExtra` (type code) |
| 3 | flags | `Flag` |
| 4 | remote database | `Expression` |
| 5 | table | `Name1` |
| 6 | column | `Name1` table, `Name2` field/alias, `Expression`, `Flag` |
| 7 | join | `Name1`/`Name2` tables, `Expression`, `Flag` (1 inner, 2 left, 3 right) |
| 8 | WHERE | `Expression` |
| 9 | GROUP BY | `Expression` |
| 10 | HAVING | `Expression` |
| 11 | ORDER BY | `Name1`, `Flag` (descending) |
| 255 | end | — |

Unknown attribute codes and query flavors answer a precise `Unsupported`.
The decoder separates the attributes by role (`criteria` = WHERE,
`group_by`, `having`, typed `joins` with the flag flavor and typed
`order_by` entries with the descending bit); `select_flags` carries
attribute 1 and `parameters` the attribute-2 names.

**Execution subset (step 27.1, M12).** `Select` queries execute through
`datacraft-core::native_exec` against materialized port rows:

- one or more FROM tables with **equality** joins only; attribute-7 flag
  `1` = inner, `2` = left (preserves the left operand), `3` = right
  (preserves the right operand). Outer joins must bind a new table — a
  second outer predicate over already bound tables is refused.
- WHERE expressions use the phase-15 Access expression grammar verbatim
  (`And`/`Or`/`Not`, `In`, `Is [Not] Null`, `Like`, `Between`, brackets,
  double-quoted text); identifiers are whitelisted against the schema and
  ambiguous bare names are refused.
- projection accepts plain field references and `Table.*` / `*`
  wildcards; ORDER BY accepts field references with their descending bit.
- `GROUP BY`/`HAVING`, parameters, select modifiers
  (`DISTINCT`/`TOP`/`PERCENT`), computed columns, self-joins/duplicate FROM
  tables and non-equality joins answer a precise `Unsupported`. A
  deterministic row cap (1,000,000 joined rows) protects crafted
  cartesian joins.

Control-flow flavors (crosstab/delete/update/append/make-table/DDL/
pass-through/union) stay decoded-but-not-executed — they are not selects
and would mutate or restructure the source.

Parameter types (`PARAM_TYPE_MAP`, jackcess): 0 value, 1 boolean, 10 text
(Jet `TEXT`), 2 byte, 3 integer, 4 long, 5 currency, 6 single, 7 double,
8 date/time, 9 binary, 11 OLE, 15 GUID (`DataType` values).

## MSysAccessStorage — named-macro inventory (step 17.1, item L4)

Access 2007+ keeps compiled application objects in a tree table
(`MSysAccessStorage`): each row is a storage (`Type = 1`) or stream
(`Type = 2`), addressed by `Id`/`ParentId`; stream payloads live in `Lv`.
The column order varies between writers, so the decoder looks columns up
by name (`Id`, `ParentId`, `Type`, `Name`, `Lv`).

Named macros hang off the `Scripts` storage (observed on the pinned
`northwind2007.accdb`):

- `Scripts/<n>/Blob` — one definition stream per macro;
- `Scripts/\u{3}DirData` — the directory:
  `header (4 bytes)`, then entries of
  `tag (1 byte, 0x04)`, `length (1 byte)`, `name (length − 4 bytes,
  UTF-16LE, no terminator)`, `index (u32)`; the child storage named
  `<index>` carries the `Blob` stream.

The definition blob is a binary action tree whose readable parts are
length-tagged UTF-16 texts (expressions, SQL statements, form names). The
inventory decoder emits those texts in storage order and strips the
one-byte record tag that precedes some of them (`7=MsgBox(…)`,
`8Delete * from …`). Action-name tables and execution semantics are
**not** decoded — DataCraft does not execute macros (ADR-2,
`docs/ROADMAP.md` §3). The catalog names come from `MSysObjects`
`Type = -32766` (same value in Jet 3 and ACE).

Oracle: `fixtures/northwind2007.accdb` decodes `AutoExec` (5 arguments,
including `Not [CurrentProject].[IsTrusted]` and the two form names) and
`Delete All Data` (the `MsgBox` expression followed by six
`Delete * from …` statements). Jet 3 keeps macro definitions inside
`MSysAccessObjects`; that compound-file chunking is not decoded yet, so
Jet 3 names are reported with `definition_decoded = false`.

## VBA project extraction (step 17.2, item L5)

The ACE VBA project lives under `MSysAccessStorage`:
`VBA/VBAProject/VBA` (storages) holds `dir`, `_VBA_PROJECT`, `PROJECT`,
`PROJECTwm` and one stream per module. The `dir` stream names the modules,
so the stream names (random-looking identifiers) are resolved through it.

**Compression ([MS-OVBA] §2.4.1).** `dir` and the source part of each
module stream are `CompressedContainer`s: signature `0x01`, then chunks
whose 2-byte header carries the chunk size minus three (bits 0–11), the
`0b011` signature (bits 12–14) and the compressed flag (bit 15). A
compressed chunk is a sequence of flag bytes (least-significant bit =
first token) followed by up to eight tokens; a set bit is a copy token.
The copy offset/length bit split is dynamic:
`BitCount = max(ceil(log2(position_in_chunk)), 4)`,
`Length = (token & (0xFFFF >> BitCount)) + 3`,
`Offset = ((token & !(0xFFFF >> BitCount)) >> (16 - BitCount)) + 1`;
copies may overlap (run-length). Raw chunks copy their bytes verbatim.

**`dir` stream ([MS-OVBA] §2.3.4.2).** Decompressed, it is a flat
`id (2) size (4) payload` record sequence. `PROJECTVERSION` (`0x0009`) is
the exception: a fixed 12-byte record without a size field. Used records:
`0x0003` code page (1252/1200/65001 decoded), `0x000F` modules start,
`0x0019` module name, `0x001A` module stream name, `0x0031` module offset,
`0x0021` procedural / `0x0022` non-procedural type, `0x002B` terminator.
Unknown records are skipped.

**Module stream ([MS-OVBA] §2.3.4.3).** `[PerformanceCache]\
[CompressedContainer]` — the performance cache is `MODULEOFFSET` bytes
and ignored on read; the remainder decompresses to VBA source in the
project code page. Kind mapping: procedural → standard; non-procedural
`Form_*` → form, `Report_*` → report, everything else → class.

**Scope.** P-code (`_VBA_PROJECT` performance cache) is not decoded and
nothing is executed (ADR-2). Jet 3 keeps the project in
`MSysAccessObjects` (compound-file chunking), which is not decoded yet —
Jet 3 databases report no project (BACKLOG remainder).

Oracle: `fixtures/northwind2007.accdb` extracts 18 modules with source
(`Utilities` … `Report_Monthly Sales Report`); the jackcess ACE templates
carry an empty project (0 modules).
