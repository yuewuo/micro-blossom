import argparse
import os
import sys
import time
import subprocess


script_dir = os.path.dirname(os.path.abspath(__file__))
default_ttyfile = os.path.join(script_dir, "ttymicroblossom")


def get_ttyoutput(
    filename=default_ttyfile,
    command="",
    exit_word="[exit]",
    timeout=180,
    silent=True,
    cwd=None,
    shell=False,
):
    tty_output = ""
    command_output = ""
    with open(filename, "r") as file:
        file.seek(0, 2)  # seek to the end
        if command != "":
            stdout = subprocess.PIPE if silent else sys.stdout
            child = subprocess.Popen(command, shell=shell, cwd=cwd, stdout=stdout)
        last = time.time()
        while True:
            time.sleep(0.1)
            content = file.read()
            if not silent:
                print(content, end="")
            if content != "":
                last = time.time()
            tty_output += content.strip("\x00")
            if time.time() - last > timeout:
                print(f"[warning] ttyoutput timeouted after {timeout}s")
                break
            if exit_word in tty_output:
                break
    if command != "":
        child.wait()
        if silent:
            command_output = child.stdout.read().decode("utf-8")
    return tty_output, command_output


def main(args=None):
    parser = argparse.ArgumentParser(
        description="Get TTY Output, monitoring the output given a trigger command, exit word and timeout"
    )
    parser.add_argument(
        "-f",
        "--file",
        default=default_ttyfile,
        help="the path of the tty output file, see README.md for more information",
    )
    parser.add_argument(
        "-c",
        "--command",
        default="",
        help="execute this command while waiting for the file output",
    )
    parser.add_argument(
        "-e",
        "--exit-word",
        default="[exit]",
        help="when seeing this word, the program is returned",
    )
    parser.add_argument(
        "-t",
        "--timeout",
        default="180",
        help="timeout value if exit word is not found, default to 3min. any active data will reactive a new timeout",
    )
    parser.add_argument("-s", "--silent", action="store_true", help="no printing")
    args = parser.parse_args(args=args)

    # print(args)
    filename = args.file
    exit_word = args.exit_word
    timeout = float(args.timeout)
    silent = args.silent
    command = args.command

    return get_ttyoutput(filename, command, exit_word, timeout, silent)


if __name__ == "__main__":
    main()
