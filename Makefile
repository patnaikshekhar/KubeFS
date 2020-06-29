.PHONY: build, run

run: build
	fusermount -uz $$HOME/kubefstest && \
	./target/debug/kubefs $$HOME/kubefstest

build:
	cargo build
