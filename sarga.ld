ENTRY(_start)

SECTIONS {
    . = 0x400000;

    .text : {
        KEEP(*(.text._start))
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    .data.rel.ro : {
        *(.data.rel.ro .data.rel.ro.*)
    }

    .dynamic : {
        *(.dynamic)
    }

    .got : {
        *(.got)
        *(.got.plt)
    }

    .plt : {
        *(.plt)
        *(.plt.*)
    }

    .data : {
        *(.data .data.*)
    }

    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    }

    /DISCARD/ : {
        *(.note.gnu.property)
        *(.comment)
        *(.note.GNU-stack)
    }
}
