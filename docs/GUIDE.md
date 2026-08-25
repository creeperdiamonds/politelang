# Learning PoliteLang

**A programming language you ask nicely.**

This guide starts from nothing at all. You do not need to have programmed before, and there is
nothing to memorise before you begin — you already speak English, and you already know how to ask
for something politely. That is the whole entry cost.

Work through it in order. Every lesson is short, every lesson ends with something to try yourself,
and every program in here really runs.

---

## Before you start

Build the language once:

```
cd politelang
cargo build --release
```

Then run any program like this:

```
./target/release/polite run examples/guide/01-hello.polite
```

Two commands are worth knowing straight away, because they mean you never have to remember
anything:

```
polite words                    every word the language knows
polite explain "repeat until"   what a word means, in English
```

---

## 1. Saying hello

Make a file called `hello.polite` and put one line in it:

```polite
please show "Hello."
```

Run it. It says:

```
Hello.
```

That is a complete program. Three things are going on:

- **`please`** opens a request. Every request that *does* something starts with a courtesy word.
- **`show`** is the request — it puts something on the screen.
- **`"Hello."`** is text. Text goes between quote marks.

You do not have to say `please` if it feels too pleading. All of these mean **exactly** the same
thing, and the language never treats one differently from another:

```polite
please show "Hello."
kindly show "Hello."
would you show "Hello."
would you please show "Hello."
if you would show "Hello."
if you would be so kind show "Hello."
if it is not too much trouble show "Hello."
```

Pick whichever you like. Mix them. It genuinely does not matter.

> **Try it.** Write a program that says your name, then says what you had for breakfast.

---

## 2. Being rude

Leave the `please` off:

```polite
show "Hello."
```

The language does not shout at you:

```
In hello.polite, line 1:

    show "Hello."
    ^^^^

This is missing a `please`.

Out here at the top of the file, every request asks for itself.
Inside a block you only have to ask once, when you open it.

Ask, and I will happily do it:

    please show ...
```

That is what every message in PoliteLang is like. It says what happened, shows you the line,
suggests a fix, and explains why. It never blames you, and it never uses a word it has not
explained.

You will see a lot of these. They are the friendliest part of the language, not the scary part.

---

## 3. Remembering things

A program that only says things is not much use. Give a value a name:

```polite
please remember name is "Ada"
please remember age is 11

please show name
please show age
```

```
Ada
11
```

To **change** something you already remembered, use a different word:

```polite
please remember score is 0
please change score to 10
please show score
```

`remember` introduces a name. `change` updates one that already exists. They are deliberately
different words, so that if you mistype a name the language notices instead of quietly making a
second thing:

```polite
please remember score is 0
please change scoer to 10
```

```
I do not know a name called `scoer`.

Nothing here has been remembered under that name. There is a `score`
though, which is very close.

Did you mean:

    score
```

Two shortcuts you will use constantly:

```polite
please add 1 to score
please take 1 from score
```

> **Try it.** Remember a number, add five to it, take two away, and show it. What do you get?

---

## 4. Putting values inside text

You often want to show a value *in the middle of* a sentence. Put it in braces:

```polite
please remember name is "Ada"
please remember age is 11

please show "{name} is {age} years old."
please show "Next year she will be {age plus 1}."
```

```
Ada is 11 years old.
Next year she will be 12.
```

Anything that is a value anywhere is a value inside braces — not just a name.

---

## 5. Asking a question

```polite
please ask "What is your name? " as name
please show "Hello, {name}."
```

The program waits for you to type something, then carries on.

Here is the nice part. You never say what *kind* of answer you want:

```polite
please ask "How old are you? " as age

please check if age is over 10:
    show "You are older than ten."
thanks
```

Because you compared `age` to a number, the language works out that the answer should be read as a
number. If somebody types "banana", it asks again politely rather than falling over.

---

## 6. Choosing

```polite
please remember score is 7

please check if score is over 10:
    show "That is a lot."
otherwise if score is over 5:
    show "That is respectable."
otherwise:
    show "Early days."
thank you for checking
```

```
That is respectable.
```

Three new things:

- A **colon** at the end of a line says *here comes a block*, the way it does in ordinary writing.
- **`thank you`** closes the block. You can also write just `thanks`.
- Notice the lines inside the block have **no `please`**. You asked nicely once, when you opened
  the block. You do not have to keep asking.

You may say what you are closing, and the language checks it:

```polite
thank you for checking
thank you for repeating
thank you for defining
thank you for trying
```

Get it wrong and it tells you — which catches crossed-over blocks before they confuse you.

**Indentation does not matter.** It is there to help you read, and nothing else. Paste code in
badly indented and it still runs exactly the same. This is on purpose: it removes a whole category
of thing that goes wrong for beginners in other languages.

Ways to ask a question:

