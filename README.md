[![Version](https://img.shields.io/crates/v/defrag-dirs.svg)](https://crates.io/crates/defrag-dirs)

# defrag-dirs

A simple directory index "defragmentation" tool since e4defrag only handles regular files
It works by moving all directory contents into temporary directories and then replacing the old ones.


Caveats: 

* Will interfere with any process that tries to access the directories at the same time.
* Does not preserve xattrs, acls or timestamps of directories. Metadata of other dir entries is preserved
* If an error is encountered in the middle of a move files may be left stranded in a temporary directory

## Build

* install liblzo2 and libz (build-time dependencies) 
* install rust and cargo
* clone repo
* `cargo build --release`
* `target/release/defrag-dirs --help`
