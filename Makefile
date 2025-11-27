all:
	cd libbambu-networking-api && make build

clean:
	cd libbambu-networking-api && cargo clean
	cd core && cargo clean
