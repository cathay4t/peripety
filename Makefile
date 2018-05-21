all:
	cargo build --all

tidy:
	cargo fmt

run: all
	sudo target/debug/peripetyd

check:
	./tests/scsi.sh
	./tests/lvm_tp.sh
	./tests/dmmp.sh

clean:
	cargo clean
