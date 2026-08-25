# PoliteLang

**A programming language you ask nicely.**

```polite
please repeat for every n from 1 to 20:
    check if n divides evenly by 15:
        show "FizzBuzz"
    otherwise if n divides evenly by 3:
        show "Fizz"
    otherwise if n divides evenly by 5:
        show "Buzz"
    otherwise:
        show n
    thanks
thank you for repeating
```

By Creeperdiamonds Studios.

---

## Why it exists

Three reasons, in this order:

1. **You learn more English.** Real words, used correctly, in real sentences.
2. **You learn to have more manners.** Courtesy is the syntax, and the language is courteous back.
3. **You get a genuinely simple language** — one you can write real programs in.

And the sentence that makes those three fit together:

> **Simple means few *rules*, not few *words*.**
> English has a million words and a grammar a five-year-old handles. PoliteLang takes the same
> bargain: four structural rules, unlimited vocabulary.

### Why not just use Python or Scratch

Both are "beginner-friendly" by *programmer* standards — easier than C, not easy in absolute
terms.

- **Python asks you to learn notation**: `:` `==` `def` `[]` `{}` `self`, plus indentation rules
  that silently break working programs.
- **Scratch asks you to learn an interface**: a palette, drag-and-drop, block shapes, a mouse.

**PoliteLang asks you to learn nothing you do not already have.** You already speak English. You
already know how to ask for something politely. That is the entire entry cost.

---

## Getting started

```
cargo build --release
./target/release/polite run examples/hello.polite
```

The `polite` command:

```
polite run <file.polite>        run a program
polite check <file.polite>      look it over without running it
    --show-middle               also print the middle language
polite words [about <topic>]    every word the language knows
polite explain <word>           what a word means, in PoliteLang and in English
polite check-vocabulary         make sure no two phrases could collide
polite bench                    measure against the budgets in the spec
polite grammar <file.json>      write the editor grammar
```

Try `polite words` first. Forty words is the whole of the everyday language.

**New to programming?** Start with **[the guide](docs/GUIDE.md)**. It begins from nothing at all,
fourteen short lessons, and every program in it lives in `examples/guide/` and is run by the test
suite — so nothing in it can quietly stop working.

---

## The four rules

That is all the structure there is.

**1. `please` opens a request.**

```polite
please show "hello"
```

`kindly`, `would you`, `would you please`, `if you would`, `if you would be so kind` and
`if it is not too much trouble` all mean **exactly** the same thing. They exist for people who
find `please` too pleading. The language never treats one differently from another.

**2. `thank you` closes a block.** (`thanks` is the same word, shorter.)

This takes the place of brackets, of `end`, and of Python's meaningful whitespace. **Indentation
is purely cosmetic** — paste code in badly indented and it still runs correctly. A whole category
of beginner pain simply does not exist here.

A closer may say what it is closing, and if it does, the language checks it:

```polite
thank you for repeating
thank you for checking
thank you for defining
thank you for trying
```

**3. One courtesy word covers everything inside the block it opened.**

You asked nicely to go in; you do not re-ask on every line. A five-hundred-line program carries
about forty courtesy words, not four hundred — one per action, loop and check. Being extra polite
anywhere inside is always accepted and never a problem.

```polite
please:
    show "one"
    courtesy
    word
thank you for everything
```

**4. Words first, symbols allowed.** `2 plus 3` and `2 + 3` are the same thing.

---

## A tour

**Remembering things.** Introducing a name and changing one are different words on purpose, so a
typo is caught instead of quietly becoming a second variable.

```polite
please remember score is 0
please change score to 5
please add 1 to score
please take 1 from score
```

**Asking.** You never say what kind of answer you want — the language works it out from what you
do with it afterwards.

```polite
please ask "What is your guess? " as guess
please check if guess is under secret:      -- so `guess` is read as a number
    show "A little higher."
thanks
```

**Text**, with values dropped straight in. Anything that is a value anywhere is a value in here.

```polite
please show "Got it in {guesses} guesses!"
please show "the list holds {the number of items in things}"
please show "hello, " then name then "!"
```

**Lists and lookups.**

```polite
please remember shopping is an empty list
please add "bread" to shopping
please show the join of the sorted version of shopping with ", "

please remember ages is an empty lookup
please put 11 for "ada" in ages
please show the value for "ada" in ages or 0
```

**Actions.** Defining one adds a word to your vocabulary, and names can be several words long.

```polite
please define greet with a name and a greeting:
    give back "{greeting}, {name}!"
thank you for defining

please show greet with "Creeperdiamonds Studios" and "Good morning"
```

**Numbers have no limit, and can be exact.** Whole numbers keep going as far as you need, and
fractions stay fractions:

