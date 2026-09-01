# elftool

A command-line tool to manipulate ELF files.

## Build elftool via [ppkg](https://github.com/leleliu008/ppkg)

```bash
ppkg install elftool
```

## Build elftool via [xcpkg](https://github.com/leleliu008/xcpkg)

```bash
xcpkg install elftool
```

## Build elftool via [ndk-pkg](https://github.com/leleliu008/ndk-pkg)

```bash
ndk-pkg install elftool
```

## Build elftool via [./build.sh](https://github.com/leleliu008/elftool/blob/master/build.sh)

`./build.sh` accepts any compiler options. If no compiler options are given, `-std=gnu99 -Os -flto -Wl,-s -o elftool` will be passed to the C Compiler.

`./build.sh` will install [gcc](https://gcc.gnu.org/) via your system's package manager if no C Compiler found.

## Build elftool using C Compiler directly

```bash
cc -o elftool -Isrc src/*.c
```

**Note:** This tool use `elf.h`, most systems have it. If your system does NOT have it, you can get it from <https://sourceware.org/git/?p=glibc.git;a=blob_plain;f=elf/elf.h;hb=HEAD> and put it in `src` directory.

## elftool command usage

- **show help of this command**

    ```bash
    elftool -h
    elftool --help
    ```

- **show version of this command**

    ```bash
    elftool -V
    elftool --version
    ```

- **print the interpreter of the given ELF file**

    ```bash
    elftool print-interp /usr/bin/ls
    ```

- **print the soname of the given ELF file**

    ```bash
    elftool print-soname /usr/lib64/ld-linux-x86-64.so.2
    ```

- **print all needed dynamic libraries of the given ELF file**

    ```bash
    elftool print-needed /usr/bin/ls
    ```

- **print all RUNPATH encoded in the given ELF file**

    ```bash
    elftool print-rpath /usr/bin/ls
    ```
