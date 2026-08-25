# PoliteLang — Design Specification

- **Author:** Creeperdiamonds Studios
- **Date:** 2026-08-25
- **Status:** Draft 1 — design approved in brainstorming
- **Short name:** Polite · **CLI:** `polite` · **File extension:** `.polite`

---

## 1. Purpose

PoliteLang exists for three reasons, in this order:

1. **You learn more English.** Real words, used correctly, in real sentences.
2. **You learn to have more manners.** Courtesy is the syntax, and the language models good manners back.
3. **You get a genuinely simple language** — one you can write real programs in.

### 1.1 The bargain that makes this possible

> **Simple means few *rules*, not few *words*.**

English has a million words and a grammar a five-year-old handles. PoliteLang takes the same
bargain: **four structural rules, unlimited vocabulary.** This sentence resolves the apparent
contradiction between "over-oversimplified" and "large vocabulary", and every later decision in
this document follows from it.

### 1.2 Why not just use Python or Scratch

Both are "beginner-friendly" by *programmer* standards — easier than C, not easy in absolute terms.

- **Python asks you to learn notation**: `:`, `==`, `def`, `[]`, `{}`, `self`, plus indentation
  rules that silently break working programs.
- **Scratch asks you to learn an interface**: a palette, drag-and-drop, block shapes, a mouse.

**PoliteLang asks you to learn nothing you do not already have.** You already speak English. You
already know how to ask for something politely. That is the entire entry cost.

---

## 2. Design principles

These are binding. A proposed feature that fails one of them does not go in.

1. **The read-aloud test.** Read any PoliteLang program aloud to somebody who has never
   programmed. They should understand roughly what it does. A feature that fails this is
   redesigned or dropped.
2. **Few rules.** Four structural rules exist. A feature needing a fifth must justify itself
   against principle 1 and against the whole of section 10.
3. **Never blame the human.** No diagnostic ever says *illegal*, *invalid*, *fatal*, or "you did X
   wrong". Enforced by a build lint (section 11).
4. **Never silently invent an answer.** The language does not crash, and it does not guess
   (section 7).
5. **Cleverness lives in the compiler, never in the user's head.** If something can be optimised
   automatically, optimise it automatically rather than teaching a workaround (section 10.3).
6. **The target machine is a cheap laptop.** 2 cores, 3.8 GB RAM, slow eMMC. Beginners are the
   people on school laptops and hand-me-downs. Performance budgets are requirements, not
   aspirations (section 10.4).

---

## 3. Language surface

### 3.1 The four structural rules

**Rule 1 — `please` opens a request.** Every statement that *does* something begins with a
courtesy word. Continuations (`otherwise:`) do not; they belong to a request already in progress.

**Rule 2 — `thank you` closes a block.** This replaces braces, `end`, and significant whitespace.
**Indentation is purely cosmetic.** Badly indented code still runs correctly. This deliberately
removes an entire category of beginner pain that Python inflicts.

**Rule 3 — one courtesy word covers everything inside the block it opened.** You asked nicely to
enter; you do not re-ask on every line. Being extra polite anywhere inside is always accepted and
never an error.

**Rule 4 — words first, symbols allowed.** `2 plus 3` and `2 + 3` are the same thing. Nobody is
forced to spell out arithmetic; nobody is forced to memorise operators.

### 3.2 Courtesy vocabulary

Openers, all **exactly** interchangeable with no semantic difference whatsoever:

```
please · kindly · would you · would you please · if you would · if you would be so kind
```

They exist as style alternatives — specifically for people who find `please` too pleading. The
language never treats one differently from another.

Closers, likewise interchangeable:

```
thank you · thanks
```

A closer may optionally name what it closes. If present it is **checked**, and a mismatch is
reported — so the option is genuinely useful rather than decoration:

```
thank you for checking
thank you for repeating
```

### 3.3 A worked example

```polite
-- FizzBuzz, politely

please repeat for every n from 1 to 100:
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

```polite
-- A guessing game

please define play:
    remember secret is a random whole number from 1 to 100
    remember guesses is 0

    keep going forever:
        ask "What is your guess? " as guess
        add 1 to guesses

        check if guess is secret:
            show "Got it in {guesses} guesses!"
            stop repeating
        otherwise if guess is under secret:
            show "A little higher."
        otherwise:
            show "A little lower."
        thanks
    thanks
thanks

