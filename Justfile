pre-publish:
	cargo test
	cargo clippy --color=always --all-targets -- -D warnings
	cargo fmt
	cargo doc

publish:
	cargo publish --dry-run --workspace

really-publish:
	cargo publish --workspace
