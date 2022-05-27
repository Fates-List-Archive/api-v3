all:
	RUSTFLAGS="-C target-cpu=native $(RUSTFLAGS) -C link-arg=-fuse-ld=lld" DATABASE_URL=postgresql://localhost/fateslist cargo build $(CARGOFLAGS) --release
run:
	./target/release/fates $(FATESFLAGS)
flame:
	RUSTFLAGS="-C target-cpu=native $(RUSTFLAGS) -C link-arg=-fuse-ld=lld" DATABASE_URL=postgresql://localhost/fateslist cargo flamegraph $(CARGOFLAGS) --bin fates
cross:
	CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-unknown-linux-gnu-gcc cargo build --target=x86_64-unknown-linux-gnu --release
push:
	scp -P 911 target/x86_64-unknown-linux-gnu/release/fates meow@100.87.78.60:api-v3