```
score is 10             score is not 10
score is over 10        score is under 10
score is at least 10    score is at most 10
score is between 1 and 10
```

And you can join them with `and`, `or` and `not`.

> **Try it.** Ask somebody their age, then say whether they can vote.

---

## 7. Repeating

There are several ways to say this, because there are several ways to say it in English. They all
do the same thing underneath.

```polite
please repeat 3 times:
    show "again"
thanks
```

```polite
please repeat for every n from 1 to 5:
    show n
thanks
```

```polite
please remember n is 0
please repeat while n is under 3:
    add 1 to n
    show n
thanks
```

```polite
please repeat until score is over 100:
    add 10 to score
thanks
```

```polite
please keep going forever:
    show "once"
    stop repeating
thanks
```

Two words for stepping out of a loop:

- **`stop repeating`** leaves the loop entirely.
- **`skip to the next one`** abandons this turn and starts the next.

```polite
please repeat for every n from 1 to 5:
    check if n is 3:
        skip to the next one
    thanks
    show n
thanks
```

```
1
2
4
5
```

> **Try it.** Show the two times table, from 2 to 24.

---

## 8. Lists

A list is a row of things kept in order.

```polite
please remember shopping is an empty list
please add "bread" to shopping
please add "apples" to shopping
please add "tea" to shopping

please show "There are {the number of items in shopping} things to buy."

please for every thing in shopping:
    show "  - {thing}"
thanks
```

```
There are 3 things to buy.
  - bread
  - apples
  - tea
```

Positions count from **1**, the way people count:

```polite
please show item 1 of shopping or "nothing"
```

That `or "nothing"` matters, and lesson 11 explains why. For now: it is what to say if there is no
first item, because the list might be empty.

Some things you can ask of a list:

```polite
the number of items in shopping
the first item of shopping or "nothing"
the last item of shopping or "nothing"
the sorted version of shopping
the reverse of shopping
the join of shopping with ", "
shopping contains "tea"
the rest of shopping
the total of numbers
the average of numbers or 0
```

> **Try it.** Make a list of five numbers, then show them in order, largest last, and show their
> total.

---

## 9. Lookups

A lookup finds things by name, the way a dictionary finds a meaning by its word.

```polite
please remember ages is an empty lookup
please put 11 for "ada" in ages
please put 42 for "grace" in ages

please show the value for "ada" in ages or 0
please show ages knows about "nobody"
please show the join of the keys of ages with ", "
```

```
11
no
ada, grace
```

---

## 10. Teaching the language your own words

This is where it gets good. When you define an action, it becomes a word you can use:

```polite
please define greet with a name:
    give back "Hello, {name}."
thank you for defining

please show greet with "Ada"
```

```
Hello, Ada.
```

- **`define`** teaches the language a word.
- **`with a name`** says what it needs.
- **`give back`** is how it answers.
- Then you use it like any other word: `greet with "Ada"`.

Two values? Separate them with `and`:

```polite
please define greet with a name and a greeting:
    give back "{greeting}, {name}!"
thanks

please show greet with "Ada" and "Good morning"
```

An action's name can be several words long, which often reads better:

```polite
please define count down from with a number:
    check if number is at most 0:
        give back "done"
    thanks
    show number
    give back count down from with number minus 1
thanks

please show count down from with 3
```

```
3
2
1
done
```

An action may call itself, as that one does. And you can use an action before you have defined it
— the language reads the whole file first.

> **Try it.** Write an action called `is even` that gives back yes or no, and use it in a loop.

---

## 11. When things might not work out

Some things genuinely can fail. A file might not be there. Text might not be a number. A list
might be too short.

Other languages handle this in one of two bad ways: they **crash** at you, or they quietly make
something up so your program is wrong and you never find out.

PoliteLang refuses both:

> **If something might not work out, you must say what happens if it doesn't.** The language will
> never crash on you, and it will never quietly invent an answer.

There are three ways to say it. Use whichever fits.

### One: give it something to fall back on

```polite
please remember saved is the contents of "save.txt" or "nothing saved yet"
please show saved
```

Two extra words. This is the everyday case, and it reads aloud correctly: *remember saved is the
contents of save.txt, or nothing saved yet*.

### Two: handle it properly

When there is more to do than substitute a value:

```polite
please try to:
    remember settings is the contents of "settings.polite"
    show "The settings are {the length of settings} letters long."
otherwise if it does not work out:
    show "I could not read them, because {what went wrong}."
thank you for trying
```

`{what went wrong}` is plain text explaining the reason. There is nothing to learn about it.

### Three: take responsibility

When you are certain:

```polite
please remember config is the contents of "settings.polite", I am sure
```

If you are wrong, the program stops — politely, and holding you to your word:

```
You said you were sure about this, but there is no file called
"settings.polite". Stopping here rather than guessing.
```

### It spreads by itself