```polite
please remember f is 1
please repeat for every n from 1 to 100:
    multiply f by n
thanks
please show f            -- all 158 digits of a hundred factorial, exactly

please show 1 over 3 plus 1 over 3 plus 1 over 3   -- 1, exactly
please show 1 over 2 plus 1 over 3                 -- 5/6
please show 0.75 as a fraction                     -- 3/4
```

And complex numbers, written the way they are read:

```polite
please remember here is 3 plus the imaginary number 4
please show the size of here                  -- 5.0
please show here times here                   -- -7+24i
please show the complex square root of 0 minus 1   -- 1i
```

They can be the same or different but never greater or lesser, and the language says so before the
program runs rather than inventing an order.

**Arithmetic, in full.** Trigonometry, logarithms, powers and roots, statistics, percentages:

```polite
please show the number pi rounded to 5 decimal places      -- 3.14159
please show the sine of the number pi divided by 2 or 0    -- 1.0
please show 180 in radians rounded to 5 decimal places     -- 3.14159
please show the logarithm of 8 in base 2 or 0              -- 3.0
please show the factorial of 10 or 0                       -- 3628800
please show the greatest common factor of 12 and 18        -- 6
please show the median of marks or 0
please show the spread of marks or 0
please show 25 as a percentage of 200 or 0                 -- 12.5
please show 15 percent of 200                              -- 30.0
please show 15 kept between 1 and 10                       -- 10
```

Every one of these that *can* fail says so and makes you say what happens instead — there is no
angle whose sine is 2, no logarithm of zero, and no share of nothing.

**Number theory, counting, bases, bits, and linear algebra:**

```polite
please show 2147483647 is prime                          -- yes
please show the join of the prime factors of 360 with " times "
please show the divisors of 28
please show 2 to the power 100 within 1000000007 or 0    -- stays small
please show the ways to choose 50 from 100 or 0          -- 100891344545564193334812497256

please show 255 in binary                                -- 11111111
please show 255 in hexadecimal                           -- ff
please show the value of "ff" in base 16 or 0            -- 255
please show 12 bitwise and 10                            -- 8
please show 1 shifted left by 10                         -- 1024

please show the mode of marks or 0
please show the variance of marks or 0
please show the correlation of ups and downs or 0        -- -1.0

please show the dot product of a and b or 0
please show the join of the cross product of x and y with ", "
please show the magnitude of across                      -- 5.0
please show the determinant of m or 0
please show the inverse matrix of m or an empty list
please show the identity matrix of size 3 or an empty list
```

A vector is a list of numbers and a matrix is a list of those lists, so there is no new kind of
thing to learn: the rows really are rows, and you can look at one the way you look at any list.

**Only what could actually go wrong.** `1 over 3` can no more fail than `1 plus 3` can, so the
language works that out and stays quiet. Where anything is unknown it asks, as always — and a
fallback that could never be reached says so kindly rather than failing:

```
Just so you know, in game.polite, line 9:

    please show the square root of 81 or 0
                ^^^^^^^^^^^^^^^^^^^^^^^^^^

This fallback is never needed, because `the square root of {value}` always works out here.
```

**A note on how phrases that follow a value read.** Some belong to the number right beside them,
and some are said about the whole sum. Which is which is written in the vocabulary table rather
than guessed at, and both read the way English does:

```polite
please show 2 plus 3 squared                 -- 11, because squaring belongs to the three
please show (2 plus 3) squared               -- 25
please show 3 plus 40 kept between 1 and 10  -- 10, because keeping is said about the sum
```

**And a good deal more**, because a large vocabulary is the whole point. `polite words` lists
every one of them, and `polite explain` says what any of them means:

```polite
please show the piece of "hello there" from 1 to 5
please show "the rain in spain" with "ain" changed to "AIN"
please show the letter 2 of "hello" or "?"
please show the remainder of 17 divided by 5 or 0
please show the smaller of 3 and 8
please show 2 to the power of 10
please show the average of numbers or 0
please show the rest of numbers
please show the first 2 items of numbers
please show numbers is empty
```

And several ways to say things you could already say, because `please` is not the only word
somebody might reach for:

```polite
please have a look at "another way of showing something"
please remember that greeting is "hello"
please update greeting to "good day"

please if greeting is "good day":
    show "a shorter way of checking"
thanks

please look at every letter in the letters of "hi":
    show letter
thanks
```

**Borrowing between files.** A file keeps everything to itself unless it offers it — which is
both safer and exactly the manner of offering something rather than leaving your things lying
about.

```polite
-- greetings.polite
please define greet with a name:
    give back "hello, {name}!"
thanks

please share greet
```

```polite
-- main.polite
please use "greetings.polite"
please show greet with "Creeperdiamonds Studios"
```

`borrow` says the same thing as `use`, and is the more mannerly of the two. Anything not shared
stays private, files that borrow each other in a circle are caught before anything runs, and every
message points at the line in the file you actually wrote it in.

