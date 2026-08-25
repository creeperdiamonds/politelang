// The `.env` reader in the JavaScript runtime, against the same cases the Rust one is tested on.
//
// There are two of these readers, one in each backend, and the whole point of them is that a
// program cannot tell which it is running under. They part company quietly -- somebody fixes a
// corner in one and not the other -- so the cases in `crates/polite-std/src/secrets.rs` are
// repeated here, and both have to agree.
//
//     node tools/check-dotenv.mjs

import { readEnvFile } from "../runtime/polite.mjs";

const NL = String.fromCharCode(10);
const BS = String.fromCharCode(92);

const cases = [
  {
    what: "the plain shape is read",
    text: `DISCORD_TOKEN=abc123${NL}OTHER=hello${NL}`,
    wanted: { DISCORD_TOKEN: "abc123", OTHER: "hello" },
  },
  {
    what: "remarks and blank lines are passed over",
    text: `# a remark${NL}${NL}  # an indented one${NL}A=1${NL}`,
    wanted: { A: "1" },
  },
  {
    what: "export in front is allowed because people write it",
    text: `export A=1${NL}export   B=2${NL}`,
    wanted: { A: "1", B: "2" },
  },
  {
    what: "a quoted value keeps everything inside the quotes",
    text: `A="spaces and a # inside"${NL}B='single quoted'${NL}`,
    wanted: { A: "spaces and a # inside", B: "single quoted" },
  },
  {
    what: "an unquoted value stops at a remark and is trimmed",
    text: `A=value   # what it is for${NL}B=  spaced  ${NL}`,
    wanted: { A: "value", B: "spaced" },
  },
  {
    what: "escapes work inside double quotes and not outside them",
    text: `A="one${BS}ntwo"${NL}B=one${BS}ntwo${NL}C='one${BS}ntwo'${NL}`,
    wanted: { A: `one${NL}two`, B: `one${BS}ntwo`, C: `one${BS}ntwo` },
  },
  {
    what: "a value may itself contain an equals sign",
    text: `A=abc=def==${NL}`,
    wanted: { A: "abc=def==" },
  },
  {
    what: "nonsense lines are ignored rather than guessed at",
    text: `no equals sign here${NL}=novalue${NL}BAD NAME=1${NL}GOOD=1${NL}`,
    wanted: { GOOD: "1" },
  },
  {
    what: "an empty value is still a value",
    text: `A=${NL}B=""${NL}`,
    wanted: { A: "", B: "" },
  },
  {
    what: "nothing is substituted into anything else",
    text: `A=one${NL}B=$A/two${NL}`,
    wanted: { A: "one", B: "$A/two" },
  },
];

let wrong = 0;
for (const { what, text, wanted } of cases) {
  const got = Object.fromEntries(readEnvFile(text));
  const same =
    Object.keys(got).length === Object.keys(wanted).length &&
    Object.entries(wanted).every(([k, v]) => got[k] === v);
  if (!same) {
    wrong++;
    console.log(`  ${what}`);
    console.log(`      wanted ${JSON.stringify(wanted)}`);
    console.log(`      got    ${JSON.stringify(got)}`);
  }
}

if (wrong) {
  console.log(`${wrong} of ${cases.length} cases disagree with the reference runner.`);
  process.exit(1);
}
console.log(`all ${cases.length} cases read the same way in both backends`);
