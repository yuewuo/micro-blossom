import os
from run import *
import traceback


def main(config: Configuration):
    for d in config.d_vec:
        configuration = config.config_of(d)
        project = configuration.get_project()
        project.build()
        assert not project.timing_sanity_check_failed()

        # test hello world application
        project.build_embedded_binary()
        project.build_vivado_project(force_recompile_binary=True)

        if project.is_hardware_connected():
            tty_output = project.run_application()
            with open(
                os.path.join(project.hardware_proj_dir(), f"hello.log"), "w"
            ) as log:
                log.write(tty_output)
                assert "Hello world!" in tty_output
        else:
            print("skip hello world test because hardware is not connected")


if __name__ == "__main__":
    errors = []
    for configuration in configurations:
        try:
            main(configuration)
        except Exception as e:
            error_str = traceback.format_exc()
            print(error_str)
            errors.append(error_str)
    if len(errors) > 0:
        for error in errors:
            print(error)
        raise Exception("have errors above")
