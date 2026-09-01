#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <unistd.h>
#include <fcntl.h>

#include <sys/stat.h>
#include <sys/mman.h>

#include <elf.h>

#include "elftool.h"

typedef struct {
    const char * arg;

    int (*handle_elf32)(const unsigned char * elf);
    int (*handle_elf64)(const unsigned char * elf);

    int (*handle_elf32_swap)(const unsigned char * elf);
    int (*handle_elf64_swap)(const unsigned char * elf);
} Action;

int main(int argc, char* argv[]) {
    if (argc == 1) {
        elftool_print_help();
        return ELFTOOL_OK;
    }

    if (argv[1][0] == '\0') {
        elftool_print_help();
        return ELFTOOL_ERROR;
    }

    if ((strcmp(argv[1], "-h") == 0) || (strcmp(argv[1], "--help") == 0)) {
        elftool_print_help();
        return ELFTOOL_OK;
    }

    if ((strcmp(argv[1], "-V") == 0) || (strcmp(argv[1], "--version") == 0)) {
        printf("%s\n", ELFTOOL_VERSION_STRING);
        return ELFTOOL_OK;
    }

    const Action actions[] = {
        {
            "print-interp",
            elftool_print_interpreter_handle_elf32,
            elftool_print_interpreter_handle_elf64,
            elftool_print_interpreter_handle_elf32_swap,
            elftool_print_interpreter_handle_elf64_swap
        },
        {
            "print-needed",
            elftool_print_needed_handle_elf32,
            elftool_print_needed_handle_elf64,
            elftool_print_needed_handle_elf32_swap,
            elftool_print_needed_handle_elf64_swap
        },
        {
            "print-soname",
            elftool_print_soname_handle_elf32,
            elftool_print_soname_handle_elf64,
            elftool_print_soname_handle_elf32_swap,
            elftool_print_soname_handle_elf64_swap
        },
        {
            "print-rpath",
            elftool_print_rpath_handle_elf32,
            elftool_print_rpath_handle_elf64,
            elftool_print_rpath_handle_elf32_swap,
            elftool_print_rpath_handle_elf64_swap
        },
        {NULL, NULL, NULL, NULL, NULL}
    };

    for (int i = 0; ; i++) {
        if (actions[i].arg == NULL) {
            fprintf(stderr, "%s: unrecognized action: %s\n", argv[0], argv[1]);
            elftool_print_help();
            return ELFTOOL_ERROR_ARG_IS_UNKNOWN;
        }

        if (strcmp(argv[1], actions[i].arg) != 0) {
            continue;
        }

        const char * fp = argv[2];

        if (fp == NULL) {
            fprintf(stderr, "Usage : %s %s <FILEPATH>, <FILEPATH> is unspecified.\n", argv[0], argv[1]);
            return ELFTOOL_ERROR_ARG_IS_NULL;
        }

        if (fp[0] == '\0') {
            fprintf(stderr, "Usage : %s %s <FILEPATH>, <FILEPATH> should be a non-empty string.\n", argv[0], argv[1]);
            return ELFTOOL_ERROR_ARG_IS_EMPTY;
        }

        int fd = open(fp, O_RDONLY);

        if (fd == -1) {
            perror(fp);
            return ELFTOOL_ERROR;
        }

        struct stat st;

        if (fstat(fd, &st) == -1) {
            perror(fp);
            close(fd);
            return ELFTOOL_ERROR;
        }

        if (st.st_size < 52) {
            fprintf(stderr, "NOT an ELF file: %s\n", fp);
            close(fd);
            return ELFTOOL_ERROR_NOT_ELF_FILE;
        }

        ///////////////////////////////////////////////////////////

        unsigned char a[6];

        ssize_t readBytes = read(fd, a, 6);

        if (readBytes == -1) {
            perror(fp);
            close(fd);
            return ELFTOOL_ERROR;
        }

        if (readBytes != 6) {
            perror(fp);
            close(fd);
            fprintf(stderr, "not fully read.\n");
            return ELFTOOL_ERROR;
        }

        ///////////////////////////////////////////////////////////

        // https://www.sco.com/developers/gabi/latest/ch4.eheader.html
        if ((a[0] != 0x7F) || (a[1] != 0x45) || (a[2] != 0x4C) || (a[3] != 0x46)) {
            fprintf(stderr, "NOT an ELF file: %s\n", fp);
            close(fd);
            return ELFTOOL_ERROR_NOT_ELF_FILE;
        }

        ///////////////////////////////////////////////////////////

        int swap = 0;

        switch (a[5]) {
            case ELFDATA2LSB:
#if __BYTE_ORDER__ == __ORDER_BIG_ENDIAN__
                swap = 1;
#endif
                break;
            case ELFDATA2MSB:
#if __BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__
                swap = 1;
#endif
                break;
            default:
                fprintf(stderr, "Invalid ELF file: %s\n", fp);
                return ELFTOOL_ERROR_BROKEN_ELF_FILE;
        }

        ///////////////////////////////////////////////////////////

        void * p = mmap(NULL, st.st_size, PROT_READ, MAP_PRIVATE, fd, 0);

        if (p == MAP_FAILED) {
            perror(fp);
            close(fd);
            return ELFTOOL_ERROR;
        }

        ///////////////////////////////////////////////////////////

        close(fd);

        int ret;

        switch (a[4]) {
            case ELFCLASS32:
                if (swap == 0) {
                    ret = actions[i].handle_elf32(p);
                } else {
                    ret = actions[i].handle_elf32_swap(p);
                }
                break;
            case ELFCLASS64:
                if (swap == 0) {
                    ret = actions[i].handle_elf64(p);
                } else {
                    ret = actions[i].handle_elf64_swap(p);
                }
                break;
            default: 
                fprintf(stderr, "Invalid ELF file: %s\n", fp);
                ret = ELFTOOL_ERROR_BROKEN_ELF_FILE;
        }

        munmap(p, st.st_size);

        return ret;
    }
}
