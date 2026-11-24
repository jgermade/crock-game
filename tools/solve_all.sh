#!/usr/bin/env bash
# Regenera las rutas de assets/replays/ con el solver, una por sala con salida.
# Úsalo tras cambiar assets/tuning.cfg o editar una sala: el test
# `shipped_replays_complete_their_rooms` comprueba que siguen funcionando.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release -q -p crock-solver
mkdir -p assets/replays
rooms=$(cat assets/levels/capitulo*.txt | grep -v '^#' | sed '/^\s*$/d')
# Con argumentos que no empiezan por '-', sólo esas salas (p. ej. c2-01-el-colmillo.tmj).
only=()
while [ $# -gt 0 ] && [ "${1#-}" = "$1" ]; do only+=("$1"); shift; done
[ ${#only[@]} -gt 0 ] && rooms="${only[*]}" || rooms="$rooms golpe.tmj cornisas.tmj"
for room in $rooms; do
  name="${room%.tmj}"
  # Si el haz por defecto se queda corto, se reintenta con uno más ancho.
  ./target/release/crock-solver "assets/levels/$room" --tuning=assets/tuning.cfg \
    --out="assets/replays/$name.rep" --secs=30 "$@" 2>/dev/null ||
  ./target/release/crock-solver "assets/levels/$room" --tuning=assets/tuning.cfg \
    --out="assets/replays/$name.rep" --secs=40 --beam=2000 "$@" 2>/dev/null
done
