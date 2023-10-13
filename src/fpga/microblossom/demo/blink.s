.section .text
.globl _start
_start:
    li x5, 0x1
    li x28, 0x20
    li x6, 0x10000000
    
outer:
    li x7, 0x40000
loop0:
    sub x7, x7, x5
    bne x7, x0, loop0
    sw x28, 0(x6)
    srli x28, x28, 1
    beq x28, x0, reset
    j outer
reset:
    li x28, 0x20
    j outer
    