please play
```

Rule 3 in action: a 500-line program carries roughly 40 courtesy words — one per action, loop and
check — rather than 400. The language reads politely at every boundary, which is where politeness
reads naturally, and does not tax you on line 400 of a game loop.

### 3.4 A courtesy block

One courtesy word may open a plain run of actions:

```polite
please:
    show "Starting up..."
    remember score is 0
    remember lives is 3
    show "Ready."
thanks
```

### 3.5 Values, naming and updating

```polite
remember score is 0            -- introduces a new name
change score to 5              -- updates an existing one
set score to 5                 -- synonym
add 1 to score                 -- common shorthand
take 1 from score
```

Introducing and updating are **different words on purpose**: it lets the checker catch a typo'd
name instead of silently creating a second variable, which is one of the most common sources of
baffling beginner bugs.

### 3.6 Comparisons and logic

```
is · is not · is over · is under · is at least · is at most · is between a and b
and · or · not
```

Symbol forms (`>`, `<`, `>=`, `<=`) are accepted, per rule 4.

### 3.7 Text

Double quotes. Interpolation with braces, or the word form — identical meaning:

```polite
show "Got it in {guesses} guesses!"
show "Got it in " then guesses then " guesses!"
```

Braces are the single piece of invented notation in the language. They earn their place by
removing `+` from almost all text handling, and the word form exists for anyone who would rather
not use them.

### 3.8 Comments

```polite
-- primary form
by the way, this is also a comment
```

### 3.9 Actions

Defining an action **adds a verb to your vocabulary**:

```polite
please define greet with a name and a greeting:
    give back "{greeting}, {name}!"
thanks

please show greet with "Creeperdiamonds Studios" and "Good morning"
```

### 3.10 Two known ambiguities and their rules

These are real edges, resolved by stated rules rather than pretended away.

**`and` does double duty** — separating arguments, and boolean conjunction. **Rule: inside an
argument list, `and` separates arguments.** To pass a boolean expression as an argument, name it
first with `remember`. One teachable rule instead of a parsing ambiguity.

**`or` does double duty** — boolean disjunction, and the fallback operator of section 7.
**Rule: after an expression that might not work out, `or` is the fallback; otherwise it is
boolean.** This is resolved by the checker, which already knows which expressions are risky, so
the parser never has to guess.

---

## 4. The vocabulary system

The vocabulary is expected to grow large — hundreds of phrases, eventually thousands. This section
is what makes that a strength rather than the thing that kills the project.

### 4.1 The vocabulary is data, not code

Nobody hand-writes parser code per phrase. The language's words live in a table the compiler reads:

```
phrase  "repeat {n} times"                      ->  Loop.Count(n)
phrase  "repeat while {cond}"                   ->  Loop.While(cond)
phrase  "repeat until {cond}"                   ->  Loop.Until(cond)
phrase  "keep going forever"                    ->  Loop.Forever
phrase  "do this for every {x} in {list}"       ->  Loop.Each(x, list)
phrase  "repeat for every {x} from {a} to {b}"  ->  Loop.Range(x, a, b)

synonym show           =  display, print, say, write out, put
synonym remember       =  note, keep, store, let
synonym stop repeating =  break out, that is enough, leave the loop
```

Six loop phrasings and five ways to say `show`, all landing on the same handful of IR
instructions. This is the whole reason for choosing a middle language (section 9.3).

Consequences, all from this one table:

- **Adding a word is adding a row.** No parser edit. No backend edit. Ever.
- **The VS Code extension is generated from it** — highlighting, completion, hover docs, always in
  sync, never hand-maintained.
- **The documentation is generated from it.** A 400-word language documents itself or it does not
  get documented.
- **Typo help is free** — `dispay` -> *"did you mean `display`?"* is a text-distance search over
  the table.
- **Tests are free** — every row gets a generated round-trip test (section 11).

The table has its own small line-based format, parsed by hand. This deliberately avoids `serde`
and `toml` — two of the heaviest common Rust dependencies — and produces a format pleasant for
contributors to edit.

### 4.2 Tiers — nobody learns 400 words

The table marks each row with a tier:

| Tier | Size | Contents |
|---|---|---|
| **Everyday** | ~40 words | show, ask, remember, check, repeat, define, give back, add, lists |
| **Working** | ~150 words | text, files, dates, sorting, failure handling, modules |
| **Full** | 400+ | drawing, networking, processes, binary data, concurrency |

A beginner is **complete** at tier 1 — real programs are writable having learned forty words. Since
your own definitions become vocabulary too, tier 1 plus your own words genuinely suffices.

### 4.3 The ambiguity check — the part that usually goes wrong

At scale, phrases start colliding: two rows that could both match the same sentence, and now the
language guesses. That is how a friendly language becomes an unpredictable one.

**`polite check-vocabulary` runs on every build**, cross-checks every phrase against every other,
and **fails the build** on an ambiguous pattern. A bad row is caught the moment it is added, not
six months later when somebody's program does something baffling.

This is cheap to build early and near-impossible to retrofit.

### 4.4 Discovery

Discovery is a command, not a manual:

```
polite words
polite words about lists
polite explain "repeat until"
```

`polite explain` gives the **English** meaning alongside the code meaning, serving purpose 1.

### 4.5 Extension

Libraries are vocabulary packs. Words are namespaced to the pack they came from, so two libraries
may both add `draw` without conflict (resolution in section 5).

---

## 5. Modules and imports

```polite
please use the drawing kit
please borrow the drawing kit          -- identical; borrow is the more mannerly form

