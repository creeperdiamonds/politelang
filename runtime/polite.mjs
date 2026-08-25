// The PoliteLang runtime, in JavaScript.
//
// Everything a PoliteLang program can ask for, on the other side of the middle language. The
// emitted program calls into here and nowhere else, so this file and `crates/polite-js` are the
// two halves of one thing: a builtin exists in both or in neither.
//
// Conventions, once, so the rest of the file can be short:
//
//   a list      a JavaScript array
//   a lookup    a Map, which unlike an object keeps the order things were put in
//   text        a string
//   nothing     null
//   yes / no    true / false
//   a number    a JavaScript number
//
// Counting starts at one, everywhere, as it does in the language itself.
//
// Two departures from the reference runner, both of them JavaScript's doing and neither of them
// hidden: a whole number beyond about nine thousand million million loses exactness, where the
// reference runner keeps it exact forever; and a decimal that happens to hold a whole value prints
// as `7` rather than `7.0`, because JavaScript has one kind of number and does not remember which
// kind it was.

import * as fs from "node:fs";
import * as readline from "node:readline";

// ---------------------------------------------------------------------------
// Things not working out
// ---------------------------------------------------------------------------

/// Something that might not work out, and did not. This is what `or` and `try to` catch.
export class Politely extends Error {}

/// Ending the whole program. No `try` catches this, which is the point of it.
export class Finished extends Error {
  constructor(message, quiet) {
    super(message);
    this.quiet = quiet === true;
  }
}

function nope(message) {
  throw new Politely(message);
}

function notHere(what) {
  throw new Politely(
    `${what} is not in the JavaScript backend yet, so this program cannot run here. ` +
      `It works when run the ordinary way, with \`polite run\`.`
  );
}

export function stopEverything() {
  throw new Finished("", true);
}

export function wasNotSure(reason, what) {
  throw new Finished(
    `You said you were sure about ${what}, and it did not work out: ${reason}`
  );
}

export function cannotOrder() {
  nope("two lists cannot be put in order against each other");
}

// ---------------------------------------------------------------------------
// Showing
// ---------------------------------------------------------------------------

export function showable(v) {
  if (v === null || v === undefined) return "nothing";
  if (typeof v === "boolean") return v ? "yes" : "no";
  if (typeof v === "number") return numberText(v);
  if (typeof v === "string") return v;
  if (Array.isArray(v)) return "[" + v.map(showable).join(", ") + "]";
  if (v instanceof Map) {
    const inner = [];
    for (const k of sortedKeys(v)) inner.push(`${k}: ${showable(v.get(k))}`);
    return "{" + inner.join(", ") + "}";
  }
  return String(v);
}

function numberText(v) {
  if (Number.isNaN(v)) return "not a number";
  if (v === Infinity) return "endlessly big";
  if (v === -Infinity) return "endlessly small";
  return String(v);
}

export function show(v) {
  console.log(showable(v));
}

// ---------------------------------------------------------------------------
// Asking
// ---------------------------------------------------------------------------

let lines = null;
let waiting = [];
let finishedReading = false;

function startReading() {
  if (lines) return;
  lines = [];
  const reader = readline.createInterface({ input: process.stdin, terminal: false });
  reader.on("line", (line) => {
    const next = waiting.shift();
    if (next) next(line);
    else lines.push(line);
  });
  reader.on("close", () => {
    finishedReading = true;
    while (waiting.length) waiting.shift()(null);
  });
}

function nextLine() {
  startReading();
  if (lines.length) return Promise.resolve(lines.shift());
  if (finishedReading) return Promise.resolve(null);
  return new Promise((resolve) => waiting.push(resolve));
}

/// Ask, and keep asking until the answer is the kind of thing that was wanted.
///
/// Running out of input is not an answer. A program still asking when there is nothing left to
/// read has to stop, and say that is why -- looping on an empty answer forever is how a program
/// that was piped its input hangs instead of finishing.
async function askFor(prompt, wanted) {
  for (;;) {
    process.stdout.write(showable(prompt));
    const said = await nextLine();
    if (said === null) {
      nope("There is nothing left to read, and something is still being asked for.");
    }
    if (wanted === null) return said;
    const fine = wanted.includes("yes") || looksLikeNumber(said);
    if (fine) return said;
    console.log(
      `I was hoping for ${wanted} there, and "${said.trim()}" is not one. Could you try again?`
    );
  }
}

