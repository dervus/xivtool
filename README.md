# xivtool

Rust library and CLI tool for browsing, loading, and exporting game data of Final Fantasy XIV.


## Usage

### cli
```bash
xivtool help
```

Export all exd files as csv
```bash
xivtool --repo-dir /path/to/ffxiv/sqpack --out-dir /tmp/csv_dir export exd
```

### lib
```rust
use std::sync::Arc;
use xiv::{sqpack::SqPack, ex, structs};

// open sqpack repo
let repo: Arc<SqPack> = SqPack::open("/path/to/ffxiv/sqpack").unwrap();

// load exd file using serde struct
let races: Vec<structs::Race> = ex::read_exd(repo.clone(), "Race", ex::Locale::English).unwrap().map(Result::unwrap).collect();
println!("{:#?}", races);

// load exd file using generic dynamically typed record type
let items: Vec<ex::Row> = ex::read_exd(repo.clone(), "Item", ex::Locale::English).unwrap().map(Result::unwrap).collect();
println!("{:?}", items.first());
```


## Implemented features

* [x] SqPack repository
  * [x] Lazy index loading
  * [x] Find file by name
  * [x] Read file header
  * [x] Read file into byte array
* [x] Client database (.exd files)
  * [x] Dynamically typed Row type using column data from .exh
  * [x] Custom struct mapping using Serde
  * [x] Export to CSV
  * [x] Using root.exl for bulk export
* [ ] Textures
* [ ] Models
* [ ] Animations


## License

[Public domain](UNLICENSE)
