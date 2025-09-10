pre-publish:
	cargo test
	cargo clippy --color=always --all-targets -- -D warnings
	cargo fmt
	cargo doc

publish:
	cargo publish --dry-run -p pipewire-native-macros
	cargo publish --dry-run -p pipewire-native-spa
	cargo publish --dry-run -p pipewire-native
	cargo publish --dry-run -p pipewire-native-tools

really-publish:
	cargo publish -p pipewire-native-macros
	cargo publish -p pipewire-native-spa
	cargo publish -p pipewire-native
	cargo publish -p pipewire-native-tools
