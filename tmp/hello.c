#include <dlfcn.h>
#include <stdio.h>
int main(int argc, char **argv)
{
    const char *p = argc > 1 ? argv[1] : "libz.so.1";
    void *h = dlopen(p, RTLD_NOW);
    const char *(*f)(void);
    if (!h) { printf("dlopen failed: %s\n", dlerror()); return 1; }
    f = (const char *(*)(void))dlsym(h, "zlibVersion");
    if (!f) { printf("dlsym failed: %s\n", dlerror()); return 1; }
    printf("zlibVersion() = %s\n", f());
    return 0;
}
