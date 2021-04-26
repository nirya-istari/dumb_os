
.phony: image run run-uefi debug gdb

image: 
	cd builder && cargo run	

run: 
	qemu-system-x86_64 -drive file=./target/x86_64-dumb_os/debug/boot-bios-dumb_os.img,format=raw -serial stdio -s

# Size is 128KiB
ovmf_vars.fd:	
	truncate --size=131072 ovmf_vars.fd

run-ueif: ovmf_vars.fd
	qemu-system-x86_64 \
		-enable-kvm \
		-machine q35 \
		-cpu host \
		-drive if=pflash,format=raw,readonly,file=/usr/share/edk2-ovmf/x64/OVMF.fd \
		-drive if=pflash,format=raw,file=ovmf_vars.fd \
		-drive file=./target/x86_64-dumb_os/debug/boot-uefi-dumb_os.img,format=raw \
		-serial stdio \
		-s

debug:
	qemu-system-x86_64 -drive file=./target/x86_64-dumb_os/debug/boot-bios-dumb_os.img,format=raw -serial stdio -s -S

gdb:
	gdb "target/x86_64-dumb_os/debug/dumb_os" -ex "target remote :1234"
