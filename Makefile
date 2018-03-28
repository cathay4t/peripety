all:
	cargo build --all

tidy:
	cargo fmt

run:
	cd src/peripety && cargo run

run_stdout:
	cd src/plugins/stdout/ && caro run

test:
	./tests/scsi.sh
	./tests/lvm_tp.sh
	./tests/dmmp.sh
