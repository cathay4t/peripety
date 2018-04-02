all:
	cargo build --all

tidy:
	cargo fmt

run: all
	sudo mkdir /var/run/peripety/ || :
	sudo chown $$UID /var/run/peripety/
	cd ./src/peripetyd && cargo run

run_stdout: all
	cd src/plugins/stdout/ && cargo run

test:
	# ./tests/scsi.sh
	# ./tests/lvm_tp.sh
	./tests/dmmp.sh

clean:
	cargo clean