please use helpers                     -- finds helpers.polite beside this file
please use "tools/shapes.polite"       -- or an explicit path

please use draw and fill from the drawing kit    -- only the named words
```

**A file shares nothing by default.** Sharing is explicit — both safer and literally the manner of
offering something rather than leaving your things lying around:

```polite
-- helpers.polite

please define greet with a name:
    give back "hello, {name}!"
thanks

please define wave:
    show "*waves*"
thanks

please share greet and wave
```

**Collisions are never guessed at.** The language asks, and the answer is plain English:

```
Both the drawing kit and the map kit offer `draw`.
Which did you mean? You can say
`draw from the drawing kit` to be specific.
```

```polite
please draw a circle from the drawing kit
```

**Circular imports** (A uses B, B uses A) are detected before running, and the loop is drawn out
rather than producing a stack overflow.

**Deferred:** `polite add <kit>` for installing third-party vocabulary packs. A package ecosystem
is meaningless before the language exists; this is a separate sub-project (section 13).

---

## 6. Types

### 6.1 The kinds of things

```
whole number · decimal number · text · yes or no · list · lookup · nothing · action
```

Booleans are literally `yes` and `no`, because that is what they are.

User-defined kinds are described in English:

```polite
please describe a player with a name, a score and lives
```

### 6.2 Inference — you never write a type

Types are fully inferred (Hindley–Milner style, union-find with path compression). **No type
annotations exist in version 1.** The language works types out, checks them *before* the program
runs, and only speaks up when the code genuinely does not say enough:

```
I cannot work out what `total` holds — nothing you
do with it tells me. Could you show me what goes
into it first?
```

Inference reaches through I/O. In `ask "What is your guess? " as guess`, if `guess` is later
compared to a number, `ask` reads a number and handles the conversion itself; a non-numeric reply
produces a gentle re-ask at runtime rather than a crash.

Inference is also what makes the runtime fast — see section 10.2. It is a beginner-experience
feature and a performance feature at the same time.

---

## 7. Things that might not work out

### 7.1 The problem

Some operations genuinely can fail: opening a missing file, turning `"banana"` into a number,
asking for item 99 of a 3-item list, dividing by zero, reaching the network.

Existing languages handle this in one of two bad ways:

- **Crash** — hostile, and fixing it requires learning *exceptions*.
- **Silently substitute null / 0 / empty** — no crash, but the program is now quietly **wrong**,
  which is far worse for a learner. It "works", the score is 0 forever, and there is no clue why.

PoliteLang refuses both:

> **If something might not work out, you must say what happens if it does not.** The language will
> never crash on you, and it will never quietly invent an answer.

### 7.2 Three ways to say it

**1. A fallback — one word:**

```polite
remember save is the contents of "save.txt" or ""
remember n is the number in reply or 0
remember first is item 1 of names or "nobody"
```

This is the everyday case and costs two words. It reads aloud correctly.

**2. Handle it properly**, when there is more to do than substitute a value:

```polite
please try to:
    open "save.txt"
    read the score from it
otherwise if it does not work out:
    show "Could not load your save: {what went wrong}"
    remember score is 0
thanks
```

`{what went wrong}` is plain text. No exception objects, no error classes, nothing to learn.

**3. Take responsibility**, when you are certain:

```polite
remember config is the contents of "settings.polite", I am sure
```

If you are wrong, the program stops — politely, and holding you to your word:

```
You said you were sure "settings.polite" would be there,
but I could not find it. Stopping here rather than
guessing. Line 4.
```

This is the correct manners lesson: you may vouch for something, and if you do, you own it.

### 7.3 Checking instead

```polite
check if save has something:
    show save
