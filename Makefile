# Standard makefile system for all fates projects

RUSTFLAGS_LOCAL = "-C target-cpu=native $(RUSTFLAGS) -C link-arg=-fuse-ld=lld"
DATABASE_URL = "postgresql://localhost/fateslist"
CARGO_TARGET_GNU_LINKER="x86_64-unknown-linux-gnu-gcc"
BIN_NAME ?= fates
HOST ?= 100.87.78.60

all:
	@make cross
dev:
	DATABASE_URL=$(DATABASE_URL) RUSTFLAGS=$(RUSTFLAGS_LOCAL) cargo build
run:
	@mv -vf fates.new fates # If it exists
	./$(BIN_NAME)
flame:
	RUSTFLAGS=$RUSTFLAGS_LOCAL) DATABASE_URL=$(DATABASE_URL) cargo flamegraph $(CARGOFLAGS) --bin $(BIN_NAME)
cross:
	DATABASE_URL=$(DATABASE_URL) CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=$(CARGO_TARGET_GNU_LINKER) cargo build --target=x86_64-unknown-linux-gnu --release
push:
	scp -C -P 911 target/x86_64-unknown-linux-gnu/release/$(BIN_NAME) meow@$(HOST):api-v3/$(BIN_NAME).new
remote:
	ssh meow@$(HOST) -p 911
