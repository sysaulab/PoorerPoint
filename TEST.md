
# Poorerpoint — Feature Tour

Every supported feature, one slide per area.

Navigate: **← →** or **h l** · Scroll: **↑ ↓** or **j k** · Quit: **q**

## Inline Styles

Plain text paragraph.

**Bold**, *italic*, and `inline code` in isolation.

Combined: **bold with *nested italic*** and `code with **bold** inside`.

## Bullet Lists

Flat list:

- Alpha
- Beta
- Gamma

Nested (two levels):

- Parent A
  - Child A1
  - Child A2
- Parent B
  - Child B1

Deeper:

- Level zero
  - Level one
    - Level two
      - Level three

## Ordered Lists

Numbered sequence:

1. First
2. Second
3. Third

Ordered with bullet children:

1. Ordered one
   - bullet child
   - another bullet
2. Ordered two

## Task Lists

- [x] Install dependencies
- [x] Write parser
- [ ] Write tests
- [x] Ship it

## Blockquotes

Simple quote:

> The quick brown fox
> jumps over the lazy dog.

Nested quotes:

> Level one
> > Level two
> > > Level three

Quote containing a list:

> - quoted bullet
> - second quoted bullet

## Code Blocks

Indented code:

    fn main() {
        println!("hello, world");
    }

Fenced code:

```
let x: i32 = 42;
let y = x * 2;
println!("{}", y);
```

## Horizontal Rules

A paragraph above the rule.

---

A paragraph below the rule.

## Tables

Basic:

| Name  | Value |
|-------|-------|
| alpha | 1     |
| beta  | 2     |

All four alignments:

| Left | Center | Right | Default |
|:-----|:------:|------:|:--------|
| a    | b      | c     | d       |
| longer text | more | 123 | default |

## Wide Characters

CJK: 你好世界，这是一个测试。

Mixed scripts: Hello 世界! Bonjour le monde!

Emoji: 🎉 🚀 ✨ 🦀

Wide chars in a table:

| Script  | Sample |
|---------|--------|
| Chinese | 你好   |
| Emoji   | 🎉     |

## Wrapping & Overflow

This paragraph is deliberately long so that on a narrow terminal it must wrap across several lines. It should break only at word boundaries provided each word fits in the available width.

Antidisestablishmentarianismpseudopseudohypoparathyroidism — a single unbreakable word long enough that it must be chunked mid-word.

## Heading Hierarchy

Content under an H2 (title stack: H1 + H2).

### Level 3

Content under an H3 (title stack grows to H1 + H2 + H3).

#### Level 4

Content under an H4 (title stack grows to four levels).

## Scroll Test

A tall slide to exercise scrolling:

```
line 01
line 02
line 03
line 04
line 05
line 06
line 07
line 08
line 09
line 10
line 11
line 12
line 13
line 14
line 15
line 16
line 17
line 18
line 19
line 20
line 21
line 22
line 23
line 24
line 25
```

Paragraph after the tall code block.

## Edge Cases

Empty parent list item:

-
  - orphan child

Long unbreakable identifier:

`a_very_long_identifier_with_no_spaces_at_all_that_will_need_mid_word_breaking`

---

End of the tour. **q** to quit.