otherwise:
    show "Nothing saved yet."
thanks
```

Inside the first branch the checker **knows** `save` definitely has something (flow-sensitive
narrowing), so it is used freely with no further ceremony and never re-checked.

### 7.4 It propagates automatically

An action that does something risky without handling it **becomes risky itself**, with no
annotation written by anyone:

```polite
please define load the score:
    give back the number in the contents of "save.txt"
thanks
```

```
`load the score` might not work out, because opening
"save.txt" might not. That is fine — I just wanted you
to know, so anyone using it can plan for it.
```

Callers get the same three choices. Safety travels the whole way up because inference is already
doing the work.

### 7.5 Which operations are risky

A **flag on the vocabulary table row** (section 4.1). The list is therefore closed, generated into
the documentation automatically, and identical across every backend.

### 7.6 Honest cost

Code using many risky operations needs many `or`s. Three things make that acceptable: it is one
word, the compiler always shows the exact line to write, and in exchange **the single most common
crash in every mainstream language becomes impossible** — without anybody learning the words
*null*, *exception*, or *Option*.

---

## 8. Diagnostics

Error messages are the product, not an afterthought. Five rules, enforced on every message:

1. Plain English sentences; no jargon without explaining it.
2. **Never blame.** "I cannot…", never "you did X wrong", never *illegal* / *invalid* / *fatal*.
3. Show the actual line.
4. Suggest a fix.
5. Explain *why*, so the reader learns something.

```
In game.polite, line 12:

    please add "hello" to score

I cannot add text to a number. `score` is a whole number —
you set it to 0 back on line 3 — and "hello" is text.
Did you mean to show them together instead?

    please show "hello" and score
```

Compare `TypeError: unsupported operand type(s) for +=: 'int' and 'str'`. Same bug; one of the two
teaches English and the fix.

---

## 9. Compiler architecture

### 9.1 Chosen approach

Source is lowered to a small middle language (**PoliteIR**) before execution or code generation.
Backends consume PoliteIR, never the sentence tree.

The alternative — walking the sentence tree directly — was rejected. With V vocabulary forms and N
backends, tree-walking costs **V × N** (every backend must understand every phrasing), whereas
lowering costs **V + N**. With one backend those are equal; with a large vocabulary and several
backends they are not remotely equal, and "large vocabulary, many backends" is exactly this
project's goal. The extra cost is defining the IR and the lowering step — roughly one to two
weeks, paid once.

### 9.2 Crates

```
polite-vocab    the phrase table, its loader, the ambiguity checker
polite-syntax   lexer + parser -> sentence tree  (driven by the table)
polite-check    names, type inference, might-not-work-out analysis
polite-ir       PoliteIR definition + lowering from the checked tree
polite-run      the reference runner                     <- backend #1
polite-diag     gentle-message machinery
polite-std      the standard library
polite-cli      the polite binary
```

Later, and only later: `polite-native` (Cranelift), `polite-jvm`, `polite-wasm`.

The split is not tidiness — on 2 cores it is what keeps rebuilds fast, since editing one crate does
not recompile the others.

### 9.3 PoliteIR

Small, boring, regular: roughly 25–30 instructions, arranged in **basic blocks with numbered
slots**, with types attached to every value.

```
-- please repeat 5 times: show "hi" thanks

block start:
    %1 = const 0
    jump loop

block loop:
    %2 = lt %1, 5
    branch %2, body, done

block body:
    call show, "hi"
    %1 = add %1, 1
    jump loop

block done:
    return
