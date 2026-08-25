# PoliteLang for VS Code

Syntax highlighting, snippets and sensible editing behaviour for `.polite` files.

## What it gives you

- **Highlighting** for every word in the language. The grammar is *generated* from
  `vocabulary/core.vocab` by `polite grammar`, so it can never drift out of date — adding a word to
  the language colours it here without anybody editing a grammar file.
- **Snippets** for the everyday shapes: `show`, `remember`, `ask`, `repeat`, `every`, `check`,
  `define`, `try`, `or`, `please`.
- **Indentation that follows the blocks**: a line ending in `:` indents, and `thanks`,
  `thank you` and `otherwise` come back out. Indentation in PoliteLang is only ever cosmetic, so
  this is a convenience and never something that can break your program.
- **Comments** with `--`.

## Installing it while developing

Copy or link this folder into your VS Code extensions folder and restart the editor:

```
Windows   %USERPROFILE%\.vscode\extensions\politelang
macOS     ~/.vscode/extensions/politelang
Linux     ~/.vscode/extensions/politelang
```

Then open any `.polite` file.

## Keeping the grammar current

After changing `vocabulary/core.vocab`:

```
polite grammar
```

That rewrites `syntaxes/politelang.tmLanguage.json`. A test in the repository fails if the shipped
grammar does not match the table, so this cannot be forgotten quietly.

## Still to come

Live errors and completion, by way of `polite lsp` — a thin wrapper around the same checker that
produces the messages you see in the terminal. The design for it is in the spec; the work is not
done yet.

---

PoliteLang is by Creeperdiamonds Studios.
