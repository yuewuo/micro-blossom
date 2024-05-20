# VMK180 RPU

## Usage

```sh
# create Vivado project in `./vmk180bram`
vivado -s create_vivado.tcl
# create Vitis workspace in `./vmk180bram_vitis`
vitis -s create_vitis.py
```

If you want to hide Vivado GUI, use `vivado -mode batch -s create_vivado.tcl` instead.
If you want to use interactive command, use `vitis -i` instead and then run `run create_vitis.py` inside the Python shell.
If you want to open the Vitis GUI, use `vitis -w ./vmk180bram_vitis` after you run the `create_vitis.py` manually.

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
    1.5765 DMIPS/MHz for A72 (DDR), 0.5477DMIPS/MHz for R5F (DDR or OCM)
    when using TCM for the R5F CPU, the Dhrystone benchmark boosts to 12.8598DMIPS/MHz!!!! Using OCM does not give this improvement.
    Even crazier, using ATCM for the instruction (.text) and the BTCM for the data (.heap and .stack) would increase it to 31.1724DMIPS/MHz!
        There must be something wrong when calculating the number... Yes the timer is not accurate, as large as 4x difference!
    remember to set them in the application component -> Sources -> src -> lscript.ld


