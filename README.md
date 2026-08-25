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
polite explain <phrase>         what a phrase means, in English
polite check-vocabulary         make sure no two phrases could collide
polite bench                    measure against the budgets in the spec
polite grammar <file.json>      write the editor grammar
```

Try `polite words` first. Forty words is the whole of the everyday language.

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

The same table generates the editor highlighting, the documentation, the typo suggestions and one
test per row. And **`polite check-vocabulary` runs on every build**, refusing any two phrases that
could both match the same sentence, because the language must never guess which one you meant.

That claim has been paid for once already: the vocabulary grew from 88 phrases to 117 in a single
sitting — a third again as many words — and the measured cost of matching them did not move.

---

## Speed, on a school laptop

The reference machine is a 2-core, 3.8 GB laptop with slow eMMC storage — not an obstacle to work
around, but *the target*. A language for beginners that needs 8 GB is self-defeating: beginners
are the people on school laptops and hand-me-downs.

Measured there, by `polite bench`:

| | measured | budget |
|---|---|---|
| check a 1,000 line program | **5.7 ms** | < 10 ms |
| checking throughput | **176,000 lines/sec** | ≥ 150,000 |
| compiler memory per line | **181 bytes** | < 2 KB |
| compile and run `hello` | **0.03 ms** | < 3 ms |
| 300,000 turn numeric loop | **10.5 ms** | — |
| the same loop in CPython | 43.8 ms | — |
| **times faster than CPython** | **4.3×** | ≥ 2× |
| **parse slowdown at 40× the vocabulary** | **0.4–1.0%** | < 5% |
| `polite` binary, stripped | **768 KB** | < 3 MB |

That last row is the one that matters most, because it tests a *claim* rather than a speed: the
whole project rests on a large vocabulary being cheap. The benchmark generates four thousand extra
phrases and proves parsing does not notice.

Timings are the *best* of several runs rather than the average, and the vocabulary ratio is the
median of paired samples. On a two-core laptop something else is always waking up, and an average
measures whatever else the machine happened to be doing.

`polite bench --save` writes the baseline; a later run fails if anything slips by more than a
tenth, beyond a small slack so that a number sitting near zero is not judged by percentages.

---

## What is here, and what is not

**Working today**

- The four rules, and 117 phrases across 80 meanings
- Full type inference — you never write a type, and mistakes are caught before the program runs
- The `or` / `try` / `I am sure` system, and riskiness that spreads by itself
- Actions, including multi-word names, recursion, and calling before defining
- Lists, lookups, text, numbers, files, time, chance
- PoliteIR, constant folding, unreachable-code removal, and the `Backend` socket
- The reference runner, which never checks a type at runtime
- **Borrowing between files**, with sharing, private names, and circles caught before anything runs
- `run`, `check`, `words`, `explain`, `check-vocabulary`, `bench`, `grammar`
- VS Code highlighting, snippets and block-aware indentation
- 73 tests: a corpus of 19 programs, 20 pinned messages, a generated test per vocabulary row, and
  the ambiguity check

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
examples/                   programs to read
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
