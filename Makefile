test: test-rust test-ocaml

test-rust:
	@cargo test --features=link -- --test-threads=1

test-ocaml:
	@dune runtest --root=test --force --no-buffer

utop:
	@dune utop --root=test

clean:
	cargo clean
	dune clean --root=test

publish:
	cd sys && cargo package && cargo publish && sleep 5
	cd derive && cargo package && cargo publish && sleep 5
	cargo package && cargo publish

.PHONY: test clean