function looksLikeNumber(said) {
  const trimmed = said.trim();
  if (trimmed === "") return false;
  return Number.isFinite(Number(trimmed));
}

export async function askText(prompt) {
  return await askFor(prompt, null);
}

export async function askWhole(prompt) {
  const said = await askFor(prompt, "a whole number");
  return Math.trunc(Number(said.trim()));
}

export async function askDecimal(prompt) {
  const said = await askFor(prompt, "a number");
  return Number(said.trim());
}

export async function askYesNo(prompt) {
  const said = await askFor(prompt, "a yes or a no");
  return ["yes", "y", "true", "ok", "okay"].includes(said.trim().toLowerCase());
}

// ---------------------------------------------------------------------------
// Comparing
// ---------------------------------------------------------------------------

export function same(a, b) {
  if (a === b) return true;
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return false;
    return a.every((item, i) => same(item, b[i]));
  }
  if (a instanceof Map && b instanceof Map) {
    if (a.size !== b.size) return false;
    for (const [k, v] of a) {
      if (!b.has(k) || !same(v, b.get(k))) return false;
    }
    return true;
  }
  return false;
}

// ---------------------------------------------------------------------------
// Arithmetic the tower decides
// ---------------------------------------------------------------------------

/// A whole number, checked.
///
/// The emitter puts every whole-number step through here. JavaScript stops being exact at about
/// nine thousand million million, and a whole number past that is not an approximate answer but a
/// wrong one wearing the clothes of a right one. The reference runner keeps whole numbers exact
/// however large they grow, so the honest thing here is to stop and say which of the two you are
/// standing in.
export function whole(v) {
  if (!Number.isSafeInteger(v)) {
    nope(
      "this whole number has grown too big for the JavaScript backend to hold exactly, and I " +
        "will not hand you a number that is nearly right. Run it the ordinary way, with " +
        "`polite run`, where whole numbers stay exact however big they get."
    );
  }
  return v;
}

function plain(v) {
  if (typeof v !== "number") notHere("this kind of number");
  return v;
}
export function add(a, b) {
  return plain(a) + plain(b);
}
export function sub(a, b) {
  return plain(a) - plain(b);
}
export function mul(a, b) {
  return plain(a) * plain(b);
}
export function negate(a) {
  return -plain(a);
}

// ---------------------------------------------------------------------------
// Lists
// ---------------------------------------------------------------------------

function outOfRange(position, len) {
  if (len === 0) return `there is no item ${position}, because the list is empty`;
  return `there is no item ${position}, because the list holds ${len} ${len === 1 ? "item" : "items"}`;
}

export function newList() {
  return [];
}
export function listItem(items, position) {
  if (position < 1 || position > items.length) nope(outOfRange(position, items.length));
  return items[position - 1];
}
export function listCount(items) {
  return items.length;
}
export function listFirst(items) {
  if (!items.length) nope("the list is empty, so it has no first item");
  return items[0];
}
export function listLast(items) {
  if (!items.length) nope("the list is empty, so it has no last item");
  return items[items.length - 1];
}
export function listAppend(items, value) {
  items.push(value);
}
export function listPutAt(items, position, value) {
  if (position < 1 || position > items.length) nope(outOfRange(position, items.length));
  items[position - 1] = value;
}
export function listRemoveAt(items, position) {
  if (position < 1 || position > items.length) nope(outOfRange(position, items.length));
  items.splice(position - 1, 1);
}
export function listSum(items) {
  let total = 0;
  for (const v of items) total += Number(v);
  return total;
}
export function listBiggest(items) {
  if (!items.length) nope("the list is empty, so it has no biggest item");
  return items.reduce((a, b) => (compareValues(b, a) > 0 ? b : a));
}
export function listSmallest(items) {
  if (!items.length) nope("the list is empty, so it has no smallest item");
  return items.reduce((a, b) => (compareValues(b, a) < 0 ? b : a));
}
function compareValues(a, b) {
  if (typeof a === "number" && typeof b === "number") return a - b;
  const [x, y] = [showable(a), showable(b)];
  return x < y ? -1 : x > y ? 1 : 0;
}
export function listSorted(items) {
  return [...items].sort(compareValues);
}
export function listReversed(items) {
  return [...items].reverse();
}
export function listJoin(items, separator) {
  return items.map(showable).join(separator);
}
export function listContains(items, value) {
  return items.some((item) => same(item, value) || item === value);
}
export function listPosition(items, value) {
  const at = items.findIndex((item) => same(item, value) || item === value);
  if (at < 0) nope(`${showable(value)} is not in the list`);
  return at + 1;
}
export function listRest(items) {
  return items.slice(1);
}
export function listFirstFew(items, count) {
  return items.slice(0, Math.max(0, count));
}
export function listAverage(items) {
  if (!items.length) nope("an empty list has nothing to share");
  return listSum(items) / items.length;
}
export function listCountIn(items, value) {
  return items.filter((item) => same(item, value) || item === value).length;
}

