rjg
===

[![rjg](https://img.shields.io/crates/v/rjg.svg)](https://crates.io/crates/rjg)
[![Actions Status](https://github.com/sile/rjg/workflows/CI/badge.svg)](https://github.com/sile/rjg/actions)
![License](https://img.shields.io/crates/l/rjg)

Random JSON Generator.

```console
// Install.
$ cargo install rjg

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
```

Rules
-----

- Literal JSON values within a JSON template are outputted exactly as they are
- Non-literal JSON values are classified as follows:
  - **Variables**: JSON strings starting with the `$` prefix
  - **Generators**: Single-member objects with a key starting with the `$` prefix
  - NOTE:
    - The prefix can be changed using `--prefix` option.
    - Both variables and generators cannot be used as object names.
- **Variables**:
  - Variables can be [pre-defined](#pre-defined-variables) or user-defined (the latter are defined via `--var` option)
  - The value of a variable is evaluated to a JSON value when generating a JSON value
- **Generators**:
  - [Generators](#generators) produce a random JSON value based on their content

Generators
----------

### `int`

`int` generator produces a JSON integer between `min` and `max`.

```
{"$int": {"min": INTEGER, "max": INTEGER}}
```

#### `int` examples

```console
$ rjg --count 3 '{"$int": {"min": -5, "max": 5}}'
-3
5
4
```

### `str`

`str` generator procudes a JSON string by concating the values with in the given array.
Note that `null` values are filtered out from the result.

```
{"$str": [VALUE, ...]}
```

#### `str` examples

```console
$ rjg --count 3 '{"$str": ["$digit", " + ", "$digit"]}'
"1 + 0"
"7 + 5"
"0 + 8"

$ rjg --count 3 '{"$str": [{"$option": "_"}, "$alpha", "$alpha", "$digit"]}'
"Ae8"
"_UQ6"
"Cd1"

$ rjg --count 3 '{"$str": {"$arr": {"len": 8, "val": "$digit"}}}'
"84534098"
"91367444"
"16584252"
```

### `arr`

`arr` generator produces a JSON array based on the provided length and value.
Unlike other generators, `arr` postpones the evaluation of `val` until each individual array item is generated.

```
{"$arr": {"len": INTEGER, "val": VALUE}}
```

#### `arr` examples

```console
$ rjg --count 3 '{"$arr": {"len": 3, "val": "$digit"}}'
[0,0,2]
[6,7,3]
[1,7,5]

$ rjg --count 3 '{"$arr": {"len": "$digit", "val": "$digit"}}'
[7,4,5,0,4]
[6,2,4,2,6,9,3]
[]
```

### `obj`

`obj` generator produces a JSON object from an array of objects that specify a name and value.
Note that `null` values in the array are ignored when producing the resulting object.

```
{"$obj": [{"name": STRING, "val": VALUE} | null, ...]}
```

#### `obj` examples

```console
$ rjg --count 3 '{"$obj": [{"name":"foo", "val":"$u8"}, {"$option":{"name":"bar", "val":"$i8"}}]}'
{"foo":119}
{"bar":-75,"foo":42}
{"bar":-87,"foo":233}

$ rjg --count 3 --var key='{"$str": ["$alpha", "$alpha"]}' '{"$obj": [{"name":"$key", "val":"$u8"}]}'
{"jz":63}
{"MA":234}
{"Ir":164}
```

### `oneof`

`oneof` generator selects a JSON value from the given array.

```
{"$oneof": [VALUE, ...]}
```

#### `oneof` examples

```console
$ rjg --count 3 '{"$oneof": ["foo", "bar", "baz"]}'
"bar"
"bar"
"foo"
```

### `option`

`option` is syntactic sugar for `oneof`.
That is, `{"$option": VALUE}` is equivalent with `{"$oneof": [VALUE, null]}`.

Pre-defined variables
---------------------

### `i`

`i` represents the current iteration count.

```console
$ rjg --count 3 '"$i"'
0
1
2
```

### `u8`, `u16`, `u32`, `i8`, `i16`, `i32`, `i64`, `digit`

Integer variables.
Each integer variable represents a JSON integer within the pre-defined range.

```console
$ rjg --count 3 '"$u8"'
67
21
75

$ rjg --count 3 '"$i32"'
-713325780
-1502802973
172465743
```

### `bool`

`bool` represents `true` or `false`.

```console
$ rjg --count 3 '"$bool"'
false
true
false
```

### `alpha`

`alpha` represents a JSON string containing an alphabetic character from the ASCII character set.

```console
$ rjg --count 3 '"$alpha"'
"h"
"A"
"g"
```
