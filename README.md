rjg
===

Random JSON Generator.


- `__oneof`
- `__integer: {min, max}`
- `__string: {len, char}`
- `__array: {len, value}`
- `__object: [{"name", :"value"} | null, ..]`
- `__optional: {value, ..}`
- `__$i`
- `__$u8`, ...


terminology:
- variable: prefixed string
- generator: single entry object having prefixed name
- both only happens in value positions (not object name)
