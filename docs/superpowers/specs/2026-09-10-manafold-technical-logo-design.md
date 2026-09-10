# Manafold — Technical Company Logo

**Status:** design-only; awaiting written-spec review before implementation planning
**Design baseline:** `d647e63`
**Scope:** Discord server branding and public documentation

This specification defines a vector-first company logo for Manafold. The
wordmark must remain part of every approved lockup. Existing working-tree
changes are unrelated and remain outside this design.

## 1. Objective

Create a clear, technical, and credible Manafold wordmark that works in public
documents and in a square Discord composition. The logo should feel like a
software or infrastructure company with a distinctive visual idea, not like a
generic game emblem.

## 2. Design concept

The symbol is an abstract folded “M” made from two angular, interlocking
planes. Their negative space communicates a fold, transformation, and a
structured system. The geometry should remain simple enough to recognize at
small sizes.

The primary lockup places the symbol to the left of the exact wordmark
`Manafold`. The compact lockup places the symbol above the wordmark so the name
remains present in a square Discord asset. A symbol-only lockup is out of
scope; the wordmark is always included.

The visual tone is technical, calm, and modern. The design must avoid literal
cards, mana symbols, fantasy creatures, or other game-specific imagery. It
must be an original mark and must not imitate a third-party logo or protected
symbol.

## 3. Visual system

The proposed palette is:

- Graphite: `#111827` for the wordmark and light-background use.
- Electric teal: `#19D3C5` for the folded symbol and emphasis.
- Off-white: `#F4F7F8` for the wordmark on dark backgrounds.

The primary dark treatment uses a teal symbol with an off-white wordmark. The
light treatment uses a teal symbol with a graphite wordmark. A flat,
single-color version is required; gradients and fine internal details are not
required for the core identity.

The wordmark should use a legible, open-licensed technical sans-serif base,
with medium or semibold weight and restrained letter spacing. The capitalization
and spelling remain exactly `Manafold`.

## 4. Required lockups

1. **Horizontal primary:** symbol on the left, `Manafold` on the right. Use in
   public documents, headers, and profile banners.
2. **Compact square:** symbol above `Manafold`, centered with generous clear
   space. Use for the Discord server image and other square placements.
3. **Monochrome variants:** dark-on-light and light-on-dark versions of both
   lockups.

All variants must use the same geometry, proportions, wordmark, and spelling.

## 5. Acceptance criteria

- Every approved lockup includes the `Manafold` wordmark.
- The folded “M” remains recognizable at approximately 32 px when used as the
  supporting symbol.
- The compact square composition keeps the wordmark readable at its intended
  Discord export size.
- The logo works on both light and dark backgrounds and remains identifiable in
  monochrome.
- The source is vector-first, uses a transparent background, and does not
  depend on remote assets at render time.
- The identity is readable, restrained, and technical without becoming
  visually indistinguishable from a generic developer-tool logo.

## 6. Non-goals

This design does not change the repository name, README, application UI,
documentation templates, or project metadata. It does not define a full brand
book, animation system, merchandise treatment, or final asset filenames until
the implementation plan is approved.