// ---------------------------------------------------------------------------
// Lookups
// ---------------------------------------------------------------------------

// A lookup is keyed by text, always, and hands its keys back in order -- the reference runner
// keeps one in a sorted tree, and a lookup that came out in a different order here would be a
// different program rather than the same one written twice.
function sortedKeys(map) {
  return [...map.keys()].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
}

export function newLookup() {
  return new Map();
}
export function lookupGet(map, key) {
  const k = showable(key);
  if (!map.has(k)) nope(`there is nothing kept under "${k}"`);
  return map.get(k);
}
export function lookupPut(map, key, value) {
  map.set(showable(key), value);
}
export function lookupForget(map, key) {
  map.delete(showable(key));
}
export function lookupKeys(map) {
  return sortedKeys(map);
}
export function lookupHas(map, key) {
  return map.has(showable(key));
}
export function lookupCount(map) {
  return map.size;
}

// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

const letters = (t) => [...t];

export function textLength(t) {
  return letters(showable(t)).length;
}
export function textCapitals(t) {
  return showable(t).toUpperCase();
}
export function textSmall(t) {
  return showable(t).toLowerCase();
}
export function textTrimmed(t) {
  return showable(t).trim();
}
export function textWords(t) {
  const found = showable(t).trim();
  return found === "" ? [] : found.split(/\s+/);
}
export function textSplit(t, separator) {
  return showable(t).split(separator);
}
export function textNumber(t) {
  const trimmed = showable(t).trim();
  if (/^[+-]?\d+$/.test(trimmed)) return Number(trimmed);
  const v = Number(trimmed);
  if (trimmed !== "" && Number.isFinite(v)) return v;
  nope(`"${trimmed}" is not a number`);
}
export function textOf(v) {
  return showable(v);
}
export function textStartsWith(t, prefix) {
  return showable(t).startsWith(prefix);
}
export function textEndsWith(t, suffix) {
  return showable(t).endsWith(suffix);
}
export function textContains(t, part) {
  return showable(t).includes(part);
}
export function textSlice(t, from, to) {
  const all = letters(showable(t));
  if (from < 1 || to > all.length || from > to + 1) {
    nope(`there is no piece from ${from} to ${to} in text of ${all.length}`);
  }
  return all.slice(from - 1, to).join("");
}
export function textReplace(t, older, newer) {
  return showable(t).split(older).join(newer);
}
export function textLetter(t, position) {
  const all = letters(showable(t));
  if (position < 1 || position > all.length) {
    nope(`there is no letter ${position}, because the text has ${all.length}`);
  }
  return all[position - 1];
}
export function textLetters(t) {
  return letters(showable(t));
}
export function textRepeated(t, count) {
  return count <= 0 ? "" : showable(t).repeat(count);
}
export function isEmpty(v) {
  if (v === null || v === undefined) return true;
  if (typeof v === "string") return v.length === 0;
  if (Array.isArray(v)) return v.length === 0;
  if (v instanceof Map) return v.size === 0;
  return false;
}

// ---------------------------------------------------------------------------
// Numbers
// ---------------------------------------------------------------------------

