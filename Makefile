
image: 
	cd builder && cargo run	

run: 
	qemu-system-x86_64 -drive file=./target/x86_64-dumb_os/debug/boot-bios-dumb_os.img,format=raw -serial stdio -s

debug:
	qemu-system-x86_64 -drive file=./target/x86_64-dumb_os/debug/boot-bios-dumb_os.img,format=raw -serial stdio -s -S

gdb:
	gdb "target/x86_64-dumb_os/debug/dumb_os" -ex "target remote :1234"
