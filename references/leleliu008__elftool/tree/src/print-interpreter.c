#include <stdio.h>
#include <stdlib.h>

#include <elf.h>

#include "elftool.h"

int elftool_print_interpreter_handle_elf32(const unsigned char * elf) {
    Elf32_Ehdr * ehdr = (Elf32_Ehdr*)elf;

    for (Elf32_Half i = 0; i < ehdr->e_phnum; i++) {
        Elf32_Phdr * phdr = (Elf32_Phdr*)(elf + ehdr->e_phoff + i * ehdr->e_phentsize);

        if (phdr->p_type == PT_INTERP) {
            puts((char*)(elf + phdr->p_offset));
            break;
        }
    }

    return 0;
}

int elftool_print_interpreter_handle_elf32_swap(const unsigned char * elf) {
    Elf32_Ehdr * ehdr = (Elf32_Ehdr*)elf;

    uint16_t phnum = __builtin_bswap16(ehdr->e_phnum);
    uint32_t phoff = __builtin_bswap32(ehdr->e_phoff);
    uint16_t phentsize = __builtin_bswap16(ehdr->e_phentsize);

    for (uint16_t i = 0; i < phnum; i++) {
        Elf32_Phdr * phdr = (Elf32_Phdr*)(elf + phoff + i * phentsize);

        if (__builtin_bswap32(phdr->p_type) == PT_INTERP) {
            puts((char*)(elf + __builtin_bswap32(phdr->p_offset)));
            break;
        }
    }

    return 0;
}

int elftool_print_interpreter_handle_elf64(const unsigned char * elf) {
    Elf64_Ehdr * ehdr = (Elf64_Ehdr*)elf;

    for (Elf64_Half i = 0; i < ehdr->e_phnum; i++) {
        Elf64_Phdr * phdr = (Elf64_Phdr*)(elf + ehdr->e_phoff + i * ehdr->e_phentsize);

        if (phdr->p_type == PT_INTERP) {
            puts((char*)(elf + phdr->p_offset));
            break;
        }
    }

    return 0;
}

int elftool_print_interpreter_handle_elf64_swap(const unsigned char * elf) {
    Elf64_Ehdr * ehdr = (Elf64_Ehdr*)elf;

    uint16_t phnum = __builtin_bswap16(ehdr->e_phnum);
    uint64_t phoff = __builtin_bswap64(ehdr->e_phoff);
    uint16_t phentsize = __builtin_bswap16(ehdr->e_phentsize);

    for (uint16_t i = 0; i < phnum; i++) {
        Elf64_Phdr * phdr = (Elf64_Phdr*)(elf + phoff + i * phentsize);

        if (__builtin_bswap32(phdr->p_type) == PT_INTERP) {
            puts((char*)(elf + __builtin_bswap64(phdr->p_offset)));
            break;
        }
    }

    return 0;
}
