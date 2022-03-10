root := justfile_directory()
rotthdir := (root + "/rotth-src/examples")
compiler := (root + "/rotth-src/rotth")

default:
  @just --list

build FILE:
    cargo run --release -- --compile -- {{rotthdir}}/{{FILE}}.rh
    nasm -g -F dwarf -f elf64 {{root}}/print.asm -o {{root}}/print.o
    nasm -g -F dwarf -f elf64 {{rotthdir}}/{{FILE}}.asm -o {{rotthdir}}/{{FILE}}.o
    ld -o {{rotthdir}}/{{FILE}} {{rotthdir}}/{{FILE}}.o {{root}}/print.o

run FILE: (build FILE)
    {{rotthdir}}/{{FILE}}

compiler:
    cargo run --release -- --compile -- {{compiler}}.rh
    nasm -g -F dwarf -f elf64 {{root}}/print.asm -o {{root}}/print.o
    nasm -g -F dwarf -f elf64 {{compiler}}.asm -o {{compiler}}.o
    ld -o {{compiler}} {{compiler}}.o {{root}}/print.o

clean-run FILE: clean (run FILE)

clean:
    find {{rotthdir}} -type f  ! -name "*.rh"  -delete