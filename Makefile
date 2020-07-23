.PHONY: build, run

run: build
	RUST_LOG=info ./target/debug/kubefs $$HOME/kubefstest

build:
	cargo build

install: build
	cp ./target/debug/kubefs /usr/local/bin