export function randomRange(from, to) {
  const low = Math.min(from, to);
  const high = Math.max(from, to);
  return low + Math.floor(Math.random() * (high - low + 1));
}
export function rounded(v) {
  // Away from zero on a half, which is how a person rounds and not how JavaScript does.
  return Math.sign(v) * Math.round(Math.abs(v));
}
export function roundedDown(v) {
  return Math.floor(v);
}
export function roundedUp(v) {
  return Math.ceil(v);
}
export function roundedTo(v, places) {
  const scale = Math.pow(10, places);
  return (Math.sign(v) * Math.round(Math.abs(v) * scale)) / scale;
}
export function absolute(v) {
  return Math.abs(v);
}
export function squareRoot(v) {
  if (v < 0) nope("a number below zero has no square root");
  return Math.sqrt(v);
}
export function cubeRoot(v) {
  return Math.cbrt(v);
}
export function dividesEvenly(a, b) {
  return b === 0 ? false : a % b === 0;
}
export function divideNumbers(a, b) {
  if (b === 0) nope("nothing can be shared into zero parts");
  return a / b;
}
export function remainder(a, b) {
  if (b === 0) nope("nothing can be shared into zero parts");
  return a % b;
}
export function smaller(a, b) {
  return Math.min(a, b);
}
export function larger(a, b) {
  return Math.max(a, b);
}
export function power(a, b) {
  return Math.pow(a, b);
}
export function squared(v) {
  return v * v;
}
export function cubed(v) {
  return v * v * v;
}
export function wholePart(v) {
  return Math.trunc(v);
}
export function fractionPart(v) {
  return v - Math.trunc(v);
}
export function sign(v) {
  return Math.sign(v);
}
export function keptBetween(v, low, high) {
  return Math.min(Math.max(v, low), high);
}
export function pi() {
  return Math.PI;
}
export function eulerE() {
  return Math.E;
}
export function sine(v) {
  return Math.sin(v);
}
export function cosine(v) {
  return Math.cos(v);
}
export function tangent(v) {
  return Math.tan(v);
}
export function arcSine(v) {
  if (v < -1 || v > 1) nope("only a number between minus one and one has an angle");
  return Math.asin(v);
}
export function arcCosine(v) {
  if (v < -1 || v > 1) nope("only a number between minus one and one has an angle");
  return Math.acos(v);
}
export function arcTangent(v) {
  return Math.atan(v);
}
export function angleOver(a, b) {
  return Math.atan2(a, b);
}
export function toDegrees(v) {
  return (v * 180) / Math.PI;
}
export function toRadians(v) {
  return (v * Math.PI) / 180;
}
export function hyperbolicSine(v) {
  return Math.sinh(v);
}
export function hyperbolicCosine(v) {
  return Math.cosh(v);
}
export function hyperbolicTangent(v) {
  return Math.tanh(v);
}
export function naturalLogarithm(v) {
  if (v <= 0) nope("only a number above zero has a logarithm");
  return Math.log(v);
}
export function commonLogarithm(v) {
  if (v <= 0) nope("only a number above zero has a logarithm");
  return Math.log10(v);
}
export function logarithmInBase(v, base) {
  if (v <= 0) nope("only a number above zero has a logarithm");
  if (base <= 0 || base === 1) nope("that is not a base a logarithm can be taken in");
  return Math.log(v) / Math.log(base);
}
export function exponential(v) {
  return Math.exp(v);
}
export function greatestCommonFactor(a, b) {
  let [x, y] = [Math.abs(Math.trunc(a)), Math.abs(Math.trunc(b))];
  while (y) [x, y] = [y, x % y];
  return x;
}
export function smallestCommonMultiple(a, b) {
  if (a === 0 || b === 0) return 0;
  const v = Math.abs(Math.trunc(a) * Math.trunc(b)) / greatestCommonFactor(a, b);
  return exactly(v, "that multiple");
}
/// A whole number that JavaScript can no longer hold exactly.
///
/// The reference runner keeps whole numbers exact however large they get. JavaScript stops being
/// exact at about nine thousand million million, and a number past that is not a slightly wrong
/// answer -- it is a wrong answer wearing the clothes of a right one. So it is refused.
function exactly(v, what) {
  if (!Number.isSafeInteger(v)) {
    nope(
      `${what} is too big for the JavaScript backend to hold exactly, so I will not pretend to. ` +
        `Run it the ordinary way, with \`polite run\`, where whole numbers stay exact however big ` +
        `they get.`
    );
  }
  return v;
}

