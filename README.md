rjg
===

[![rjg](https://img.shields.io/crates/v/rjg.svg)](https://crates.io/crates/rjg)
[![Actions Status](https://github.com/sile/rjg/workflows/CI/badge.svg)](https://github.com/sile/rjg/actions)
![License](https://img.shields.io/crates/l/rjg)

Random JSON Generator.

`rjg` reads a JSON template from stdin and generates random JSON values based on embedded generator patterns.

```console
// Install
$ cargo install rjg

// Generate integer arrays
$ echo '[0, "$u4", 9]' | rjg --count 3
[0,8,9]
[0,6,9]
[0,5,9]

// Generate objects
$ echo '{"put": {"key": "$s[3]", "value": {"$oneof": [null, "$u16"]}}}' | rjg --count 3
{"put":{"key":"cic","value":63308}}
{"put":{"key":"b36","value":10142}}
{"put":{"key":"9dj","value":null}}

// Print help.
$ rjg -h
Random JSON Generator

Usage: rjg [OPTIONS]

Options:
  -h, --help            Print help ('--help' for full help, '-h' for summary)
      --version         Print version
  -c, --count <INTEGER> Number of JSON values to generate [default: 1]
  -s, --seed <INTEGER>  Seed for the random number generator
```

Rules
-----

- Input templates can use JSONC format (JSON with comments), but the output is always plain JSON
- Literal JSON values in a template are output exactly as written
- The following patterns are treated as **generators** and will produce random values:
  - Strings beginning with `$` (e.g., `"$u8"`, `"$i32"`, `"$s[5]"`)
  - Objects with a single `$oneof` key
- Generators produce output as follows:
  - `"$u<bits>"`: Generates a random unsigned integer with the specified bit width (e.g., `"$u8"`, `"$u16"`; maximum 64 bits)
  - `"$i<bits>"`: Generates a random signed integer with the specified bit width (e.g., `"$i8"`, `"$i32"`; maximum 64 bits)
  - `"$s[<length>]"`: Generates a random alphanumeric string of the specified length (e.g., `"$s[5]"`)
  - `{"$oneof": [VALUE, ...]}`: Randomly selects and outputs one value from the provided array
