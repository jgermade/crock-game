#!/usr/bin/env bash
# Regenera las salas de Tiled (assets/levels/*.tmj) desde sus bocetos ASCII
# (assets/src/salas/*.txt). Ojo: sobrescribe los .tmj; si has editado alguno
# en Tiled, lleva antes los cambios al boceto (o no lo regeneres).
set -euo pipefail
cd "$(dirname "$0")/.."
declare -A NAMES=(
  [golpe]="Sala de pruebas: golpe"
  [movimiento]="Sala de pruebas: movimiento"
  [cornisas]="Sala de pruebas: cornisas"
  [c1-01-entrada]="Entrada"
  [c1-02-primera-roca]="La primera roca"
  [c1-03-el-foso]="El foso"
  [c1-04-bichos]="Bichos"
  [c1-05-la-chimenea]="La chimenea"
)
for src in assets/src/salas/*.txt; do
  name=$(basename "$src" .txt)
  python3 tools/ascii2tmj.py "$src" "assets/levels/$name.tmj" "${NAMES[$name]:-$name}"
  echo "assets/levels/$name.tmj"
done
