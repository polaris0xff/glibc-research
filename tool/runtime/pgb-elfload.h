/* pgb-elfload.h -- the shape of the compiled-in ELF loader and its providers.
 *
 * Shared by pgb-elfload.c, which is the loader; by pgb-dlopen.c, which falls
 * through to it; and by pgb-provider-table.c, which `pgb` GENERATES per build.
 * Three files that have to agree on a layout get one header, or they agree
 * until somebody edits one of them.
 *
 * SPDX-License-Identifier: MIT
 */
#ifndef PGB_ELFLOAD_H
#define PGB_ELFLOAD_H

#include <stddef.h>

/* One symbol this executable can define for a loaded object. `addr` is NULL
 * when the name was in the generated list but the link did not pull the
 * archive member that defines it -- see the weak-reference note below. */
struct pgb_provider_sym {
    const char *name;
    void       *addr;
};

/* ⭐ WEAK, AND THE WEAKNESS IS THE MECHANISM, NOT A CONVENIENCE.
 *
 * The generated table takes the address of every name it lists. A STRONG
 * reference to each would force the archive member defining it out of libc.a
 * and into the link -- all of libc.a, for every program, whether or not any
 * plugin ever needed it. A WEAK undefined reference does not pull an archive
 * member: it resolves to 0 if nothing else in the link needed that symbol,
 * and to the real address if something did.
 *
 * So the table costs its own strings and pointers and nothing else, and the
 * decision about how much of libc.a to link is made SEPARATELY, by the -u
 * list `pgb` passes. That separation is what makes the size cost a dial
 * rather than a constant. Measured in experiments/76-.
 */
extern const struct pgb_provider_sym pgb_provider_syms[] __attribute__((weak));

/* Sonames this executable satisfies internally. A DT_NEEDED naming one of
 * these is answered out of the provider table instead of being mapped, which
 * is what keeps a second libc from entering the process. NULL-terminated. */
extern const char *const pgb_provider_sonames[] __attribute__((weak));

/* The loader. pgb-dlopen.c calls these after its own compiled-in plugin table
 * misses; nothing else should. Returns NULL / sets the error string. */
void       *pgb_elf_dlopen(const char *path, int flags);
void       *pgb_elf_dlsym(void *handle, const char *name);
int         pgb_elf_dlclose(void *handle);
const char *pgb_elf_dlerror(void);

/* ⭐ solo's mechanism 5: make "what did this binary satisfy internally"
 * observable from INSIDE, beside the syscall trace pgb verify takes from
 * outside. Two independent instruments on the same question. Writes an
 * ldd-format listing to fd, including names served WITHOUT a mapping. */
void pgb_elf_trace_loaded(int fd);

/* Non-zero once a provider table is compiled in and non-empty. pgb-dlopen.c
 * uses it to decide whether falling through is even possible, so that a build
 * without the loader keeps its existing honest error instead of a new one. */
int pgb_elf_available(void);

#endif /* PGB_ELFLOAD_H */
