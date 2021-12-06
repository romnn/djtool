from pathlib import Path
from proto_compile import proto_compile

ROOT_DIR = Path(__file__).parent
PROTO_DIR = ROOT_DIR / "proto"
WEB_PROTO_OUT_DIR = ROOT_DIR / "ui" / "src" / "generated"

if __name__ == "__main__":
    print("compiling into {}".format(WEB_PROTO_OUT_DIR))
    proto_compile.compile_grpc_web(
        options=proto_compile.BaseCompilerOptions(
            proto_source_dir=ROOT_DIR,
            clear_output_dirs=False,
            output_dir=WEB_PROTO_OUT_DIR,
        )
    )
