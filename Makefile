RUSTFLAGS_LOCAL = "-C target-cpu=native $(RUSTFLAGS) -C link-arg=-fuse-ld=lld"
DATABASE_URL = "postgresql://localhost/fateslist"
CARGO_TARGET_GNU_LINKER=x86_64-unknown-linux-gnu-gcc

export DATABASE_URL

all:
	@make cross
dev:
	RUSTFLAGS=$RUSTFLAGS_LOCAL cargo build
run:
	@mv -vf fates.new fates # If it exists
	./fates
flame:
	RUSTFLAGS=$RUSTFLAGS_LOCAL cargo flamegraph $(CARGOFLAGS) --bin fates
cross:
	CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=$CARGO_TARGET_GNU_LINKER cargo build --target=x86_64-unknown-linux-gnu --release
push:
	scp -P 911 target/x86_64-unknown-linux-gnu/release/fates meow@100.87.78.60:api-v3/fates.new