```

All six loop phrasings from section 4.1 produce exactly this. That is V + N made real.

**Why blocks-and-slots rather than a stack machine:** Cranelift (native) wants blocks and values;
the JVM wants a stack. Blocks-and-slots converts cleanly to **both**. A stack IR would have been
easier for the JVM and awkward for native. This choice keeps both doors open.

### 9.4 The backend socket

```rust
trait Backend {
    fn name(&self) -> &str;
    fn emit(&mut self, program: &ir::Program) -> Result<Output, Diagnostic>;
}
```

This trait is the "many backends" promise made concrete. Nothing above `polite-ir` knows a backend
exists.

### 9.5 Build-time discipline

- **Hand-written lexer and recursive-descent parser.** No parser-generator crates — they are
  procedural macros, the single slowest thing in Rust compilation.
- **Near-zero dependencies.** No `serde`, no `toml` (section 4.1), no `regex`, no async runtime.
- **Cranelift is not added until the native backend phase**, so it never slows daily work.
- **`cargo check` as the inner loop**; full builds only when running something.
- **`debug = "line-tables-only"`** in the dev profile — meaningfully less RAM while linking, which
  matters at 3.8 GB.
- **`rust-lld` as the linker** where the toolchain provides it; it links substantially faster than
  the default on Windows.

---

## 10. Optimization

The constrained laptop is **the target machine**, not an obstacle. A language for beginners that
needs 8 GB and a fast SSD is self-defeating: beginners are the people on school laptops. If
PoliteLang is comfortable here, it is comfortable everywhere.

### 10.1 Compiler

- **Handles, not pointers.** Every tree and IR node lives in a flat array, referenced by a 32-bit
  index. No `Box`, no `Rc`, no scattered object graph. Worth roughly 3–5× in both memory and speed
  versus the obvious design, because nodes sit adjacent in cache.
- **Every name becomes a number, once.** Identifiers are interned on first sight and compared as
  `u32` thereafter. Comparing words is the most common thing a compiler does.
- **Positions are byte offsets**, not line/column. One `u32`; line and column are computed only
  when a message is actually printed.
- **Diagnostics are lazy.** A message is a small structured record, not a string. Code that
  compiles pays nothing for the elaborate machinery of section 8.
- **Vocabulary matching is a lookup, not a search.** The phrase table is compiled into a static
  jump structure keyed on the first word, generated at build time. At runtime: hash the first
  word, check a handful of candidates. **Matching cost does not grow with vocabulary size** —
  exactly what a language designed to grow to thousands of words requires.
- **Slow eMMC is respected.** Parsed-and-checked results are cached per file, keyed by content
  hash. Re-running a program whose imports are unchanged touches almost no disk.
- **Startup is a feature.** Beginners iterate by running constantly; a language that takes half a
  second to say hello feels broken to them even when it is not.

### 10.2 Runtime

Because types are fully inferred, **the interpreter never checks a type at runtime**:

```
please add 1 to score      ->  add.int %3, 1
please add ", " to name    ->  concat.text %7, %8
```

The decision is made once, at lowering. A dynamic language makes it every time the line runs; in a
million-iteration loop, that is a million decisions never made.

- **Numbers are unboxed.** A slot known to hold a whole number is a raw 64-bit value in a flat
  array — no allocation, no tag, no unwrapping.
- **Register-based IR** needs roughly half the dispatches of a stack machine for the same work.
- **Instructions are small and fixed-size**, keeping the instruction stream in cache.
- **Text is immutable, refcounted, with short strings stored inline** (up to ~22 bytes, no
  allocation). Beginner programs are mostly short strings.
- **Reference counting, not garbage collection.** Lower memory ceiling, no pause spikes, much
  smaller runtime. **Honest caveat:** plain refcounting leaks cycles. A cycle collector is a later
  addition; beginner programs essentially never build cycles, and shipping small and correct beats
  shipping large and clever.
- **Lists grow by 1.5×, not 2×.** Peak memory is what kills you at 3.8 GB.

### 10.3 The optimization that teaches nothing

Immutable text makes building a string in a loop accidentally O(n²) — the classic beginner
performance trap in every language:

```polite
please repeat for every word in words:
    add word to sentence