**Things that might not work out.** The language will never crash on you, and it will never
quietly invent an answer. So when something might fail, you say what happens if it does — in one
of three ways.

*Give it something to fall back on:*

```polite
please remember saved is the contents of "save.txt" or "nothing saved yet"
```

*Handle it properly:*

```polite
please try to:
    remember settings is the contents of "settings.polite"
    show settings
otherwise if it does not work out:
    show "I could not read them, because {what went wrong}."
thank you for trying
```

*Or take responsibility:*

```polite
please remember config is the contents of "settings.polite", I am sure
```

If you are wrong about that last one, the program stops — politely, and holding you to your word.

**And it spreads by itself.** An action that does something risky without handling it becomes
risky too, with nobody writing an annotation:

```
Just so you know, in game.polite, line 4:

    give back the number in the contents of "score.txt"
              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

`load the score` might not work out, because `the number in {value}` might not.

That is perfectly fine. I am telling you so that whoever uses this
action can plan for it.
```

The upshot: **the single most common crash in every mainstream language is impossible here**, and
you never learned the words *null*, *exception* or *Option* to get that.

---

## The messages are the product

```
In game.polite, line 2:

    please add "hello" to score
               ^^^^^^^

I was expecting a number here, and this is text.

This place wants what is being added.

You can read a number out of text:

    the number in ...
```

Compare `TypeError: unsupported operand type(s) for +=: 'int' and 'str'`. Same mistake. One of the
two teaches you English *and* the fix.

Five rules hold for every message: plain English sentences, **never blame the person reading it**,
show the actual line, suggest a fix, explain why. There are seventeen pinned message snapshots in
`tests/errors`, and a lint that fails the build if any message ever contains *illegal*, *invalid*
or *fatal*.

---

## How it is built

```
you write        game.polite
                     |
polite-syntax    words, then sentences        <- grammar comes from the vocabulary table
polite-check     names, types, riskiness      <- you never write a type
polite-ir        PoliteIR, the middle language
                     |
                     +--> polite-run    the reference runner   (works today)
                     +--> native        Cranelift              (later)
                     +--> JVM           Minecraft plugins      (later)
                     +--> WebAssembly   a browser playground   (later)
```

**The vocabulary is data, not code.** Every word in the language is a row in
[`vocabulary/core.vocab`](vocabulary/core.vocab):

```
phrase  "repeat {n} times"        ->  Loop.Count
phrase  "repeat while {cond}"     ->  Loop.While
phrase  "keep going forever"      ->  Loop.Forever
synonym show = display, print, say, write out, tell me
```

Adding a word is adding a row. No parser is edited. No backend is edited. Ever. And because every
phrasing collapses to the same middle language, supporting *V* ways of saying things across *N*
backends costs **V + N** rather than **V × N** — which is exactly why a large vocabulary is
affordable at all.

The same table also holds **English**. Every one of the 250 words the language uses is written
down with its part of speech and what it actually means, quite apart from any program, so
`polite explain` answers in two halves:

```
  show {value}

  -- IN POLITELANG --
  To show something is to let it be seen. This puts a value on the screen.
  A request, from the everyday vocabulary.
  Other ways to say the very same thing: display, print, say, write out, tell me...

  -- IN ENGLISH --
  show
    verb   To let something be seen; to put it where somebody can look at it.
    noun   A display put on for people to watch.
```

A test fails if any word is ever left without one. That serves purpose 1 directly: you cannot use
this language without learning some English along the way.

The same table generates the editor highlighting, the documentation, the typo suggestions and one
test per row. And **`polite check-vocabulary` runs on every build**, refusing any two phrases that
could both match the same sentence, because the language must never guess which one you meant.

That claim has been paid for twice already: the vocabulary went from 88 phrases to 117, and then
to 150 when the whole of arithmetic went in — nearly double, in an afternoon — and the measured
cost of matching them did not move.

---

## Speed, on a school laptop

The reference machine is a 2-core, 3.8 GB laptop with slow eMMC storage — not an obstacle to work
around, but *the target*. A language for beginners that needs 8 GB is self-defeating: beginners
are the people on school laptops and hand-me-downs.

Measured there, by `polite bench`:

| | measured | budget |
|---|---|---|
| check a 1,000 line program | **5.7 ms** | < 10 ms |
| checking throughput | **170,000–190,000 lines/sec** | ≥ 150,000 |
| compiler memory per line | **181 bytes** | < 2 KB |
| compile and run `hello` | **0.03 ms** | < 3 ms |
| 300,000 turn numeric loop | **12.8 ms** | — |
| the same loop in CPython | 42.7 ms | — |
| **times faster than CPython** | **3.4×** | ≥ 2× |
| **parse slowdown at 40× the vocabulary** | **0.2–1.0%** | < 5% |
| `polite` binary, stripped | **768 KB** | < 3 MB |

