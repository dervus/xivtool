# xivtool

Rust library and CLI tool for browsing, loading, and exporting game data of
Final Fantasy XIV.


## Usage

### cli
```
xivtool help
```

### lib
```rust
use std::sync::Arc;
use xiv::{sqpack::SqPack, ex, structs};

// open sqpack repo
let repo: Arc<SqPack> = SqPack::open("/path/to/ffxiv/sqpack").unwrap();

// load exd file using serde struct
let races: Vec<structs::Race>
    = ex::read_exd(repo.clone(), "Race", ex::Locale::English)
        .unwrap()
        .map(Result::unwrap)
        .collect();

println!("{:#?}", races);

// load exd file using generic dynamically typed record type
let items: Vec<ex::Row> =
    ex::read_exd(repo.clone(), "Item", ex::Locale::English)
        .unwrap()
        .map(Result::unwrap)
        .collect();
        
println!("{:?}", items.first());
```


## Implemented features

* [x] SqPack repository
  * [x] Lazy index loading
  * [x] Find file by name
  * [x] Read file
* [x] Client database (.exd files)
  * [x] Dynamically typed Row type
  * [x] Map to custom structs using Serde
  * [x] Export to CSV
* [x] Textures (.tex files)
  * [x] Export to PNG, JPG, TGA using [image-rs](https://crates.io/crates/image)
  * [ ] Export to KTX2
* [ ] Models (.mdl files)
  * [ ] Export to glTF
* [ ] Animations


## License

[Public domain](UNLICENSE)
