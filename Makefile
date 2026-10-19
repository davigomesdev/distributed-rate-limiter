.PHONY: fmt clippy test run bench docker-up docker-down clean

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets -- -D warnings

test:
	cargo test --all-targets

run:
	cargo run --release

bench:
	cargo bench

docker-up:
	docker compose up --build -d

docker-down:
	docker compose down

clean:
	cargo clean