export function factorial(v) {
  if (v < 0) nope("a number below zero has no factorial");
  let total = 1;
  for (let i = 2; i <= v; i++) {
    total *= i;
    if (!Number.isSafeInteger(total)) exactly(total, `${v} factorial`);
  }
  return total;
}
export function isPrime(v) {
  const n = Math.trunc(v);
  if (n < 2) return false;
  if (n % 2 === 0) return n === 2;
  for (let d = 3; d * d <= n; d += 2) if (n % d === 0) return false;
  return true;
}
export function primeFactors(v) {
  let n = Math.abs(Math.trunc(v));
  const found = [];
  for (let d = 2; d * d <= n; d++) {
    while (n % d === 0) {
      found.push(d);
      n /= d;
    }
  }
  if (n > 1) found.push(n);
  return found;
}
export function divisors(v) {
  const n = Math.abs(Math.trunc(v));
  const found = [];
  for (let d = 1; d <= n; d++) if (n % d === 0) found.push(d);
  return found;
}
export function powerWithin(a, b, m) {
  if (m === 0) nope("nothing can be shared into zero parts");
  let result = 1;
  let base = ((a % m) + m) % m;
  let e = b;
  while (e > 0) {
    if (e % 2 === 1) result = (result * base) % m;
    base = (base * base) % m;
    e = Math.floor(e / 2);
  }
  return result;
}
export function inverseWithin(a, m) {
  for (let x = 1; x < m; x++) if ((a * x) % m === 1) return x;
  nope(`${a} has no inverse within ${m}`);
}
export function waysToChoose(n, k) {
  if (k < 0 || k > n) return 0;
  let total = 1;
  for (let i = 1; i <= k; i++) total = (total * (n - k + i)) / i;
  return exactly(Math.round(total), "that many ways to choose");
}
export function waysToArrange(n, k) {
  if (k < 0 || k > n) return 0;
  let total = 1;
  for (let i = 0; i < k; i++) total *= n - i;
  return exactly(total, "that many ways to arrange");
}
export function inBinary(v) {
  return Math.trunc(v).toString(2);
}
export function inHexadecimal(v) {
  return Math.trunc(v).toString(16);
}
export function inBase(v, base) {
  if (base < 2 || base > 36) nope("a base has to be between two and thirty-six");
  return Math.trunc(v).toString(base);
}
export function valueOfInBase(t, base) {
  if (base < 2 || base > 36) nope("a base has to be between two and thirty-six");
  const v = parseInt(showable(t).trim(), base);
  if (Number.isNaN(v)) nope(`"${showable(t)}" is not a number in base ${base}`);
  return v;
}
export function bitwiseAnd(a, b) {
  return Math.trunc(a) & Math.trunc(b);
}
export function bitwiseOr(a, b) {
  return Math.trunc(a) | Math.trunc(b);
}
export function bitwiseExclusiveOr(a, b) {
  return Math.trunc(a) ^ Math.trunc(b);
}
export function bitwiseNot(a) {
  return ~Math.trunc(a);
}
export function shiftedLeft(a, b) {
  return exactly(Math.trunc(a) * Math.pow(2, Math.trunc(b)), "that number, shifted");
}
export function shiftedRight(a, b) {
  return Math.floor(Math.trunc(a) / Math.pow(2, Math.trunc(b)));
}
export function asPercentageOf(a, b) {
  if (b === 0) nope("nothing can be shared into zero parts");
  return (a / b) * 100;
}
export function percentOf(percent, v) {
  return (percent / 100) * v;
}

// ---------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------

