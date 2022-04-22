all:
	RUSTFLAGS="-C target-cpu=native $(RUSTFLAGS)" DATABASE_URL=postgresql://localhost/fateslist cargo build $(CARGOFLAGS) --release
run:
	./target/release/fates $(FATESFLAGS)
