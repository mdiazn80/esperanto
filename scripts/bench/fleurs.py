#!/usr/bin/env python3
"""Build an English/Spanish test set for esperanto-bench from FLEURS.

FLEURS (https://huggingface.co/datasets/google/fleurs, CC-BY-4.0) reads the
same sentences in many languages: the `id` column is shared, so the Spanish
reading of a sentence is the reference translation of the English one and
vice versa.

Downloads the `test` split of en_us and es_419 (~0.9 GB, streamed; only the
selected clips are written to disk) and writes:

    <out>/manifest.jsonl
    <out>/en/<id>.wav
    <out>/es/<id>.wav

Standard library only. Usage:
    python3 scripts/bench/fleurs.py --out ~/.cache/esperanto-bench/fleurs --count 100
"""

from __future__ import annotations

import argparse
import csv
import io
import json
import random
import sys
import tarfile
import urllib.request
from pathlib import Path

BASE_URL = "https://huggingface.co/datasets/google/fleurs/resolve/main/data"
LANGUAGES = {"en": "en_us", "es": "es_419"}
SPLIT = "test"


def fetch(url: str):
    request = urllib.request.Request(url, headers={"User-Agent": "esperanto-bench"})
    return urllib.request.urlopen(request)  # noqa: S310 (fixed https URL)


def read_index(config: str) -> dict[str, tuple[str, str]]:
    """Maps sentence id -> (first recording file name, raw transcription)."""
    with fetch(f"{BASE_URL}/{config}/{SPLIT}.tsv") as response:
        text = response.read().decode("utf-8")
    index: dict[str, tuple[str, str]] = {}
    for row in csv.reader(io.StringIO(text), delimiter="\t", quoting=csv.QUOTE_NONE):
        sentence_id, file_name, raw = row[0], row[1], row[2]
        index.setdefault(sentence_id, (file_name, raw))
    return index


def extract(config: str, wanted: dict[str, str], target_dir: Path) -> None:
    """Streams the split's audio archive, writing only the wanted files.

    `wanted` maps recording file name -> sentence id.
    """
    target_dir.mkdir(parents=True, exist_ok=True)
    remaining = dict(wanted)
    url = f"{BASE_URL}/{config}/audio/{SPLIT}.tar.gz"
    print(f"Streaming {url} …", file=sys.stderr)
    with fetch(url) as response, tarfile.open(fileobj=response, mode="r|gz") as archive:
        for member in archive:
            name = Path(member.name).name
            if not member.isfile() or name not in remaining:
                continue
            source = archive.extractfile(member)
            if source is None:
                continue
            (target_dir / f"{remaining.pop(name)}.wav").write_bytes(source.read())
            if not remaining:
                break
    if remaining:
        raise SystemExit(f"{config}: {len(remaining)} clips not found in the archive")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--count", type=int, default=100, help="sentences per language")
    parser.add_argument("--seed", type=int, default=42)
    args = parser.parse_args()

    indexes = {lang: read_index(config) for lang, config in LANGUAGES.items()}
    shared = sorted(set(indexes["en"]) & set(indexes["es"]), key=int)
    selected = sorted(random.Random(args.seed).sample(shared, min(args.count, len(shared))), key=int)
    print(f"{len(shared)} sentences in both languages; using {len(selected)}", file=sys.stderr)

    for lang, config in LANGUAGES.items():
        wanted = {indexes[lang][sid][0]: sid for sid in selected}
        extract(config, wanted, args.out / lang)

    lines = []
    for sid in selected:
        for lang, other in (("en", "es"), ("es", "en")):
            lines.append(
                json.dumps(
                    {
                        "id": sid,
                        "audio": f"{lang}/{sid}.wav",
                        "language": lang,
                        "transcript": indexes[lang][sid][1],
                        "translations": {other: indexes[other][sid][1]},
                    },
                    ensure_ascii=False,
                )
            )
    (args.out / "manifest.jsonl").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"Wrote {len(lines)} clips to {args.out / 'manifest.jsonl'}", file=sys.stderr)


if __name__ == "__main__":
    main()