That last row is the one that matters most, because it tests a *claim* rather than a speed: the
whole project rests on a large vocabulary being cheap. The benchmark generates four thousand extra
phrases and proves parsing does not notice.

Whole numbers having no limit costs about a third of the speed of an integer loop — every
addition has to notice when it outgrows a machine word — which took that loop from 10.5 ms to
12.8 ms. That is written down here rather than quietly left out: it is the price of never wrapping
round to a wrong answer, and it is worth paying.

Timings are the *best* of several runs rather than the average, and the vocabulary ratio is the
median of paired samples. On a two-core laptop something else is always waking up, and an average
measures whatever else the machine happened to be doing.

`polite bench --save` writes the baseline; a later run fails if anything slips by more than a
tenth, beyond a small slack so that a number sitting near zero is not judged by percentages.

---

## What is here, and what is not

**Working today**

- The four rules, and 195 phrases across 175 meanings, with each of its 250 words defined in English too
- Full type inference — you never write a type, and mistakes are caught before the program runs
- The `or` / `try` / `I am sure` system, and riskiness that spreads by itself
- Actions, including multi-word names, recursion, and calling before defining
- Lists, lookups, text, numbers, files, time, chance
- PoliteIR, constant folding, unreachable-code removal, and the `Backend` socket
- The reference runner, which never checks a type at runtime
- **Borrowing between files**, with sharing, private names, and circles caught before anything runs
- `run`, `check`, `words`, `explain`, `check-vocabulary`, `bench`, `grammar`
- VS Code highlighting, snippets and block-aware indentation
- **Mathematics in full** — see below
- 137 tests: a corpus of 22 programs, 20 pinned messages, a generated test per vocabulary row, the
  ambiguity check, and every lesson in the guide

**Designed, written down, not built yet** — and named here rather than left to be discovered:

- **Choosing only some words to borrow** — `use draw and fill from the drawing kit`. Borrowing
  takes the whole of what a file shares, for now.
- **A package manager**, so borrowing reaches beyond files sitting next to each other.
- **`check if save has something`** (spec 7.3). The three ways in 7.2 all work; flow-sensitive
  narrowing does not exist yet.
- **Kinds of your own** — `please describe a player with a name and a score`. Use a lookup for now.
- **Actions are monomorphic.** One action takes one kind of value, across the whole program.
- **No language server yet**, so the editor has highlighting but not live errors.
- **A cycle collector.** Reference counting leaks values that point in a circle. Beginner programs
  essentially never build those, and the spec always said this was a later sub-project.

**Deliberate departures from the spec**, made while building and worth knowing about:

- Whole and decimal numbers mix freely, and mixing gives a decimal. Text and numbers still refuse
  to mix, which is what catches the mistake in section 8.
- A type nothing pins down becomes **text**, quietly, rather than producing the spec's "I cannot
  work out what this holds" message. In practice an unsettled name is always one being shown or
  dropped into text, and a lecture would not have helped.
- The O(n²) text trap of spec 10.3 is closed in the runtime rather than by a compiler rewrite:
  text grows **in place** when nothing else is holding it. Same result, more general, less
  machinery.
- Your own actions win over the shipped vocabulary when both could match. You taught the language
  that word on purpose.
- `then` joins values as text (`"hello, " then name`), which the spec promised and did not define.
- Anything between `{ }` in text is a full value, not just a name — including text of its own.

---

## Where things live

```
vocabulary/core.vocab       every word in the language
crates/polite-diag          gentle messages
crates/polite-vocab         the phrase table, its index, the ambiguity check
crates/polite-syntax        lexer and parser
crates/polite-check         names, types, riskiness
crates/polite-ir            PoliteIR, lowering, optimisation, the Backend socket
crates/polite-run           the reference runner
crates/polite-std           values and the standard library
crates/polite-cli           the `polite` command
docs/GUIDE.md               learn the language from nothing
examples/                   programs to read
examples/guide/             one runnable program per lesson
tests/programs              the corpus, and the future backend conformance suite
tests/errors                every message, pinned word for word
tests/*/parts               files the corpus borrows from
vscode-politelang/          the editor extension
docs/superpowers/specs/     the design, written before any of this
```

The whole thing has **no dependencies at all** — no `serde`, no `toml`, no `regex`, no parser
generator, no async runtime. That is what keeps a full rebuild under half a minute on two cores.

---

## What comes next

Each of these is its own project, plugging into a socket and a test corpus that already exist:

1. **A package manager**, so borrowing reaches past the files next door
2. **A native backend** through Cranelift — the road toward genuinely low-level work
3. **A JVM backend** — the realistic road to Minecraft plugins
4. **A WebAssembly backend** — a browser playground, so anyone can try it with no install
5. **A language server**, so the editor shows the same gentle messages as you type

---

## Licence

MIT.
