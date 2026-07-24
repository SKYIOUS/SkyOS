@echo off
REM Launch SARGA OS with ADE fixes — PS/2 mouse, 800x600 framebuffer
REM No -usb -device usb-tablet (kernel only supports PS/2 mouse)

set BOOTIMG=C:\Users\nanda\Desktop\Github\SKYIOUS KERNEL\target\x86_64-vahi\debug\bootimage-vahi_kernel.bin
set BIOS=C:\Users\nanda\Desktop\Github\SKYIOUS KERNEL\OVMF.fd
set LOG=C:\Users\nanda\Desktop\Github\SKYIOUS KERNEL\qemu_display.log

del "%LOG%" 2>nul

start "" "C:\Program Files\qemu\qemu-system-x86_64.exe" ^
  -bios "%BIOS%" ^
  -drive if=ide,format=raw,file="%BOOTIMG%" ^
  -m 512M -smp 1 -vga std -cpu max ^
  -no-reboot -k en-us -display sdl ^
  -serial file:"%LOG%"

echo QEMU started. Boot log at %LOG%
