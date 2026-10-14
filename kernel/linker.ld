/* Ferrix kernel image layout for the Limine boot protocol.
 *
 * The kernel is linked into the top 2 GiB of the address space, which is where
 * Limine maps it and where the x86_64 "kernel" code model expects to live.
 */

OUTPUT_FORMAT(elf64-x86-64)
ENTRY(kmain)

/* One segment per set of permissions, so Limine can map text read-execute and
 * everything else without execute permission. */
PHDRS
{
    text    PT_LOAD;
    rodata  PT_LOAD;
    data    PT_LOAD;
}

SECTIONS
{
    . = 0xffffffff80000000;

    .text : {
        *(.text .text.*)
    } :text

    . = ALIGN(CONSTANT(MAXPAGESIZE));

    .rodata : {
        *(.rodata .rodata.*)
    } :rodata

    . = ALIGN(CONSTANT(MAXPAGESIZE));

    .data : {
        *(.data .data.*)
    } :data

    /* Limine writes response pointers back into these structures, so they must
     * land in a writable segment. The markers must bracket the requests. */
    .requests : {
        KEEP(*(.requests_start_marker))
        KEEP(*(.requests))
        KEEP(*(.requests_end_marker))
    } :data

    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    } :data

    /DISCARD/ : {
        *(.eh_frame*)
        *(.note .note.*)
    }
}
