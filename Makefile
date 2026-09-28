fmt:
	cargo fmt --all

.PHONY: fmt-json
fmt-json:
	npx --yes prettier@3.6.2 --parser json --object-wrap collapse --print-width 120 --write \
		tests/fixtures/saved_group_server_payloads.json benchmarks/saved-group-v2-results.json

clippy:
	cargo clippy --all-targets --all-features -- -Dwarnings -Dclippy::unwrap_used

test:
	@cargo watch -q -c -x 'nextest run ${FILTER} --no-capture'

test_oneshot:
	cargo nextest run
