# cROCK — tareas habituales.
#
#   make install   dependencias del sistema (SDL3 se compila desde fuente) y de Rust
#   make test      formato, lints y tests
#   make up        jugar en modo desarrollo (compilación rápida, métricas, recarga en caliente)
#   make build     binario optimizado + assets en dist/crock/
#
# Variables: ARGS="--chapter=1 --room=3" se pasa al juego en `make up`.

.DEFAULT_GOAL := help
.PHONY: help install test up build clean

CARGO ?= cargo
ARGS ?=
DIST := dist/crock
SUDO := $(shell [ "$$(id -u)" -eq 0 ] || echo sudo)

# Cabeceras que SDL3 necesita para compilarse en Linux (vídeo X11/Wayland, audio, mando).
APT_DEPS := build-essential cmake pkg-config libx11-dev libxext-dev libxcursor-dev libxi-dev \
	libxrandr-dev libxss-dev libxfixes-dev libxtst-dev libwayland-dev libxkbcommon-dev \
	libegl-dev libgl-dev libasound2-dev libpulse-dev libudev-dev libdbus-1-dev

help:
	@echo "make install   dependencias del sistema y de Rust"
	@echo "make test      formato, lints y tests"
	@echo "make up        jugar en modo desarrollo (ARGS=\"...\" para el juego)"
	@echo "make build     binario optimizado y assets en $(DIST)/"
	@echo "make clean     borra target/ y dist/"

install:
	@if command -v apt-get >/dev/null 2>&1; then \
		echo ">> apt: dependencias de SDL3"; \
		$(SUDO) apt-get update -q && $(SUDO) apt-get install -y -q $(APT_DEPS); \
	elif command -v brew >/dev/null 2>&1; then \
		echo ">> brew: cmake"; brew install cmake; \
	elif command -v dnf >/dev/null 2>&1; then \
		echo ">> dnf: dependencias de SDL3"; \
		$(SUDO) dnf install -y cmake gcc gcc-c++ libX11-devel libXext-devel libXcursor-devel \
			libXi-devel libXrandr-devel libXScrnSaver-devel libXfixes-devel libXtst-devel \
			wayland-devel libxkbcommon-devel mesa-libEGL-devel alsa-lib-devel \
			pulseaudio-libs-devel systemd-devel dbus-devel; \
	else \
		echo "!! No sé instalar dependencias en este sistema: hace falta cmake y un compilador de C."; \
	fi
	@command -v $(CARGO) >/dev/null 2>&1 || { \
		echo "!! Falta Rust: instálalo con https://rustup.rs y vuelve a ejecutar make install"; exit 1; }
	rustup component add clippy rustfmt 2>/dev/null || true
	$(CARGO) fetch

test:
	$(CARGO) fmt --all --check
	$(CARGO) clippy --workspace --all-targets -- -D warnings
	$(CARGO) test --workspace

up:
	$(CARGO) run -p crock -- --overlay $(ARGS)

build:
	$(CARGO) build --release -p crock
	rm -rf $(DIST)
	mkdir -p $(DIST)
	cp target/release/crock$(if $(filter Windows_NT,$(OS)),.exe,) $(DIST)/
	cp -r assets $(DIST)/assets
	rm -rf $(DIST)/assets/src $(DIST)/assets/replays
	@echo ">> listo: $(DIST)/ (ejecuta $(DIST)/crock)"

clean:
	$(CARGO) clean
	rm -rf dist
