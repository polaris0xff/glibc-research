#ifndef ELFTOOL_H
#define ELFTOOL_H

#define ELFTOOL_OK                     0
#define ELFTOOL_ERROR                  1

#define ELFTOOL_ERROR_ARG_IS_NULL      2
#define ELFTOOL_ERROR_ARG_IS_EMPTY     3
#define ELFTOOL_ERROR_ARG_IS_INVALID   4
#define ELFTOOL_ERROR_ARG_IS_UNKNOWN   5

#define ELFTOOL_ERROR_NOT_ELF_FILE    10
#define ELFTOOL_ERROR_BROKEN_ELF_FILE 11

#ifndef ELFTOOL_VERSION_STRING
#define ELFTOOL_VERSION_STRING "1.0.0"
#endif

int elftool_print_help();

//////////////////////////////////////////

int elftool_print_interpreter_handle_elf32(const unsigned char * elf);
int elftool_print_interpreter_handle_elf64(const unsigned char * elf);

int elftool_print_interpreter_handle_elf32_swap(const unsigned char * elf);
int elftool_print_interpreter_handle_elf64_swap(const unsigned char * elf);

//////////////////////////////////////////

int elftool_print_needed_handle_elf32(const unsigned char * elf);
int elftool_print_needed_handle_elf64(const unsigned char * elf);

int elftool_print_needed_handle_elf32_swap(const unsigned char * elf);
int elftool_print_needed_handle_elf64_swap(const unsigned char * elf);

//////////////////////////////////////////

int elftool_print_soname_handle_elf32(const unsigned char * elf);
int elftool_print_soname_handle_elf64(const unsigned char * elf);

int elftool_print_soname_handle_elf32_swap(const unsigned char * elf);
int elftool_print_soname_handle_elf64_swap(const unsigned char * elf);

//////////////////////////////////////////

int elftool_print_rpath_handle_elf32(const unsigned char * elf);
int elftool_print_rpath_handle_elf64(const unsigned char * elf);

int elftool_print_rpath_handle_elf32_swap(const unsigned char * elf);
int elftool_print_rpath_handle_elf64_swap(const unsigned char * elf);

#endif
