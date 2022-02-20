all:
	DATABASE_URL=postgresql://localhost/fateslist cargo build --profile release-lto $(CARGOFLAGS)
fast:
	DATABASE_URL=postgresql://localhost/fateslist cargo build --profile release-fast $(CARGOFLAGS)
