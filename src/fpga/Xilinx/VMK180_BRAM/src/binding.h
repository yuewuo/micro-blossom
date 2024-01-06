#ifndef __BINDING_H_
#define __BINDING_H_

/*
 * Provided by C
 */

void print_char(char c);
uint32_t test_read32();
void test_write32(uint32_t value);

/*
 * Provided by Rust
 */
extern void rust_main();


#endif
