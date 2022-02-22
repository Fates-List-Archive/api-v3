all:
	RUSTFLAGS="-C target-cpu=native" DATABASE_URL=postgresql://localhost/fateslist cargo build --profile release-lto $(CARGOFLAGS)
