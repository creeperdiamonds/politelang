# Run every example twice -- once through the reference runner, once through Node after being
# written out as JavaScript -- and insist the two agree.
#
# This is the only test of a backend worth having. A backend that emits plausible-looking code is
# worth nothing; a backend that produces the same answers as the runner it is standing in for is
# worth everything.

import glob, os, re, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
POLITE = os.path.join(ROOT, "target", "release", "polite" + (".exe" if os.name == "nt" else ""))
OUT = os.path.join(ROOT, "target", "js-comparison")

# Left out, with the reason, because a silent skip reads as a pass.
SKIPPED = {
    "parts": "borrowed by another file rather than run on its own",
    "drawing": "draws, and drawing is not in the JavaScript backend yet",
    "errand": "a game wanting a hundred and thirty commands",
}
UNSTEADY = ("a random", "random whole", "the time now", "shuffle")


def normalise(text):
    # JavaScript has one kind of number and does not remember whether it was a whole one, so a
    # decimal holding a whole value prints as `7` where the runner prints `7.0`. Known, written
    # down in the README, and not what this test is looking for.
    text = re.sub(r"(?<![\d.])(\d+)\.0(?![\d])", r"\1", text)
    return text.replace("\r\n", "\n").strip()


def run(cmd):
    try:
        done = subprocess.run(
            cmd, cwd=ROOT, stdin=subprocess.DEVNULL,
            capture_output=True, text=True, timeout=120, encoding="utf-8", errors="replace",
        )
        return (done.stdout or "") + (done.stderr or "")
    except subprocess.TimeoutExpired:
        return "<<timed out>>"


files = sorted(glob.glob(os.path.join(ROOT, "examples", "**", "*.polite"), recursive=True))

agreed, differed, skipped, refused = [], [], [], []

for path in files:
    rel = os.path.relpath(path, ROOT).replace("\\", "/")

    why = next((w for k, w in SKIPPED.items() if "/" + k + "/" in "/" + rel), None)
    if why is None:
        # Whatever this file borrows counts too: the chance can be in the borrowed file rather
        # than in this one, and a run whose questions are drawn at random will never match.
        src = open(path, encoding="utf-8").read()
        for extra in glob.glob(os.path.join(os.path.dirname(path), "parts", "*.polite")):
            src += open(extra, encoding="utf-8").read()
        if any(u in src for u in UNSTEADY):
            why = "leaves something to chance or to the clock, so two runs never match"
    if why:
        skipped.append((rel, why))
        continue

    written = subprocess.run(
        [POLITE, "build", path, "--out", OUT],
        cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace",
    )
    if written.returncode != 0:
        differed.append((rel, "could not be written out", written.stdout + written.stderr))
        continue

    module = os.path.join(OUT, os.path.splitext(os.path.basename(path))[0] + ".mjs")

    # `polite run` says what it has to say about the program before running it, and `polite build`
    # said the same things earlier. Those notices are the toolchain talking, not the program, so
    # they are taken off the front before the two are set against each other.
    notices = run([POLITE, "check", path])
    notices = chr(10).join(l for l in notices.splitlines() if not l.startswith("All good")).strip()

    from_runner = run([POLITE, "run", path])
    if notices and from_runner.strip().startswith(notices[:60]):
        from_runner = from_runner.strip()[len(notices):]
    from_runner = normalise(from_runner)
    from_node = normalise(run(["node", module]))

    # A whole number past what JavaScript can hold exactly is refused rather than guessed at.
    # That is the backend behaving correctly, not the two disagreeing, so it is counted apart.
    if "too big for the JavaScript backend" in from_node or "not in the JavaScript backend yet" in from_node:
        refused.append(rel)
    elif from_runner == from_node:
        agreed.append(rel)
    else:
        differed.append((rel, "the two do not agree", None))
        # Show the first line that parts company, which is almost always the whole story.
        a, b = from_runner.split("\n"), from_node.split("\n")
        for i in range(max(len(a), len(b))):
            x = a[i] if i < len(a) else "<nothing>"
            y = b[i] if i < len(b) else "<nothing>"
            if x != y:
                differed[-1] = (rel, f"line {i + 1}\n      runner: {x!r}\n      node  : {y!r}", None)
                break

print(f"agreed  : {len(agreed)}")
print(f"differed: {len(differed)}")
print(f"refused : {len(refused)}  (said plainly that something is not in this backend)")
print(f"skipped : {len(skipped)}")
print()
for rel, why, extra in differed:
    print(f"  {rel}\n      {why}")
    if extra:
        print("      " + extra.strip().replace("\n", "\n      ")[:400])
print()
for rel, why in skipped:
    print(f"  skipped {rel}\n      {why}")

sys.exit(1 if differed else 0)
