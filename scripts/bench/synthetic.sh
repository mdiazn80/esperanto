#!/usr/bin/env bash
# Builds a tiny English/Spanish test set with macOS text-to-speech, for
# smoke-testing esperanto-bench without downloading a dataset.
# Synthetic voices are much easier than real speech: don't use these
# numbers to compare models.
#
# Usage: scripts/bench/synthetic.sh <out-dir>
set -euo pipefail

out="${1:?usage: synthetic.sh <out-dir>}"
mkdir -p "$out/en" "$out/es"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# id | English | Spanish
sentences=(
  "1|The meeting starts at nine in the morning.|La reunión empieza a las nueve de la mañana."
  "2|Please send me the report before Friday.|Por favor, envíame el informe antes del viernes."
  "3|The weather will be sunny for the rest of the week.|El tiempo será soleado el resto de la semana."
  "4|We need to buy bread, milk and eggs.|Necesitamos comprar pan, leche y huevos."
  "5|The train to Madrid leaves from platform four.|El tren a Madrid sale del andén cuatro."
)

manifest="$out/manifest.jsonl"
: > "$manifest"
for entry in "${sentences[@]}"; do
  IFS='|' read -r id en es <<< "$entry"
  say -v Samantha -o "$tmp/en-$id.aiff" "$en"
  say -v Paulina -o "$tmp/es-$id.aiff" "$es"
  afconvert -f WAVE -d LEI16@16000 -c 1 "$tmp/en-$id.aiff" "$out/en/$id.wav"
  afconvert -f WAVE -d LEI16@16000 -c 1 "$tmp/es-$id.aiff" "$out/es/$id.wav"
  python3 - "$manifest" "$id" "$en" "$es" <<'PY'
import json, sys
path, sid, en, es = sys.argv[1:]
with open(path, "a", encoding="utf-8") as f:
    f.write(json.dumps({"id": sid, "audio": f"en/{sid}.wav", "language": "en",
                        "transcript": en, "translations": {"es": es}}, ensure_ascii=False) + "\n")
    f.write(json.dumps({"id": sid, "audio": f"es/{sid}.wav", "language": "es",
                        "transcript": es, "translations": {"en": en}}, ensure_ascii=False) + "\n")
PY
done
echo "Wrote $(wc -l < "$manifest" | tr -d ' ') clips to $manifest"
