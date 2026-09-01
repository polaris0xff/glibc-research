#include <stdio.h>
#include <stdlib.h>

#include <elf.h>

#include "elftool.h"

int elftool_print_soname_handle_elf32(const unsigned char * elf) {
    Elf32_Ehdr * ehdr = (Elf32_Ehdr*)elf;

    const unsigned char * pDynamicSection = NULL;

    for (Elf32_Half i = 0; i < ehdr->e_phnum; i++) {
        Elf32_Phdr * phdr = (Elf32_Phdr*)(elf + ehdr->e_phoff + i * ehdr->e_phentsize);

        if (phdr->p_type == PT_DYNAMIC) {
            pDynamicSection = elf + phdr->p_offset;
            break;
        }
    }

    if (pDynamicSection == NULL) return 0;

    //////////////////////////////////////////

    Elf32_Addr addr = 0;
    Elf32_Dyn * dyn;

    for (size_t i = 0; i < 100; i++) {
        dyn = (Elf32_Dyn*)(pDynamicSection + i * sizeof(Elf32_Dyn));

        if (dyn->d_tag == DT_NULL) {
            break;
        }

        if (dyn->d_tag == DT_STRTAB) {
            addr = dyn->d_un.d_ptr;
            break;
        }
    }

    if (addr == 0) {
        return 1;
    }

    //////////////////////////////////////////

    const char * dynstr = NULL;

    for (Elf32_Half i = 0; i < ehdr->e_phnum; i++) {
        Elf32_Phdr * phdr = (Elf32_Phdr*)(elf + ehdr->e_phoff + i * ehdr->e_phentsize);

        if (phdr->p_type == PT_LOAD) {
            Elf32_Addr a = phdr->p_vaddr;
            Elf32_Addr b = phdr->p_memsz + a;

            if (addr >= a && addr < b) {
                dynstr = (const char *)elf + phdr->p_offset + (addr - a);
                break;
            }
        }
    }

    if (dynstr == NULL) {
        return 1;
    }

    //////////////////////////////////////////

    for (size_t i = 0; i < 100; i++) {
        dyn = (Elf32_Dyn*)(pDynamicSection + i * sizeof(Elf32_Dyn));

        if (dyn->d_tag == DT_NULL) {
            break;
        }

        if (dyn->d_tag == DT_SONAME) {
            puts(dynstr + dyn->d_un.d_val);
            break;
        }
    }

    return 0;
}

int elftool_print_soname_handle_elf32_swap(const unsigned char * elf) {
    Elf32_Ehdr * ehdr = (Elf32_Ehdr*)elf;

    const unsigned char * pDynamicSection = NULL;

    uint16_t phnum = __builtin_bswap16(ehdr->e_phnum);
    uint32_t phoff = __builtin_bswap32(ehdr->e_phoff);
    uint16_t phentsize = __builtin_bswap16(ehdr->e_phentsize);

    for (uint16_t i = 0; i < phnum; i++) {
        Elf32_Phdr * phdr = (Elf32_Phdr*)(elf + phoff + i * phentsize);

        if (__builtin_bswap32(phdr->p_type) == PT_DYNAMIC) {
            pDynamicSection = elf + __builtin_bswap32(phdr->p_offset);
            break;
        }
    }

    if (pDynamicSection == NULL) return 0;

    //////////////////////////////////////////

    Elf32_Addr addr = 0;
    Elf32_Dyn * dyn;

    for (size_t i = 0; i < 100; i++) {
        dyn = (Elf32_Dyn*)(pDynamicSection + i * sizeof(Elf32_Dyn));

        int32_t tag = __builtin_bswap32(dyn->d_tag);

        if (tag == DT_NULL) {
            break;
        }

        if (tag == DT_STRTAB) {
            addr = __builtin_bswap32(dyn->d_un.d_ptr);
            break;
        }
    }

    if (addr == 0) {
        return 1;
    }

    //////////////////////////////////////////

    const char * dynstr = NULL;

    for (uint16_t i = 0; i < phnum; i++) {
        Elf32_Phdr * phdr = (Elf32_Phdr*)(elf + phoff + i * phentsize);

        if (__builtin_bswap32(phdr->p_type) == PT_LOAD) {
            uint32_t a = __builtin_bswap32(phdr->p_vaddr);
            uint32_t b = __builtin_bswap32(phdr->p_memsz) + a;

            if (addr >= a && addr < b) {
                dynstr = (const char *)elf + __builtin_bswap32(phdr->p_offset) + (addr - a);
                break;
            }
        }
    }

    if (dynstr == NULL) {
        return 1;
    }

    //////////////////////////////////////////

    for (size_t i = 0; i < 100; i++) {
        dyn = (Elf32_Dyn*)(pDynamicSection + i * sizeof(Elf32_Dyn));

        int32_t tag = __builtin_bswap32(dyn->d_tag);

        if (tag == DT_NULL) {
            break;
        }

        if (tag == DT_SONAME) {
            puts(dynstr + __builtin_bswap32(dyn->d_un.d_val));
            break;
        }
    }

    return 0;
}