thanks
```

The compiler **recognises this shape and rewrites it** into a builder. The beginner learns
nothing, changes nothing, and the program is fast. This is principle 5.

Because these passes — constant folding, dead code removal, this rewrite, peephole cleanups — run
**on the IR**, every future backend inherits every one of them for free. V + N again.

### 10.4 Budgets

Requirements, measured on the reference machine (2 cores, 3.8 GB, eMMC, Windows 11):

| | Budget |
|---|---|
| Compile + run `hello`, excluding process spawn | **< 3 ms** |
| `polite run hello.polite` wall clock, warm | **< 15 ms** |
| …cold from eMMC, first run of the day | **< 120 ms** |
| Checking throughput | **>= 150,000 lines/sec** |
| Check a 1,000-line program | **< 10 ms** |
| Compiler peak memory, 5,000 lines | **< 20 MB** (about 2 KB/line) |
| Interpreter vs CPython, whole corpus | **faster overall** |
| Interpreter vs CPython, numeric loops | **>= 2x faster** |
| `polite run hello` resident memory | **< 6 MB** |
| LSP: keystroke -> live errors, 1,000-line file | **< 50 ms** |
| LSP resident memory, idle | **< 50 MB** |
| `polite` binary, stripped + LTO | **< 3 MB** |
| `cargo check`, one-crate edit | **< 5 s** |
| Full cold `cargo build` | **< 90 s** |
| 10x the vocabulary -> parse slowdown | **< 5%** |

**The two that matter most.**

*The LSP budgets*, because they are felt. VS Code alone takes 500 MB–1 GB on this machine. A
language server sitting at 300 MB — completely normal; rust-analyzer routinely does far worse —
makes the editor unusable here. 50 MB resident and 50 ms to redraw errors is the difference
between PoliteLang being pleasant to write on a school laptop and being theoretical.

*The 10x vocabulary test*, because it tests a **claim** rather than a speed. Section 4 asserts a
large vocabulary is cheap. That assertion is either true or the project's central design bet is
wrong. The benchmark generates thousands of synthetic phrases and proves parsing did not slow
down. If it ever fails, that must be known immediately.

**Enforcement.** CI cannot run on this laptop, so cloud-runner numbers would be meaningless.
`polite bench` writes a baseline file, committed to the repository, measured **here**, and fails on
more than a 10% regression. Budgets are checked against the real target machine or not at all.

---

## 11. Testing

Four suites, mostly generated:

1. **Vocabulary round-trip** — every table row gets a generated parse test. 400 words, 400 tests,
   nobody writes them.
2. **Ambiguity check** — runs on every build; fails it if two phrases could match one sentence.
3. **Program corpus** — `tests/programs/*.polite` with expected output.
4. **Message snapshots** — `tests/errors/*.polite`, each pinned to its exact expected message text.
   This is the suite that matters most: section 8 claims the messages *are* the product, and this
   is what stops them rotting into jargon. Accompanied by a **lint that fails the build** if any
   message contains *illegal*, *invalid*, or *fatal* (principle 3).

Suite 3 doubles as the **backend conformance suite**: every backend must produce byte-identical
output across the whole corpus, or it is not finished.

---

## 12. VS Code extension

- **Highlighting** — generated from the vocabulary table. Always in sync, never hand-edited.
- **Live errors and completion** — `polite lsp` is a thin wrapper around `polite-check`, the same
  code that already produces the gentle messages of section 8, streamed into the editor.

The extension is therefore mostly wiring, not a second implementation.

---

## 13. Scope

### 13.1 Version 1 — this sub-project

Done means real programs are writable:

- The four structural rules; tier-1 vocabulary plus a substantial part of tier-2
- Type inference; the `or` safety analysis; imports
- PoliteIR and the reference runner
- `polite run`, `polite check`, `polite words`, `polite explain`, `polite bench`,
  `polite check-vocabulary`
- Standard library: text, numbers, lists, lookups, files, time, random
- VS Code extension
- All four test suites and the benchmark baseline

### 13.2 Later sub-projects

Each is a separate spec, plan and implementation cycle, plugging into a socket and a conformance
corpus that already exist:

2. **Native backend** — Cranelift. The route toward genuinely low-level work.
3. **JVM backend** — the realistic route to Minecraft plugins.
4. **WASM backend** — an in-browser playground, so anyone can try the language with no install.
5. **Package manager** — `polite add <kit>`, third-party vocabulary packs.
6. **Cycle collector** — closing the refcounting caveat in section 10.2.

### 13.3 Non-goals for version 1

- No type annotations (inference only).
- No concurrency.
- No foreign-function interface. PoliteLang is independent: its own vocabulary, its own standard
  library, borrowing nothing.
- No package ecosystem.
- No self-hosting.

---

## 14. Deferred decisions

Each carries the rule by which it will be decided, so none of these is an open question in
disguise.

1. **Cycle collection strategy.** Deferred to sub-project 6. Decided when a real program
   demonstrably leaks; measured, not guessed.
2. **Concurrency vocabulary.** Deferred past version 1. Decided against the read-aloud test first,
   performance second — if concurrency cannot be phrased so a non-programmer understands it read
   aloud, it does not enter the language.
3. **Exact tier-2 / tier-3 word split.** Decided empirically: a word moves to tier 1 when the
   corpus shows beginners reaching for it.
4. **Whether `decimal number` and `whole number` are visibly distinct to beginners.** Both exist in
   the type system for the sake of the native and JVM backends; whether diagnostics say "number"
   instead is decided by message-snapshot review, once real messages exist.
