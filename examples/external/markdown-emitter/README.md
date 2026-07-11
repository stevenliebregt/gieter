# External source + emitter example

A self-contained demo of gieter's external plugin mechanism, with no database. `fake_source.py` produces a hardcoded 
catalog (an `authors` and a `books` table with a foreign key), which feeds `markdown_emitter.py` to write a Markdown 
file.

Run it from this directory so the relative script paths resolve:

```sh
cd examples/external/markdown-emitter
cargo run --manifest-path ../../../Cargo.toml -p gieter
```

Or, with gieter installed, just `gieter` from this directory.

It writes `schema.md` next to this README.
