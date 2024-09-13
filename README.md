rjg
===

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

// foo
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
