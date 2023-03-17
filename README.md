## rs-tchread

A reader for [tokyo cabinet](http://fallabs.com/tokyocabinet/) hash database file, implemented in rust.

## usage

```
$ cargo build --release
$ ./target/release/rs-tchread help
$ ./target/release/rs-tchread list --pv casket.tch
```

## caveat

This library only supports hash databases and does not support modifying a database. It also does not support locks and should not read online databases.

[tokyocabinet installed with apt on debian or ubuntu is broken](https://debian-bugs-dist.debian.narkive.com/I4IA9otI/bug-667979-libtokyocabinet9-tokyocabinet-got-endianness-in-db-wrong-on-both-big-and-little-endian). To read database files created with these binaries, you'll use the `--bigendian` option.