export function median(items) {
  if (!items.length) nope("an empty list has no middle");
  const sorted = [...items].map(Number).sort((a, b) => a - b);
  const half = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[half] : (sorted[half - 1] + sorted[half]) / 2;
}
export function mode(items) {
  if (!items.length) nope("an empty list has nothing that happens most");
  const counts = new Map();
  for (const v of items) counts.set(showable(v), (counts.get(showable(v)) || 0) + 1);
  let best = null;
  let bestCount = -1;
  for (const v of items) {
    const c = counts.get(showable(v));
    if (c > bestCount) {
      bestCount = c;
      best = v;
    }
  }
  return best;
}
export function spread(items) {
  if (!items.length) nope("an empty list has no spread");
  return Math.sqrt(variance(items));
}
export function variance(items) {
  if (!items.length) nope("an empty list has no variance");
  const mean = listSum(items) / items.length;
  return items.reduce((total, v) => total + Math.pow(Number(v) - mean, 2), 0) / items.length;
}
export function correlation(a, b) {
  if (a.length !== b.length || !a.length) nope("two lists of the same length are needed");
  const meanA = listSum(a) / a.length;
  const meanB = listSum(b) / b.length;
  let top = 0;
  let leftSide = 0;
  let rightSide = 0;
  for (let i = 0; i < a.length; i++) {
    const x = Number(a[i]) - meanA;
    const y = Number(b[i]) - meanB;
    top += x * y;
    leftSide += x * x;
    rightSide += y * y;
  }
  if (leftSide === 0 || rightSide === 0) nope("a list that never changes has no correlation");
  return top / Math.sqrt(leftSide * rightSide);
}

// ---------------------------------------------------------------------------
// Vectors and matrices
// ---------------------------------------------------------------------------

function sameLength(a, b) {
  if (a.length !== b.length) nope("two lists of the same length are needed");
}
export function pairwiseSum(a, b) {
  sameLength(a, b);
  return a.map((v, i) => Number(v) + Number(b[i]));
}
export function pairwiseProduct(a, b) {
  sameLength(a, b);
  return a.map((v, i) => Number(v) * Number(b[i]));
}
export function dotProduct(a, b) {
  sameLength(a, b);
  return a.reduce((total, v, i) => total + Number(v) * Number(b[i]), 0);
}
export function crossProduct(a, b) {
  if (a.length !== 3 || b.length !== 3) nope("a cross product wants two lists of three");
  return [
    a[1] * b[2] - a[2] * b[1],
    a[2] * b[0] - a[0] * b[2],
    a[0] * b[1] - a[1] * b[0],
  ];
}
export function magnitude(a) {
  return Math.sqrt(a.reduce((total, v) => total + Number(v) * Number(v), 0));
}
export function scaledBy(a, factor) {
  return a.map((v) => Number(v) * factor);
}
export function transpose(m) {
  if (!m.length) return [];
  return m[0].map((_, c) => m.map((row) => row[c]));
}
export function matrixProduct(a, b) {
  if (!a.length || !b.length) nope("an empty matrix has no product");
  if (a[0].length !== b.length) nope("these two matrices cannot be multiplied");
  return a.map((row) =>
    b[0].map((_, c) => row.reduce((total, v, i) => total + Number(v) * Number(b[i][c]), 0))
  );
}
export function identityMatrix(n) {
  if (n < 1) nope("an identity matrix has at least one row");
  return Array.from({ length: n }, (_, r) =>
    Array.from({ length: n }, (_, c) => (r === c ? 1 : 0))
  );
}
export function determinant(m) {
  const n = m.length;
  if (!n || m.some((row) => row.length !== n)) nope("only a square matrix has a determinant");
  const a = m.map((row) => row.map(Number));
  let total = 1;
  for (let i = 0; i < n; i++) {
    let pivot = i;
    while (pivot < n && a[pivot][i] === 0) pivot++;
    if (pivot === n) return 0;
    if (pivot !== i) {
      [a[i], a[pivot]] = [a[pivot], a[i]];
      total = -total;
    }
    total *= a[i][i];
    for (let r = i + 1; r < n; r++) {
      const factor = a[r][i] / a[i][i];
      for (let c = i; c < n; c++) a[r][c] -= factor * a[i][c];
    }
  }
  return total;
}
export function matrixInverse(m) {
  const n = m.length;
  if (!n || m.some((row) => row.length !== n)) nope("only a square matrix has an inverse");
  const a = m.map((row, r) =>
    row.map(Number).concat(Array.from({ length: n }, (_, c) => (r === c ? 1 : 0)))
  );
  for (let i = 0; i < n; i++) {
    let pivot = i;
    while (pivot < n && Math.abs(a[pivot][i]) < 1e-12) pivot++;
    if (pivot === n) nope("this matrix has no inverse");
    [a[i], a[pivot]] = [a[pivot], a[i]];
    const d = a[i][i];
    for (let c = 0; c < 2 * n; c++) a[i][c] /= d;
    for (let r = 0; r < n; r++) {
      if (r === i) continue;
      const factor = a[r][i];
      for (let c = 0; c < 2 * n; c++) a[r][c] -= factor * a[i][c];
    }
  }
  return a.map((row) => row.slice(n));
}

