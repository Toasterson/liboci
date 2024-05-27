# OCI Image specification library for rust

[![crates.io](https://img.shields.io/crates/v/liboci)](https://crates.io/crates/liboci)
[![docs.rs](https://docs.rs/liboci/badge.svg)](https://docs.rs/liboci)
[![CI](https://github.com/toasterson/liboci/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/toasterson/liboci)


This library allows the reading and writing of oci specification json files. 
this currently includes the image but can be extended to runtime with the same
process. Distribution specification is mostly doing the right HTTP requests in the 
right order, thus this spec is not covered here.

## Features:
- Safe typing for json files.
- Conformant (as close as possible) serialization from and to the json files (please report bugs where this is not the case)
- Builder pattern for main json files
- deriving JsonSchema, which should help libraries that use typify
- Included documentation from jsonschema

# Planned Extensions
- [ ] compatibility spec (once ready)

# Possible Extensions
- [ ] more docs
- [ ] implement runtime spec

# License
All code in this repository is licensed under:

- Mozilla Public License Version 2.0 (https://opensource.org/license/mpl-2-0)

# Your contributions
Unless you explicitly state otherwise, any contribution intentionally submitted 
for inclusion in the work by you, shall be MPL 2.0 licensed as above, without any 
additional terms or conditions.
