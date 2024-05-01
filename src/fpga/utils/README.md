# Utilities

Here are several useful scripts to use

## Access the device output and expose interface to other programs

In the rest of the programs we assume the data is streamed to a file at `./ttymicroblossom`.
Here is how to create it.

```sh
tmux new -s serial
touch ttymicroblossom
# this will keep showing the latest tty output while writing to the `./ttymicroblossom` file
sudo picocom /dev/ttyUSB1 -b 115200 --imap lfcrlf | tee ./ttymicroblossom
# in case you need to observe only the recent changes, run
tail -f -n0 ./ttymicroblossom
```

## `build_micro_blossom.py`

```sh
# it will create an `example` folder with the verilog file
python3 build_micro_blossom.py -n example -g example_code_capacity_d3.json
# choose a main function that you want to run and build the binary
cd ../../cpu/embedded
EMBEDDED_BLOSSOM_MAIN=test_micro_blossom make Xilinx
cd -
# build the booting image
cd example
make
# execute and obtain the result in the tmux session `serial`
make run_a72
```
