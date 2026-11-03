// The argparse help surface, byte for byte.
// These texts are the Python parser's own output,
// captured from the reference implementation; the
// tape format's consumers script against them, so
// they are part of the ported spec (FT-49).

pub const TOP_USAGE: &str = r##"usage: flighttape [-h]
                  {convert,convert-dir,validate,play,digest,verify,compare,rollup,slice,screen}
                  ..."##;

pub const TOP_HELP: &str = r##"usage: flighttape [-h]
                  {convert,convert-dir,validate,play,digest,verify,compare,rollup,slice,screen}
                  ...

positional arguments:
  {convert,convert-dir,validate,play,digest,verify,compare,rollup,slice,screen}
    convert             convert a session file to a tape
    convert-dir         convert every session file in a directory
    validate            validate a tape file
    play                play a tape back as a timeline
    digest              print a tape's digest
    verify              check a tape against a digest
    compare             compare two tapes
    rollup              roll up every tape in a directory
    slice               cut a slice out of a tape
    screen              screen a tape for absolute paths

options:
  -h, --help            show this help message and exit
"##;

pub fn sub_help(command: &str) -> &'static str {
    match command {
        "convert" => r##"usage: flighttape convert [-h] [-o OUTPUT] source

positional arguments:
  source

options:
  -h, --help            show this help message and exit
  -o OUTPUT, --output OUTPUT
"##,
        "convert-dir" => r##"usage: flighttape convert-dir [-h] -o OUTPUT_DIR source_dir

positional arguments:
  source_dir

options:
  -h, --help            show this help message and exit
  -o OUTPUT_DIR, --output-dir OUTPUT_DIR
"##,
        "validate" => r##"usage: flighttape validate [-h] tape

positional arguments:
  tape

options:
  -h, --help  show this help message and exit
"##,
        "play" => r##"usage: flighttape play [-h] tape

positional arguments:
  tape

options:
  -h, --help  show this help message and exit
"##,
        "digest" => r##"usage: flighttape digest [-h] tape

positional arguments:
  tape

options:
  -h, --help  show this help message and exit
"##,
        "verify" => r##"usage: flighttape verify [-h] tape expected

positional arguments:
  tape
  expected

options:
  -h, --help  show this help message and exit
"##,
        "compare" => r##"usage: flighttape compare [-h] first second

positional arguments:
  first
  second

options:
  -h, --help  show this help message and exit
"##,
        "rollup" => r##"usage: flighttape rollup [-h] source_dir

positional arguments:
  source_dir

options:
  -h, --help  show this help message and exit
"##,
        "slice" => r##"usage: flighttape slice [-h] [--from FROM_SEQ] [--to TO_SEQ] [--kind KINDS]
                        [-o OUTPUT]
                        tape

positional arguments:
  tape

options:
  -h, --help            show this help message and exit
  --from FROM_SEQ
  --to TO_SEQ
  --kind KINDS
  -o OUTPUT, --output OUTPUT
"##,
        "screen" => r##"usage: flighttape screen [-h] [-o OUTPUT] tape

positional arguments:
  tape

options:
  -h, --help            show this help message and exit
  -o OUTPUT, --output OUTPUT
"##,
        _ => "",
    }
}

/// The usage block of a subcommand's help: its
/// first paragraph, the part usage errors print.
pub fn sub_usage(command: &str) -> &'static str {
    let help = sub_help(command);
    match help.find("\n\n") {
        Some(end) => &help[..end],
        None => help,
    }
}
