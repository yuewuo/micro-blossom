# Frequency Evaluation


Understand how the clock frequency changes with the code distance.

According to [../../src/cpu/embedded/src/mains/test_bram.rs](), the write operation takes 26ns and the read operation 
takes 127ns. The write-then-read operation takes 217ns which is the normal use case.
This is tested using a clock frequency of 200MHz.


