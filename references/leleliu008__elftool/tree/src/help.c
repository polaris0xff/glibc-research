#include <stdio.h>

#include <unistd.h>

#include "elftool.h"

#define COLOR_GREEN  "\033[0;32m"
#define COLOR_OFF    "\033[0m"

int elftool_print_help() {
    const char * p;

    if (isatty(STDOUT_FILENO)) {
        p = ""
        COLOR_GREEN
        "A command-line tool to manipulate ELF files\n\n"
        "elftool <ACTION> [ARGUMENT...]\n\n"
        "elftool --help\n"
        "elftool -h\n"
        COLOR_OFF
        "    show help of this command.\n\n"
        COLOR_GREEN
        "elftool --version\n"
        "elftool -V\n"
        COLOR_OFF
        "    show version of this command.\n\n"
        COLOR_GREEN
        "elftool print-interp <FILEPATH>\n"
        COLOR_OFF
        "    print the interpreter of the given ELF file.\n\n"
        COLOR_GREEN
        "elftool print-soname <FILEPATH>\n"
        COLOR_OFF
        "    print the soname of the given ELF file.\n\n"
        COLOR_GREEN
        "elftool print-needed <FILEPATH>\n"
        COLOR_OFF
        "    print all needed dynamic libraries of the given ELF file.\n\n"
        COLOR_GREEN
        "elftool print-rpath <FILEPATH>\n"
        COLOR_OFF
        "    print all RUNPATH encoded in the given ELF file.\n"
        ;

        printf("%s\n", p);
    } else {
        p = ""
        "A command-line tool to manipulate ELF files\n\n"
        "elftool <ACTION> [ARGUMENT...]\n\n"
        "elftool --help\n"
        "elftool -h\n"
        "    show help of this command.\n\n"
        "elftool --version\n"
        "elftool -V\n"
        "    show version of this command.\n\n"
        "elftool print-interp <FILEPATH>\n"
        "    print the interpreter of the given ELF file.\n\n"
        "elftool print-soname <FILEPATH>\n"
        "    print the soname of the given ELF file.\n\n"
        "elftool print-needed <FILEPATH>\n"
        "    print all needed dynamic libraries of the given ELF file.\n\n"
        "elftool print-rpath <FILEPATH>\n"
        "    print all RUNPATH encoded in the given ELF file.\n"
        ;
    }

    printf("%s\n", p);

    return 0;
}