If an action of yours does something risky and does not handle it, that action becomes risky too,
and the language mentions it:

```
Just so you know, in game.polite, line 4:

    give back the number in the contents of "score.txt"
              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

`load the score` might not work out, because `the number in {value}`
might not.

That is perfectly fine. I am telling you so that whoever uses this
action can plan for it.
```

You wrote nothing to make that happen. Whoever uses your action gets the same three choices.

### It only asks when it has to

`1 over 3` can no more fail than `1 plus 3` can. Where the language can see the answer, it stays
quiet — and if you write a fallback that could never be reached, it says so kindly rather than
making a fuss.

**What you get for this:** the single most common crash in every mainstream language simply cannot
happen in a PoliteLang program. And you never had to learn the words *null*, *exception* or
*Option* to get it.

> **Try it.** Ask somebody for a number, and cope politely with them typing a word instead.

---

## 12. Borrowing between files

Put words you want to reuse in their own file, and offer them:

```polite
-- parts/greetings.polite

please define greet with a name:
    give back "Hello, {name}."
thanks

please share greet
```

Then borrow it:

```polite
please use "parts/greetings.polite"

please show greet with "Ada"
```

`borrow` means exactly the same as `use`, and is the more mannerly of the two.

**A file keeps everything to itself unless it shares it.** That is safer, and it is also just the
manner of offering something rather than leaving your things lying about. Anything not shared
stays private, and the language will say so if you reach for it.

---

## 13. Numbers, as far as you want to go

Everyday arithmetic, in words or in symbols — both mean the same:

```polite
please show 7 plus 3          -- or 7 + 3
please show 7 minus 3
please show 7 times 3
please show 7 divided by 2    -- 3.5
```

**Whole numbers have no limit.** Nothing wraps round to a wrong answer:

```polite
please remember f is 1
please repeat for every n from 1 to 100:
    multiply f by n
thanks
please show f
```

That prints all 158 digits of a hundred factorial, exactly.

**Fractions are exact**, which decimals never quite are:

```polite
please show 1 over 3 plus 1 over 3 plus 1 over 3   -- 1, exactly
please show 1 over 2 plus 1 over 3                 -- 5/6
please show 0.75 as a fraction                     -- 3/4
```

And when you need them, everything else is here too — trigonometry, logarithms, primes, factors,
combinations, number bases, bits, statistics, vectors and matrices, and complex numbers:

```polite
please show the square root of 144
please show the sine of the number pi divided by 2
please show 97 is prime
please show the ways to choose 2 from 5 or 0
please show 255 in hexadecimal
please show the median of marks or 0
please show the dot product of a and b or 0
please show 3 plus the imaginary number 4
```

You do not need any of this to start. It is here for when you do.

---

## 14. Finding things out

You will never need to memorise the vocabulary, because you can ask:

```
polite words                       everything, grouped from everyday to specialist
polite words about lists           only the ones about lists
polite explain "repeat until"      what one means, in English
polite explain borrow
```

`polite explain` tells you what a word means in **English** first, and only then what it does. That
is deliberate — one of the reasons this language exists is so that you come away knowing more
English than you started with.

And to look a program over without running it:

```
polite check mygame.polite
```

---

## The whole language on one page

**Four rules, and that is all the structure there is.**

1. **`please` opens a request.** `kindly`, `would you`, `if you would be so kind` and friends all
   mean the same.
2. **`thank you` closes a block.** `thanks` is the same word, shorter. It may say what it closes.
3. **One courtesy word covers everything inside the block it opened.** Being extra polite inside is
   never a problem.
4. **Words first, symbols allowed.** `2 plus 3` and `2 + 3` are the same thing.

**Everything else:**

```polite
-- a comment
by the way, this is a comment too

please remember name is value          -- introduce a name
please change name to value            -- update one
please add 1 to name                   -- and take from, multiply by, divide by

please show value
please ask "prompt" as name

please check if condition:
    ...
otherwise if condition:
    ...
otherwise:
    ...
thanks

please repeat 3 times:                 -- also: repeat while, repeat until,
    ...                                -- keep going forever, for every x in list,
thanks                                 -- repeat for every n from 1 to 10

    stop repeating
    skip to the next one

please define name with a thing and another:
    give back value
thanks

please try to:
    ...
otherwise if it does not work out:
    show "{what went wrong}"
thanks

please use "otherfile.polite"
please share name and othername

value or fallback                      -- if it might not work out
value, I am sure                       -- if you are certain
```

---

## Where to go next

- Read the programs in `examples/` — they are all short and all run.
- Read `tests/programs/` if you want to see every corner of the language exercised.
- `docs/superpowers/specs/` has the design document, if you want to know *why* rather than *what*.

And when something goes wrong, read the message. It was written for you.

---

*PoliteLang is by Creeperdiamonds Studios.*