int elftool_print_soname_handle_elf64(const unsigned char * elf) {
    Elf64_Ehdr * ehdr = (Elf64_Ehdr*)elf;

    const unsigned char * pDynamicSection = NULL;

    for (Elf64_Half i = 0; i < ehdr->e_phnum; i++) {
        Elf64_Phdr * phdr = (Elf64_Phdr*)(elf + ehdr->e_phoff + i * ehdr->e_phentsize);

        if (phdr->p_type == PT_DYNAMIC) {
            pDynamicSection = elf + phdr->p_offset;
            break;
        }
    }

    if (pDynamicSection == NULL) return 0;

    //////////////////////////////////////////

    Elf64_Addr addr = 0;
    Elf64_Dyn * dyn;

    for (size_t i = 0; i < 100; i++) {
        dyn = (Elf64_Dyn*)(pDynamicSection + i * sizeof(Elf64_Dyn));

        if (dyn->d_tag == DT_NULL) {
            break;
        }

        if (dyn->d_tag == DT_STRTAB) {
            addr = dyn->d_un.d_ptr;
            break;
        }
    }

    if (addr == 0) {
        return 1;
    }

    //////////////////////////////////////////

    const char * dynstr = NULL;

    for (Elf64_Half i = 0; i < ehdr->e_phnum; i++) {
        Elf64_Phdr * phdr = (Elf64_Phdr*)(elf + ehdr->e_phoff + i * ehdr->e_phentsize);

        if (phdr->p_type == PT_LOAD) {
            Elf64_Addr a = phdr->p_vaddr;
            Elf64_Addr b = phdr->p_memsz + a;

            if (addr >= a && addr < b) {
                dynstr = (const char *)elf + phdr->p_offset + (addr - a);
                break;
            }
        }
    }

    if (dynstr == NULL) {
        return 1;
    }

    //////////////////////////////////////////

    for (size_t i = 0; i < 100; i++) {
        dyn = (Elf64_Dyn*)(pDynamicSection + i * sizeof(Elf64_Dyn));

        if (dyn->d_tag == DT_NULL) {
            break;
        }

        if (dyn->d_tag == DT_SONAME) {
            puts(dynstr + dyn->d_un.d_val);
            break;
        }
    }

    return 0;
}

int elftool_print_soname_handle_elf64_swap(const unsigned char * elf) {
    Elf64_Ehdr * ehdr = (Elf64_Ehdr*)elf;

    const unsigned char * pDynamicSection = NULL;

    uint16_t phnum = __builtin_bswap16(ehdr->e_phnum);
    uint64_t phoff = __builtin_bswap64(ehdr->e_phoff);
    uint16_t phentsize = __builtin_bswap16(ehdr->e_phentsize);

    for (uint16_t i = 0; i < phnum; i++) {
        Elf64_Phdr * phdr = (Elf64_Phdr*)(elf + phoff + i * phentsize);

        if (__builtin_bswap32(phdr->p_type) == PT_DYNAMIC) {
            pDynamicSection = elf + __builtin_bswap64(phdr->p_offset);
            break;
        }
    }

    if (pDynamicSection == NULL) return 0;

    //////////////////////////////////////////

    Elf64_Addr addr = 0;
    Elf64_Dyn * dyn;

    for (size_t i = 0; i < 100; i++) {
        dyn = (Elf64_Dyn*)(pDynamicSection + i * sizeof(Elf64_Dyn));

        int64_t tag = __builtin_bswap64(dyn->d_tag);

        if (tag == DT_NULL) {
            break;
        }

        if (tag == DT_STRTAB) {
            addr = __builtin_bswap64(dyn->d_un.d_ptr);
            break;
        }
    }

    if (addr == 0) {
        return 1;
    }

    //////////////////////////////////////////

    const char * dynstr = NULL;

    for (uint16_t i = 0; i < phnum; i++) {
        Elf64_Phdr * phdr = (Elf64_Phdr*)(elf + phoff + i * phentsize);

        if (__builtin_bswap32(phdr->p_type) == PT_LOAD) {
            Elf64_Addr a = __builtin_bswap64(phdr->p_vaddr);
            Elf64_Addr b = __builtin_bswap64(phdr->p_memsz) + a;

            if (addr >= a && addr < b) {
                dynstr = (const char *)elf + __builtin_bswap64(phdr->p_offset) + (addr - a);
                break;
            }
        }
    }

    if (dynstr == NULL) {
        return 1;
    }

    //////////////////////////////////////////

    for (size_t i = 0; i < 100; i++) {
        dyn = (Elf64_Dyn*)(pDynamicSection + i * sizeof(Elf64_Dyn));

        int64_t tag = __builtin_bswap64(dyn->d_tag);

        if (tag == DT_NULL) {
            break;
        }

        if (tag == DT_SONAME) {
            puts(dynstr + __builtin_bswap64(dyn->d_un.d_val));
            break;
        }
    }

    return 0;
}
