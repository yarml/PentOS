.PHONY: test
test:
	cargo test -p test

.PHONY: doc
doc:
	cargo doc --workspace --release --no-deps
	echo '<meta http-equiv="refresh" content="0; url=pentos/">' > target/doc/index.html