// ---------------------------------------------------------------------------
// Fractions and complex numbers
//
// The reference runner keeps these exactly. Doing that here means carrying a numeric tower into
// JavaScript, which is a piece of work of its own and is not what a bot needs, so for now they
// say so plainly rather than being approximated behind your back.
// ---------------------------------------------------------------------------

export const makeFraction = () => notHere("a fraction");
export const fractionTop = () => notHere("a fraction");
export const fractionBottom = () => notHere("a fraction");
export const asFraction = () => notHere("a fraction");
export const imaginaryNumber = () => notHere("a complex number");
export const realPart = () => notHere("a complex number");
export const imaginaryPart = () => notHere("a complex number");
export const conjugate = () => notHere("a complex number");
export const direction = () => notHere("a complex number");
export const complexSquareRoot = () => notHere("a complex number");

export function asDecimal(v) {
  return Number(v);
}
export function asWholeNumber(v) {
  return Math.trunc(Number(v));
}
export function wholeNumberIn(v) {
  return Math.trunc(Number(v));
}

// ---------------------------------------------------------------------------
// Drawing
//
// The canvas, the letters and the picture writer all live in the reference runner. They are not
// here yet, and a program that draws is told so rather than left to fail oddly.
// ---------------------------------------------------------------------------

const noDrawing = () => notHere("drawing");
export const openCanvas = noDrawing;
export const clearCanvas = noDrawing;
export const paintPoint = noDrawing;
export const drawLine = noDrawing;
export const drawBox = noDrawing;
export const fillBox = noDrawing;
export const drawCircle = noDrawing;
export const revealCanvas = noDrawing;
export const revealLetters = noDrawing;
export const makeColour = noDrawing;
export const namedColour = noDrawing;
export const canvasWidth = noDrawing;
export const canvasHeight = noDrawing;
export const colourAt = noDrawing;
export const writeText = noDrawing;
export const letterSize = noDrawing;
export const writtenWidth = noDrawing;
export const saveCanvas = noDrawing;
export const putInWindow = noDrawing;
export const dotSize = noDrawing;

// ---------------------------------------------------------------------------
// Files, time, waiting
// ---------------------------------------------------------------------------

/// Said in the same words the reference runner uses, because a program that handles a failure by
/// looking at what it says should not have to be written twice.
function fileTrouble(path, e) {
  if (e && e.code === "ENOENT") return `there is no file called "${path}"`;
  if (e && (e.code === "EACCES" || e.code === "EPERM")) {
    return `"${path}" is not mine to read or write`;
  }
  return `"${path}" could not be reached`;
}

export function fileContents(path) {
  try {
    return fs.readFileSync(path, "utf8");
  } catch (e) {
    nope(fileTrouble(path, e));
  }
}
export function fileWrite(text, path) {
  try {
    fs.writeFileSync(path, showable(text), "utf8");
  } catch (e) {
    nope(fileTrouble(path, e));
  }
}
export function fileAppend(text, path) {
  try {
    fs.appendFileSync(path, showable(text), "utf8");
  } catch (e) {
    nope(fileTrouble(path, e));
  }
}
export function fileExists(path) {
  return fs.existsSync(path);
}
export function timeNow() {
  return Math.floor(Date.now() / 1000);
}
export function waitFor(seconds) {
  return new Promise((resolve) => setTimeout(resolve, Math.max(0, seconds) * 1000));
}

// ---------------------------------------------------------------------------
// Discord
//
// A bot here is a loop rather than a pile of handlers: listen, look at what was said, answer.
// `listen for the next message` is where it waits, and because everything the emitter writes is
// awaited anyway, waiting costs nothing and blocks nothing.
//
// discord.js is brought in only when a program actually logs in, so a program that never mentions
// Discord never needs it installed.
// ---------------------------------------------------------------------------

let bot = null;
let inbox = [];
let listeners = [];
let heard = null;

