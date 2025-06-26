.PHONY: build test check clean install examples help

# Build the project
build:
	cargo build --release

# Run tests
test:
	cargo test

# Check code quality
check:
	cargo clippy -- -D warnings
	cargo fmt --check

# Clean build artifacts
clean:
	cargo clean

# Install globally
install: build
	cargo install --path .

# Run examples
examples: build
	@echo "=== Simple Process ==="
	./target/release/bpmncode check examples/simple.bpmn -v
	@echo
	@echo "=== Complex Process ==="
	./target/release/bpmncode check examples/complex.bpmn -v
	@echo
	@echo "=== Multi-file Process ==="
	./target/release/bpmncode check examples/multi_file/main.bpmn -v
	@echo
	@echo "=== Compile Examples ==="
	./target/release/bpmncode compile examples/simple.bpmn --check -v
	./target/release/bpmncode compile examples/complex.bpmn --format ast -v

# Show help
help:
	@echo "Available targets:"
	@echo "  build     - Build the project"
	@echo "  test      - Run tests"
	@echo "  check     - Check code quality"
	@echo "  fmt       - Format code"
	@echo "  clean     - Clean build artifacts"
	@echo "  install   - Install globally"
	@echo "  examples  - Run example files"
	@echo "  help      - Show this help"

# Default target
all: check test build
