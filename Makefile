.PHONY: build, run

run: build
	./target/debug/kubefs $$HOME/kubefstest

build:
	cargo build
