#include <stdio.h>
#include "binding.h"
#include "xil_printf.h"
#include "xil_io.h"

void print_char(char c) {
    putchar(c);
}

const UINTPTR BRAM_BASE = 0x80000000;

uint32_t test_read32() {
    return Xil_In32(BRAM_BASE);
}

void test_write32(uint32_t value) {
    Xil_Out32(BRAM_BASE, value);
}
