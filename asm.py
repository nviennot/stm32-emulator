#!/usr/bin/env python3
# pip3 install keystone-engine
#
# Example:
# $ ./asm.py < <(echo 'mov r0, #-1; bx lr')
# [0x4f, 0xf0, 0xff, 0x30, 0x70, 0x47]
from keystone import *
import sys
import fileinput

ks = Ks(KS_ARCH_ARM, keystone.KS_MODE_THUMB | keystone.KS_MODE_LITTLE_ENDIAN)
code = ''.join(fileinput.input())
encoded_asm, num_inst = ks.asm(code)
if not encoded_asm:
    raise RuntimeError("no assembly")
print('[', ', '.join([hex(b) for b in encoded_asm]), ']', sep='')
