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

    def row(name, label, what):
        if name not in f:
            return ""
        e = f[name]
        return (
            f'<li><a href="{html.escape(e["path"])}">{label}</a> <span>{what}, build {e["build"]}, {mb(e["size"])}</span>'
            f'<code>SHA-256 {e["sha256"]}</code></li>'
        )

    return f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex">
<title>Backline downloads</title>
<style>
:root {{ color-scheme: light dark; --bg: #f4f3f0; --fg: #1d1d1f; --dim: #6b6b70; --line: #d8d6d0; --link: #8a5a00; }}
@media (prefers-color-scheme: dark) {{ :root {{ --bg: #141416; --fg: #e8e6e3; --dim: #9a9aa0; --line: #2c2c30; --link: #e0b25c; }} }}
body {{ background: var(--bg); color: var(--fg); font: 16px/1.5 system-ui, sans-serif; margin: 0; padding: 40px 16px; }}
main {{ max-width: 640px; margin: 0 auto; }}
h1 {{ font-size: 28px; margin: 0 0 4px; letter-spacing: .02em; }}
h2 {{ font-size: 15px; text-transform: uppercase; letter-spacing: .08em; color: var(--dim); margin: 28px 0 8px; }}
p {{ color: var(--dim); margin: 0 0 8px; }}
ul {{ list-style: none; padding: 0; margin: 0; }}
li {{ padding: 12px 0; border-top: 1px solid var(--line); }}
a {{ color: var(--link); font-weight: 600; text-decoration: none; }}
a:hover {{ text-decoration: underline; }}
span {{ color: var(--dim); margin-left: 6px; }}
code {{ display: block; font-size: 11px; color: var(--dim); overflow-wrap: anywhere; margin-top: 4px; }}
</style></head>
<body><main>
<h1>Backline</h1>
<p>A retro music player made for practice. Once it's installed it updates itself.</p>
<h2>Windows</h2>
<ul>{row("backline-setup.msi", "Installer", "the easy way")}{row("backline-practice.exe", "The app on its own", "no install, just run it")}</ul>
<h2>Mac</h2>
<ul>{row("backline-practice-mac.tar.gz", "Backline for Mac", "Apple Silicon and Intel")}</ul>
<p>On a Mac: open the download, then the first time right-click <b>backline</b> and choose Open (it isn't signed).</p>
<p>Want just the player? Turn on Hide the Practice Tools at the top of Preferences.</p>
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
