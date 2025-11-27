.PHONY: all build test verify-abi clean

all: test verify-abi

build:
	cargo build --release

test:
	cargo test

test_abi: test_abi.cpp
	clang++ -std=c++14 -o test_abi test_abi.cpp

verify-abi: test_abi
	chmod +x verify_abi.sh
	./verify_abi.sh

clean:
	cargo clean
	rm -f test_abi test_load