export function secretCalled(name) {
  const found = process.env[showable(name)];
  if (found === undefined || found === "") {
    nope(
      `there is no secret called ${showable(name)} here. Set it before running, and it stays out ` +
        `of the program where it belongs.`
    );
  }
  return found;
}

export async function discordLogIn(token) {
  let discordjs;
  try {
    discordjs = await import("discord.js");
  } catch {
    nope(
      "discord.js is not installed here. In the folder this program is in, run: npm install discord.js"
    );
  }
  const { Client, GatewayIntentBits, Partials } = discordjs;

  bot = new Client({
    intents: [
      GatewayIntentBits.Guilds,
      GatewayIntentBits.GuildMessages,
      GatewayIntentBits.MessageContent,
      GatewayIntentBits.DirectMessages,
    ],
    partials: [Partials.Channel],
  });

  bot.on("messageCreate", (message) => {
    // A bot that hears itself answers itself, and then answers that, and so on until somebody
    // pulls the plug. It never hears itself. Other bots it does hear, so that `they are a bot`
    // is a question the program can actually answer.
    if (bot.user && message.author.id === bot.user.id) return;
    const next = listeners.shift();
    if (next) next(message);
    else inbox.push(message);
  });

  await new Promise((ready, failed) => {
    bot.once("clientReady", () => ready());
    bot.once("ready", () => ready());
    bot.once("error", (e) => failed(e));
    bot.login(showable(token)).catch((e) => failed(e));
  }).catch((e) => {
    const why = e && e.message ? e.message : String(e);
    if (/token/i.test(why)) {
      nope("Discord would not take that token. Check it is the bot's token, and the current one.");
    }
    nope(`I could not log in to Discord: ${why}`);
  });

  console.log(`  Logged in to Discord as ${bot.user.tag}.`);
}

function mustBeLoggedIn() {
  if (!bot) nope("log in to Discord before trying to talk to it.");
}

export async function discordNext() {
  mustBeLoggedIn();
  heard = inbox.length
    ? inbox.shift()
    : await new Promise((arrived) => listeners.push(arrived));
}

function mustHaveHeard() {
  mustBeLoggedIn();
  if (!heard) nope("nothing has been heard yet, so there is nothing to answer.");
  return heard;
}

async function trying(what, doing) {
  try {
    return await doing();
  } catch (e) {
    const why = e && e.message ? e.message : String(e);
    if (/permission|missing access|forbidden/i.test(why)) {
      nope(`I am not allowed to ${what} there. The bot needs permission in that channel.`);
    }
    nope(`I could not ${what}: ${why}`);
  }
}

export async function discordReply(text) {
  const message = mustHaveHeard();
  await trying("reply", () => message.reply(showable(text)));
}

export async function discordSend(text) {
  const message = mustHaveHeard();
  await trying("send that", () => message.channel.send(showable(text)));
}

export async function discordStatus(text) {
  mustBeLoggedIn();
  await trying("set my status", async () => bot.user.setActivity(showable(text)));
}

export function discordSaid() {
  return heard ? heard.content : "";
}
export function discordName() {
  if (!heard) return "";
  return heard.member?.displayName ?? heard.author.username;
}
export function discordIsBot() {
  return heard ? heard.author.bot === true : false;
}
export function discordChannel() {
  if (!heard) return "";
  return heard.channel?.name ?? "a private message";
}
export function discordServer() {
  if (!heard) return "";
  return heard.guild?.name ?? "no server";
}

// ---------------------------------------------------------------------------
// Starting and stopping
// ---------------------------------------------------------------------------

/// Run the program, and end the way the language ends rather than the way JavaScript does.
export async function begin(main) {
  try {
    await main();
  } catch (e) {
    if (e instanceof Finished) {
      if (!e.quiet) console.log(`\n${e.message}`);
    } else if (e instanceof Politely) {
      // A failure that nothing handled reached the top, which is where the program stops.
      console.log(`\n${e.message}`);
      process.exitCode = 1;
    } else {
      throw e;
    }
  } finally {
    // The line reader keeps the program alive if it was ever started, and so does a bot that is
    // still connected. Both are let go of here, or the program would sit there having finished.
    process.stdin.pause();
    if (bot) await bot.destroy();
  }
}
