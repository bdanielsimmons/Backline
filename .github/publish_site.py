"""Make Backline's download site (GitHub Pages) for a release: a folder the workflow then puts on Pages.

    publish_site.py <build number> <folder of files> <notes.md or ""> <site url> <out folder>

Pages swaps the whole site in on each deploy, so the folder holds everything the site should keep:
  builds/<n>/<file>  the newest build of each file. A file not built this time (the Mac one, when it's built less
               often) is fetched from the live site and checked against its SHA-256 so it stays.
  latest.json  the newest build of each file, with its path, SHA-256 and size. The app reads this to update, and
               checks the download against the SHA-256.
  notes.json   the latest 30 releases' notes, newest first (the app's WHAT'S NEW).
  index.html   a plain page with the downloads, for anyone given the link.
  robots.txt   asks search engines to leave the site out (it's for people given the link).
"""
import hashlib
import html
import json
import os
import shutil
import sys
import urllib.error
import urllib.request

FILES = ["backline-practice.exe", "backline-setup.msi", "backline-practice-mac.tar.gz"]


def fetch(url):
    req = urllib.request.Request(url, headers={"User-Agent": "backline-publish"})
    with urllib.request.urlopen(req, timeout=120) as r:
        return r.read()


def read_json(site, name, default):
    """The live site's file; `default` when the site has none yet (the first time). Any other failure stops the
    job, so a hiccup can't publish a site that has forgotten the Mac file or the old notes."""
    try:
        return json.loads(fetch(f"{site}/{name}"))
    except urllib.error.HTTPError as e:
        if e.code == 404:
            return default
        raise


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def mb(n):
    return f"{n / 1048576:.0f} MB"


def page(latest):
    f = latest["files"]

    def link(name, label, cls):
        if name not in f:
            return ""
        e = f[name]
        return f'<a class="{cls}" href="{html.escape(e["path"])}">{label}</a>'

    def size(name):
        return f' <span class="size">({mb(f[name]["size"])})</span>' if name in f else ""

    # Plain steps for people who don't download apps often. The SHA-256s stay in latest.json (the app checks them).
    win = link("backline-setup.msi", "Download for Windows", "big")
    mac = link("backline-practice-mac.tar.gz", "Download for Mac", "big")
    return f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex">
<title>Get Backline</title>
<style>
:root {{ color-scheme: light dark; --bg: #f4f3f0; --fg: #1d1d1f; --dim: #5f5f66; --line: #d8d6d0; --btn: #8a5a00; --btn-fg: #fff; --card: #fff; }}
@media (prefers-color-scheme: dark) {{ :root {{ --bg: #141416; --fg: #e8e6e3; --dim: #a3a3aa; --line: #2c2c30; --btn: #e0b25c; --btn-fg: #141416; --card: #1d1d20; }} }}
body {{ background: var(--bg); color: var(--fg); font: 18px/1.55 system-ui, sans-serif; margin: 0; padding: 40px 16px; }}
main {{ max-width: 560px; margin: 0 auto; }}
h1 {{ font-size: 34px; margin: 0 0 4px; }}
.lead {{ color: var(--dim); margin: 0 0 28px; }}
section {{ background: var(--card); border: 1px solid var(--line); border-radius: 14px; padding: 22px; margin: 0 0 18px; }}
h2 {{ font-size: 22px; margin: 0 0 14px; }}
a.big {{ display: block; text-align: center; background: var(--btn); color: var(--btn-fg); font-weight: 700; font-size: 20px;
  text-decoration: none; border-radius: 10px; padding: 16px; }}
.size {{ color: var(--dim); font-size: 15px; }}
ol {{ margin: 16px 0 0; padding-left: 22px; }}
li {{ margin: 0 0 8px; }}
b {{ font-weight: 650; }}
.small {{ color: var(--dim); font-size: 15px; margin: 14px 0 0; }}
.small a {{ color: inherit; }}
</style></head>
<body><main>
<h1>Backline</h1>
<p class="lead">A music player made for practicing. Install it once and it keeps itself up to date.</p>
<section>
<h2>Windows</h2>
{win}
<ol>
<li>Open the file you downloaded{size("backline-setup.msi")}. If your browser asks, choose <b>Keep</b>.</li>
<li>If Windows says <b>"Windows protected your PC"</b>, click <b>More info</b>, then <b>Run anyway</b>.</li>
<li>Click through the installer. Backline is then on your desktop and in the Start menu.</li>
</ol>
<p class="small">No installing allowed on your computer? {link("backline-practice.exe", "Get the app on its own", "")} and just double-click it.</p>
</section>
<section>
<h2>Mac</h2>
{mac}
<ol>
<li>Open the file you downloaded{size("backline-practice-mac.tar.gz")}. It turns into a file called <b>backline</b>.</li>
<li>Double-click <b>backline</b>. The first time, your Mac says it can't check it: click <b>Done</b>.</li>
<li>Open <b>System Settings</b>, then <b>Privacy &amp; Security</b>. Scroll down and click <b>Open Anyway</b>.</li>
</ol>
</section>
<p class="small">Just want the music player? In Backline, open Preferences and turn on <b>Hide the Practice Tools</b>.</p>
</main></body></html>
"""


def write(out, key, text):
    with open(os.path.join(out, key), "w", encoding="utf-8", newline="\n") as f:
        f.write(text)


def main():
    n, folder, notes_md, site, out = sys.argv[1:6]
    n = int(n)
    site = site.rstrip("/")
    os.makedirs(out, exist_ok=True)
    old = read_json(site, "latest.json", {"files": {}})
    latest = {"files": {}}

    for name in FILES:
        path = os.path.join(folder, name)
        if not os.path.exists(path):
            continue
        key = f"builds/{n}/{name}"
        os.makedirs(os.path.join(out, os.path.dirname(key)), exist_ok=True)
        shutil.copyfile(path, os.path.join(out, key))
        latest["files"][name] = {"build": n, "path": key, "sha256": sha256(path), "size": os.path.getsize(path)}
        print(f"{name}: {key}")

    # the files not built this time stay, fetched from the live site and checked
    for name, e in old.get("files", {}).items():
        if name in latest["files"] or name not in FILES:
            continue
        dest = os.path.join(out, e["path"])
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        try:
            data = fetch(f"{site}/{e['path']}")
        except urllib.error.HTTPError as err:
            print(f"::warning::{name} (build {e['build']}) is gone from the site ({err.code}), so it's left out")
            continue
        if hashlib.sha256(data).hexdigest() != e["sha256"]:
            sys.exit(f"{name} on the site doesn't match its SHA-256")
        with open(dest, "wb") as f:
            f.write(data)
        latest["files"][name] = e
        print(f"{name}: kept build {e['build']}")

    if not latest["files"]:
        sys.exit("nothing to publish")
    latest["build"] = max(e["build"] for e in latest["files"].values())

    notes = read_json(site, "notes.json", [])
    if notes_md and os.path.exists(notes_md):
        body = open(notes_md, encoding="utf-8").read().strip()
        notes = [x for x in notes if x.get("build") != n]
        notes.insert(0, {"build": n, "tag": f"v9.{n}", "body": body})
        notes = sorted(notes, key=lambda x: -x.get("build", 0))[:30]

    write(out, "notes.json", json.dumps(notes, indent=1))
    write(out, "latest.json", json.dumps(latest, indent=1))
    write(out, "index.html", page(latest))
    write(out, "robots.txt", "User-agent: *\nDisallow: /\n")
    print(f"site for build {n} made in {out}, for {site}/")


if __name__ == "__main__":
    main()
