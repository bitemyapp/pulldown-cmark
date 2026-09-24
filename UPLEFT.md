# Upleft's pulldown-cmark

This fork adds one option, `Options::ENABLE_CMARK_GFM_COMPAT`, off by default.
With it on, the parser follows cmark-gfm 0.29.0.gfm.13 as swift-cmark vendors
it (the parser swift-markdown uses) wherever that parser and CommonMark 0.31
disagree. Upleft turns it on together with `ENABLE_TABLES`,
`ENABLE_STRIKETHROUGH` and `ENABLE_TASKLISTS`, and compares the result with
cmark-gfm on every input it can generate. With the option off, nothing
changes.

The ports live in `pulldown-cmark/src/cmark_compat.rs`. Each names the C
function it follows. The rest of the patch is small hooks in `firstpass.rs`,
`parse.rs`, `scanners.rs` and `tree.rs`, and one new public item,
`LinkType::InlineAttributes`.

## What the option changes

Inlines:

- Emphasis flanking uses cmark's classes: Unicode symbols (emoji, `✅`) are
  not punctuation, so `**Ship it 🚀**now` is bold. A `~` next to a `*` or `_`
  run is skipped when the run looks at its neighbours.
- Emphasis and strikethrough are matched by cmark's `process_emphasis`.
- Strikethrough takes runs of one or two tildes by plain flanking, inside
  words too (`H~2~O`, `3~5`). Runs of unequal length cancel each other.
  Longer runs are scanned 100 at a time.
- Code spans use cmark's closer scan, with its cache of backtick run
  positions (after one scan reaches the end, a stale entry can leave a later
  span as text). Runs of more than 80 backticks never open a span.
- Inline HTML: a declaration needs an uppercase name and a space
  (`<!doctype html>` is text); comments, CDATA sections and processing
  instructions follow cmark's grammar; after a failed scan for one of them,
  cmark stops looking for that kind in the paragraph (a failed comment ends
  every `<!` form).
- Link destinations stop at the first space even inside parentheses, which
  need not balance there (`[js](alert(1 ))` is a link to `alert(1`). A
  title is the longest match. URLs and titles are unescaped entities first,
  then backslashes.
- A link turns off the link openers before it only until the next `[`, so a
  link can contain a link.
- A reference label must follow `]` directly; `[foo][ ]` is a collapsed
  reference; a shortcut or collapsed reference is impossible once a bracket
  opened inside the link text.
- `![^` is not an image. Numeric entities take up to 8 digits.
- References expand only while the destinations and titles they bring in
  add up to no more than the input's length or 100,000 bytes, whichever is
  larger; past that, they stay text. `Parser::cmark_input_length` gives the
  length when the text handed to the parser differs from cmark's input.
- swift-cmark's inline attributes, `^[text](attributes)` and
  `^[text][label]`, become links of type `LinkType::InlineAttributes` whose
  destination holds the attributes. A label after an attribute span's `]`
  that names no attribute definition disappears, as in cmark.

Blocks:

- A delimiter row turns the line before it into a table header, after any
  line of a paragraph and without a pipe (`Summary:` / `a | b` / `--|--`,
  or `a` / `-:`). The lines before stay a paragraph, with `\|` unescaped. A
  lazy line can be the header. A cell of only `:` is not a delimiter. After
  one failed try, the paragraph is never tried again. Cells, and that
  paragraph, are read with `\|` already turned into `|`, so an autolink or
  HTML tag can hold one.
- A table ends at a blank line, a line indented four columns or with a tab,
  any HTML block start, any list item, or any other block start. It also
  ends once it has filled in 0x80000 missing cells.
- Link reference definitions, and swift-cmark's attribute definitions
  (`^[label]: attributes`), are resolved when their paragraph ends or becomes
  a setext heading, not when it becomes a table. A setext underline after
  nothing but definitions is paragraph text. The destination and title rules
  are cmark's (`[a]:(` defines `(`; a title followed by text keeps it).
- HTML blocks start by CommonMark 0.29's conditions (kind 4 needs an
  uppercase letter; the kind 6 tag list is 0.29's). Kind 1 ends at any of the
  four end tags, in any case. Kind 7 can start on a lazy line.
- A task item is one where the whole line matches the tasklist extension's
  pattern: a space or tab must follow the box, and a box after `>` or after
  another item's marker on the line is text. The box is checked when the
  line holds `[x]` or `[X]` anywhere. A lazy continuation line whose deepest
  matching container is an item and which matches that pattern makes the
  item a task (its box follows the last such line) and loses its first
  three bytes.
- An empty list item stays open on a blank line indented to its content.
  An item whose only paragraph was definitions is empty.
- A line opens list items only among its first 99 new containers
  (`MAX_LIST_DEPTH`); past that, a marker is text.
- A tab that reaches column 4 before `>` does not continue a block quote.
  The rest of a tab that a container prefix partly used becomes spaces in
  the line's text (a table header after such a lazy tab keeps a leading
  space). A code fence's indent is counted in bytes.
- ATX headings lose trailing whitespace and a closing sequence as
  `chop_trailing_hashtags` cuts them. A closing code fence may be followed
  by tabs. Info strings are unescaped entities first, then trimmed, then
  unescaped backslashes.

## Updating

Rebase the `upleft` branch onto the new upstream tag. The hooks are small and
marked with `cmark_gfm_compat()`. Then run `cargo test` here and Upleft's
differential tool (`crates/markup/examples/markup_diff.rs`), which must stay
at zero differences.
