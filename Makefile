all:
	DATABASE_URL=postgresql://localhost/fateslist cargo build --release $(CARGOFLAGS)
