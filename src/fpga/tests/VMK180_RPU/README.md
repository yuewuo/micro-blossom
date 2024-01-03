# VMK180 RPU

## Learning note

According to UG1304 (page 18), we should use either
- Option1: Split mode
    - for dual-core parallel execution
- Option 2: Split mode, only one core
    - for single-core 256KB access of TCM

The embedded standalone drivers are listed in `/tools/Xilinx/Vitis/2023.2/data/embeddedsw/XilinxProcessorIPLib/drivers`.

The embedded libraries are listed in `/tools/Xilinx/Vitis/2023.2/data/embeddedsw/lib/sw_services`

When you plugin the FPGA USB, multiple UART ports like `/dev/ttyUSBx` will show up.
It's unclear how to find the one that APU/RPU outputs to, so we need to monitor all of them until we find the right one.
The command is `sudo picocom /dev/ttyUSB6 -b 115200 --imap lfcrlf`.
It will convert `\n` to `\r\n` so that programs with `\n` outputs will show correctly.


successfully run a hello world and Dhrystone benchmark program on VMK180
    1.5765 DMIPS/MHz for A72, 0.5477DMIPS/MHz for R5F
    when using TCM for the R5F CPU, the Dhrystone benchmark boosts to 12.8598DMIPS/MHz!!!!
    remember to set them in the application component -> Sources -> src -> lscript.ld
