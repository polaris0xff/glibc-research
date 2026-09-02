// mknod_linux.go — device and FIFO creation for layer extraction.
//
// SPDX-License-Identifier: MIT
package ociimg

import (
	"archive/tar"
	"os"
	"syscall"
)

func mknod(path string, typeflag byte, perm os.FileMode, major, minor int64) error {
	mode := uint32(perm)
	switch typeflag {
	case tar.TypeChar:
		mode |= syscall.S_IFCHR
	case tar.TypeBlock:
		mode |= syscall.S_IFBLK
	case tar.TypeFifo:
		mode |= syscall.S_IFIFO
	default:
		return os.ErrInvalid
	}
	return syscall.Mknod(path, mode, int(mkdev(major, minor)))
}

// mkdev packs a device number the way the kernel expects.
func mkdev(major, minor int64) uint64 {
	return uint64((major&0xfff)<<8) | uint64(minor&0xff) |
		uint64((minor&^0xff)<<12) | uint64((major&^0xfff)<<32)
}
