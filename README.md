# Connect Four Solver

This is a program that solves the game of Connect Four. In other words, it determines the outcome when both players play optimally (the first moving player can always win).

The program is written in Rust. Optimizations:

- Bit hacks
- SIMD
- Caching
- Cache key state reduction (mirroring and ignoring checkers that can't influence the outcome)
- Move ordering

A blog post with more information is available [here](https://jorrid.com/posts/the-wondrous-world-of-connect-four-bit-boards/).

# Running

To time how long it takes to solve, run:

```
docker build --target main --tag conn4 . && time docker run --init -it conn4
```

# Benchmarks

Some operations have multiple implementations (mainly SIMD or not SIMD). There are some benchmarks that help to decide what is faster.

```
rustup run nightly cargo bench
```