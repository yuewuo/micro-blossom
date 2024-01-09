#ifndef __BINDING_H_
#define __BINDING_H_

#include "inttypes.h"

/*
 * Provided by C
 */

extern void print_char(char c);
extern uint32_t test_read32();
extern void test_write32(uint32_t value);
extern uint64_t get_native_time();
extern float diff_native_time(uint64_t start, uint64_t end); // may not be accurate if time is too large, considering wrapping

/*
 * Provided by Rust
 */
extern void
rust_main();

#endif
