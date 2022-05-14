all:
	RUSTFLAGS="-C target-cpu=native $(RUSTFLAGS) -C link-arg=-fuse-ld=mold" DATABASE_URL=postgresql://localhost/fateslist cargo build $(CARGOFLAGS) --release
run:
	./target/release/fates $(FATESFLAGS)
flame:
	RUSTFLAGS="-C target-cpu=native $(RUSTFLAGS) -C link-arg=-fuse-ld=mold" DATABASE_URL=postgresql://localhost/fateslist cargo flamegraph $(CARGOFLAGS) --bin fates