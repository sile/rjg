rjg
===

[![jrg](https://img.shields.io/crates/v/jrg.svg)](https://crates.io/crates/jrg)
[![Documentation](https://docs.rs/jrg/badge.svg)](https://docs.rs/jrg)
[![Actions Status](https://github.com/sile/jrg/workflows/CI/badge.svg)](https://github.com/sile/jrg/actions)
![License](https://img.shields.io/crates/l/jrg)

Random JSON Generator.

```console
// Install.
$ cargo install rjg

// Print help.
$ rjg -h
Random JSON generator

Usage: rjg [OPTIONS] <JSON_TEMPLATE>

Arguments:
  <JSON_TEMPLATE>  JSON template used to generate values

Options:
  -c, --count <COUNT>             Number of JSON values to generate [default: 1]
  -p, --prefix <PREFIX>           Prefix for variable and generator names [default: $]
  -s, --seed <SEED>               Seed for the random number generator
  -v, --var <NAME=JSON_TEMPLATE>  User-defined variables
  -h, --help                      Print help
  -V, --version                   Print version

// Generate integer arrays.
$ rjg --count 3 '[0, {"$int": {"min": 1, "max": 8}}, 9]'
[0,3,9]
[0,8,9]
[0,5,9]

// Generate objects with user-defined variables.
$ rjg --count 3 \
      --var key='{"$str": ["key_", "$alpha", "$alpha", "$digit"]}' \
      --var val='{"$option": "$u16"}' \
      '{"put": {"key": "$key", "value": "$val"}}'
{"put":{"key":"key_im3","value":56386}}
{"put":{"key":"key_qd0","value":null}}
{"put":{"key":"key_ag4","value":49477}}
```


- `__oneof`
- `__integer: {min, max}`
- `__string: {len, char}`
- `__array: {len, value}`
- `__object: [{"name", :"value"} | null, ..]`
- `__optional: {value, ..}` == `{oneof: [x, null]}`
- `__$i`
- `__$u8`, ...


terminology:
- variable: prefixed string
- generator: single entry object having prefixed name
- both only happens in value positions (not object name)
