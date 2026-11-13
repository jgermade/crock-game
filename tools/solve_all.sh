#!/usr/bin/env bash
# Regenera las rutas de assets/replays/ con el solver, una por sala con salida.
# Úsalo tras cambiar assets/tuning.cfg o editar una sala: el test
# `shipped_replays_complete_their_rooms` comprueba que siguen funcionando.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release -q -p crock-solver
mkdir -p assets/replays
rooms=$(grep -v '^#' assets/levels/capitulo1.txt | sed '/^\s*$/d')
for room in $rooms golpe.tmj; do
  name="${room%.tmj}"
  ./target/release/crock-solver "assets/levels/$room" --tuning=assets/tuning.cfg \
    --out="assets/replays/$name.rep" --secs=30 "$@" 2>/dev/null
done
