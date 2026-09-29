# Forge parser validation: R1 and W1

Validation used Manafold baseline `1933a422fb2a6943f81f381dcece30092c7ed0db`
and Forge source revision `17c1ba92149b84127749bf84c231ed75107df1a2`. Each
Forge file was read from that revision and passed to the Rust
`parse_card_script()` function. The first `Name:` value was checked against the
Manafold card name. No Forge files were copied into this repository.

PR #257 was merged at the Manafold baseline commit above.

## Results

| Card | Deck | Forge file | Parse status | Notes |
|---|---|---|---|---|
| Fanatical Firebrand | R1 | `forge-gui/res/cardsfolder/f/fanatical_firebrand.txt` | PASS | Name verified |
| Hired Claw | R1 | `forge-gui/res/cardsfolder/h/hired_claw.txt` | PASS | Name verified |
| Magebane Lizard | R1 | `forge-gui/res/cardsfolder/m/magebane_lizard.txt` | PASS | Name verified |
| Emberheart Challenger | R1 | `forge-gui/res/cardsfolder/e/emberheart_challenger.txt` | PASS | Name verified |
| Razorkin Needlehead | R1 | `forge-gui/res/cardsfolder/r/razorkin_needlehead.txt` | PASS | Name verified |
| Hearthborn Battler | R1 | `forge-gui/res/cardsfolder/h/hearthborn_battler.txt` | PASS | Name verified |
| Ojer Axonil, Deepest Might | R1 | `forge-gui/res/cardsfolder/o/ojer_axonil_deepest_might_temple_of_power.txt` | PASS | Name verified; same file contains the `Temple of Power` alternate face |
| Nova Hellkite | R1 | `forge-gui/res/cardsfolder/n/nova_hellkite.txt` | PASS | Name verified |
| Burst Lightning | R1 | `forge-gui/res/cardsfolder/b/burst_lightning.txt` | PASS | Name verified |
| Lightning Strike | R1 | `forge-gui/res/cardsfolder/l/lightning_strike.txt` | PASS | Name verified |
| Mountain | R1 | `forge-gui/res/cardsfolder/m/mountain.txt` | PASS | Name verified |
| Rockface Village | R1 | `forge-gui/res/cardsfolder/r/rockface_village.txt` | PASS | Name verified |
| Plains | W1 | `forge-gui/res/cardsfolder/p/plains.txt` | PASS | Name verified |
| Ethereal Armor | W1 | `forge-gui/res/cardsfolder/e/ethereal_armor.txt` | PASS | Name verified |
| Spellbook Vendor | W1 | `forge-gui/res/cardsfolder/s/spellbook_vendor.txt` | PASS | Name verified |
| Ruin-Lurker Bat | W1 | `forge-gui/res/cardsfolder/r/ruin_lurker_bat.txt` | PASS | Name verified |
| Feather of Flight | W1 | `forge-gui/res/cardsfolder/f/feather_of_flight.txt` | PASS | Name verified |
| Optimistic Scavenger | W1 | `forge-gui/res/cardsfolder/o/optimistic_scavenger.txt` | PASS | Name verified |
| Shardmage's Rescue | W1 | `forge-gui/res/cardsfolder/s/shardmages_rescue.txt` | PASS | Name verified |
| Sheltered by Ghosts | W1 | `forge-gui/res/cardsfolder/s/sheltered_by_ghosts.txt` | PASS | Name verified |
| Seam Rip | W1 | `forge-gui/res/cardsfolder/s/seam_rip.txt` | PASS | Name verified |
| Origin of Spider-Man | W1 | `forge-gui/res/cardsfolder/o/origin_of_spider_man.txt` | PASS | Issue #222 alias: A Most Helpful Weaver |
| Skyward Spider | W1 | `forge-gui/res/cardsfolder/s/skyward_spider.txt` | PASS | Issue #222 alias: Wonderweave Aerialist |
| Abandoned Air Temple | W1 | `forge-gui/res/cardsfolder/a/abandoned_air_temple.txt` | PASS | Name verified |
| Evershrike's Gift | W1 | `forge-gui/res/cardsfolder/e/evershrikes_gift.txt` | PASS | Name verified |
| Dryad Militant | W1 | `forge-gui/res/cardsfolder/d/dryad_militant.txt` | PASS | Name verified |

```text
R1_UNIQUE = 12
R1_FOUND = 12
R1_PARSED = 12
R1_MISSING = 0
R1_FAILED = 0

W1_UNIQUE = 14
W1_FOUND = 14
W1_PARSED = 14
W1_MISSING = 0
W1_FAILED = 0
```

The 26 selected file parses succeeded with the parser. Ojer Axonil's file
contains an additional face, so field and ability totals below include that
face as part of the same Forge source file; it is not an extra selected deck
card.

## Syntax observed

Field occurrence counts across the complete 26 source files (including Ojer's
alternate face):

| Field | Occurrences |
|---|---:|
| `Name` | 27 |
| `ManaCost` | 27 |
| `Types` | 27 |
| `PT` | 13 |
| `K` | 21 |
| `A` | 12 |
| `T` | 14 |
| `R` | 3 |
| `S` | 8 |
| `SVar` | 38 |
| `Oracle` | 27 |
| `ALTERNATE` | 1 |

The ability parser returned structured syntax for all 37 `A`/`T`/`R`/`S`
lines. Observed `prefix$ category` forms were:

```text
AB$ ChangeZone (1)       AB$ DealDamage (1)       AB$ Mana (4)
AB$ Pump (1)             AB$ PutCounter (1)       AB$ PutCounterAll (1)
AB$ SetState (1)         Event$ DamageDone (1)    Event$ Moved (2)
Mode$ AttackersDeclared (1) Mode$ BecomesTarget (1) Mode$ ChangesZone (6)
Mode$ Continuous (8)     Mode$ Drawn (1)          Mode$ FullyUnlock (1)
Mode$ Phase (2)          Mode$ SpellCast (2)      SP$ DealDamage (2)
```

`K:` values included `Haste` (4), `Flying` (2), `Flash` (2),
`Enchant:Creature` (3), `Enchant:Creature.YouCtrl:creature you control` (2),
`Chapter:3:DBToken,DBPutCounter,DBPump`, and `Warp:2 R`. The parser retained
the text after `K:` verbatim, including its internal colons.

Unknown fields were retained: `AlternateMode`, `DeckHas`, `DeckHints`,
`DeckNeeds`, and `Variant`. The Ojer file's `ALTERNATE` separator is retained
as an ordered marker with an empty value. The SVar values include nested
`DB$`/`Count$` expressions; ability parameters include values such as
`TriggerZones$`, `Execute$`, `ReplaceWith$`, `AffectedDefined$`, and
`SpellDescription$`. The two `Variant:` values carry the issue aliases
`A Most Helpful Weaver` and `Wonderweave Aerialist`.

These are syntax observations only. Recognizing an ability shape does not
interpret its parameters or establish Manafold support. Concrete later Card
IR work will need to account for trigger/replacement/continuous mode lines,
SVar references, unknown metadata, and the two-face file boundary.

## Parser issue found

Before the fix, parsing Ojer Axonil's original file failed at line 14:

```text
ALTERNATE
```

This is Forge's standalone separator before the alternate face. The parser now
preserves that line in source order and continues parsing. A focused regression
test covers the separator and fields after it. No other structural parsing
errors occurred in the 26 selected files.
