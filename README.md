
# poorerpoint

A terminal presentation engine. Markdown in, ASCII out.

Spacebar advances. Headers persist. Content flows. Nothing else.

---

## Why

PowerPoint has no concept of "this header stays on screen." Every slide is a
fresh pile of independent objects, and persistence is something you simulate
by hand — copy the same text box N times, tune entrance and exit triggers to
the millisecond, and hope it reads as one thing that never moved. It's a lie
told with an animation timeline, and it costs a half-night before every
presentation.

poorerpoint has the concept. A header stack is sticky: a new header replaces
the same-or-higher level and everything below it, and the rest stays put. The
only thing that animates is what actually changed. Titles slide in from the
right in 333 ms — enough to notice, not enough to steal the show. Bullets
follow. Then nothing moves until you press space.

The point is not the animation. The point is that writing a deck in plain
markdown forces you to commit to a *structure* before you touch a font. If
the heading levels don't make sense as text, they won't make sense as slides,
and no theme will save them.

---

## Install

```sh
git clone https://github.com/<you>/poorerpoint
cd poorerpoint
cargo build --release
```

Binary lands at `target/release/poorerpoint`.

Requires a Rust toolchain and a terminal that speaks UTF-8.

---

## Usage

```sh
poorerpoint deck.md
```

Input is a file, always. No stdin, no config, no arguments beyond the path.

---

## Writing a deck

Regular markdown. Headers are titles and subtitles; content under a header is
a slide.

```markdown
# poorerpoint

A terminal presentation engine.

## Why

- Markdown in, ASCII out
- Headers stick, content flows
- Space is the only button you need

## How

### Parsing

Headers mutate a **sticky stack**.
Content attaches to the current stack.

### Rendering

Titles slide in from the right.
Bullets follow, one every 55 ms.

# Fin

Space to leave.
```

### The rules

- **Headers form a stack.** Each level indents one space. H1 flush left,
  H2 one space, H3 two, and so on. In practice you rarely need more than four.
- **A header persists** until another header of the same or higher level
  replaces it. Lower-level headers under it are dropped too.
- **A slide is a stack state + the content beneath it.** The next header that
  has content starts the next slide.
- **Headers with no content are skipped.** No empty slides.
- **Consecutive headers merge.** An H1 followed immediately by an H2 renders
  as a stacked title block, not two slides.

### Supported markdown

- Headers (`#` through `######`)
- Paragraphs, `**bold**`, `*italic*`, `` `code` ``
- Bullet lists (nested)
- Fenced code blocks
- Horizontal rules
- Tables (with alignment)

---

## Keys

| Key       | Action                                                     |
|-----------|------------------------------------------------------------|
| `Space`   | Next slide. Ignored while animating — never buffered.      |
| `→` / `l` | Next slide. Always live; resets and replays the animation. |
| `←` / `h` | Previous slide. Resets and replays the animation.          |
| `↓` / `j` | Scroll content down. Resets on slide change.               |
| `↑` / `k` | Scroll content up. Resets on slide change.                 |
| `q` / `Esc` | Quit.                                                    |

Content that overflows the viewport is auto-scrolled so new material is always
visible. Manual scrolling is there for the presenter when a slide runs long.

---

## Design notes

- **Timing.** Title block: 333 ms total, lines staggered so the last lands at
  333. Content: one block every 55 ms starting at 333. Frame rate 60 fps.
- **Tables** fit to terminal width. Column widths are measured two ways —
  full cell text (the natural width) and longest single word (the floor below
  which wrapping breaks) — then distributed proportionally when the table
  doesn't fit. Long prose wraps; long unbreakable words are allowed to push
  the table past the right edge rather than break mid-word.
- **Wrapping** is by whitespace everywhere. No mid-word breaks.
- **State** is a `Vec<Slide>`, each carrying its own title-stack snapshot and
  the list of lines *introduced by that slide* (a longest-common-prefix diff
  against the previous one). Animating that diff is what makes persistence
  free — unchanged lines were never re-entered, so they never needed to be
  re-hidden.

---

## Archaeology

This is the third version.

**asciishow** was the first — a curses program with a custom DSL. Every deck
opened with a settings block, then `!CONTENT!`, then commands: `!slide`,
`!label`, `!wait`, `!center`, `!indent`. You could place text at absolute
coordinates, choose per-command animation speeds, and type a title
character-by-character at 100 fps. It was more expressive than what came
after, and that was exactly the problem: it had one speaker.

**poorerpoint.py** was the second. Same DSL, finally named. Still no model of
a slide — persistence was achieved by *not clearing the curses window*, the
same trick PowerPoint makes you do by hand. It worked, and it was forgotten,
by its own author, within a few years. A private language dies with the mood
that invented it.

**poorerpoint** (this one) replaced the DSL with markdown, deleted every
layout command, and added exactly one thing the predecessors never had: a
data structure. `Slide { stack, anim, blocks }`. Everything else is a rounding
of what was already there.

The first two were written at 2pm on the patio of the cheapest hotel in
Managua, half a drink in, annoyed at a subscription fee. That is the whole
origin story. It is not a dramatic one. It is, on reflection, the correct one.

---

## License

MIT. See [LICENSE](LICENSE).

```
Copyright (c) 2026 SYSAULAB

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
